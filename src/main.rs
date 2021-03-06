use vulkano::{
    instance::{Instance, InstanceExtensions, PhysicalDevice},
    Version,
    device::{Device, DeviceExtensions, Features},
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBuffer
    },
    sync::GpuFuture,
    format::{Format},
    image::{
        ImageDimensions, StorageImage,
        view::ImageView,
    },
    pipeline::{ComputePipeline, ComputePipelineAbstract},
    descriptor::descriptor_set::PersistentDescriptorSet,
};
use std::sync::Arc;
use image::{ImageBuffer, Rgba};

mod cs {
    vulkano_shaders::shader!{
        ty: "compute",
        src: "
#version 450

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(set = 0, binding = 0, rgba8) uniform writeonly image2D img;

// For `i` on interval [0, 1], returns an RGB value corresonding to an HSV color
// with:
//     Hue        = i * 360 deg,
//     Saturation = 1 (which is the maximum possible),
//     Value      = 1 - i.
// https://web.archive.org/web/20210803061024/https://en.wikipedia.org/wiki/HSL_and_HSV#HSV_to_RGB_alternative
const float nR = 5.0;
const float nG = 3.0;
const float nB = 1.0;
const vec3 n = vec3(nR, nG, nB);
vec3 get_rgb(const float i) {
    const vec3 k = mod(n + 6.0*i, 6);
    return (1.0 - i) * (1.0 - max(vec3(0.0), min(k, min(4.0 - k, vec3(1.0)))));
}

void main() {
    vec2 norm_coordinates = (gl_GlobalInvocationID.xy + vec2(0.5)) /
                            vec2(imageSize(img));
    
    vec2 c = (norm_coordinates - vec2(0.5)) * 2.0 - vec2(1.0, 0.0);

    vec2 z = vec2(0.0, 0.0);
    float i;
    for (i = 0.0; i < 1.0; i += 0.005) {
        z = vec2(
            z.x * z.x - z.y * z.y + c.x,
            z.y * z.x + z.x * z.y + c.y
        );
        
        if (length(z) > 4.0) {
            break;
        }
    }
    
    vec4 to_write = vec4(get_rgb(i), 1.0);
    imageStore(img, ivec2(gl_GlobalInvocationID.xy), to_write);
}
"
    }
}

fn main() {
    const NPIXELS_DIM0: u32 = 2048;
    const NPIXELS_DIM1: u32 = 2048;

    let instance =
        Instance::new(
            None, Version::V1_2, &InstanceExtensions::none(), None
        )
        .expect("failed to create instance");
    
    // get physical device
    println!("Devices found:");
    for dev in PhysicalDevice::enumerate(&instance) {
        println!("{}",
            dev.properties().device_name.as_ref()
                .expect("encountered unnamed device")
        );
    };
    let physical_device =
        PhysicalDevice::enumerate(&instance)
        .find(|dev|
            dev.properties().device_name.as_ref()
            .expect("encountered unnamed device")
            .to_ascii_lowercase().contains("quadro")
        )
        .expect("failed to find specified device");

    // get a queue family that supports what we need (graphics/compute)
    let queue_family =
        physical_device.queue_families()
        .find(|&q| q.supports_compute())
        .expect("failed to find a compute queue family");
    
    // get a device and queue for the above queue family
    let (device, mut queues) =
        Device::new(
            physical_device,
            &Features::none(),
            &DeviceExtensions::none(),
            [(queue_family, 0.5)].iter().cloned()
        )
        .expect("failed to create device");
    let queue = queues.next().unwrap();

    // create image
    let image = StorageImage::new(
        device.clone(),
        ImageDimensions::Dim2d { 
            width: NPIXELS_DIM0, height: NPIXELS_DIM1, array_layers: 1 
        },
        Format::R8G8B8A8Unorm, Some(queue.family())
    ).unwrap();
    
    // load shader and create compute pipeline
    let shader = cs::Shader::load(device.clone())
        .expect("failed to create shader module");
    let compute_pipeline = Arc::new(
        ComputePipeline::new(
            device.clone(), &shader.main_entry_point(), &(), None
        )
        .expect("failed to create compute pipeline")
    );

    // create descriptor set
    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(PersistentDescriptorSet::start(layout.clone())
        .add_image(
            ImageView::new(image.clone()).unwrap()
        ).unwrap()
        .build().unwrap()
    );

    // create buffer accessible by cpu (image buffers normally are not)
    let buf = CpuAccessibleBuffer::from_iter(
        device.clone(), BufferUsage::all(), false,
        (0 .. NPIXELS_DIM0*NPIXELS_DIM1*4).map(|_| 0u8)
    ).expect("failed to create CpuAccessibleBuffer");
    
    // create command buffer
    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit
    ).unwrap();
    builder
        .dispatch(
            // `8` is the workgroup size, which must also be specified in the
            // shader
            [NPIXELS_DIM0 / 8, NPIXELS_DIM1 / 8, 1],
            compute_pipeline.clone(), set.clone(), (), None
        ).unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone()).unwrap();
    let command_buffer =
        builder.build().expect("failed to build command buffer");
    
    // submit command buffer and wait for execution to finish
    let finished =
        command_buffer.execute(queue.clone())
        .expect("failed to execute command buffer");
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    // read and save resulting image
    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        NPIXELS_DIM0, NPIXELS_DIM1, &buffer_content[..]
    ).unwrap();
    image.save("image.png").unwrap();
}