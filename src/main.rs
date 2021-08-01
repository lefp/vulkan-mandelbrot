use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::Version;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage,
                              PrimaryCommandBuffer};
use vulkano::sync::GpuFuture;
use vulkano::pipeline::ComputePipeline;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::pipeline::ComputePipelineAbstract;
use std::sync::Arc;
use vulkano::format::Format;
use vulkano::image::ImageDimensions;
use vulkano::image::StorageImage;
use vulkano::format::ClearValue;
use image::{ImageBuffer, Rgba};

fn main() {
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
            // TODO for some reason shit goes weird on the Quadro card
            .to_ascii_lowercase().contains("quadro")
        )
        .expect("failed to find specified device");

    // get a queue family that supports what we need (graphics/compute)
    let queue_family =
        physical_device.queue_families()
        .find(|&q| q.supports_graphics())
        .expect("failed to find a graphics queue family");
    
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
        // TODO should array_layers be 4? (since we use RGBA)
        ImageDimensions::Dim2d { width: 1024, height: 1024, array_layers: 1},
        Format::R8G8B8A8Unorm, Some(queue.family())
    ).unwrap();
    
    // create buffer accessible by cpu (image buffers normally are not)
    let buf = CpuAccessibleBuffer::from_iter(
        device.clone(), BufferUsage::all(), false,
        (0 .. 1024*1024*4).map(|_| 0u8)
    ).expect("failed to create CpuAccessibleBuffer");
    
    // create command buffer
    let mut builder = AutoCommandBufferBuilder::primary(
        device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit
    ).unwrap();
    builder.clear_color_image(
        image.clone(), ClearValue::Float([0.0, 0.0, 1.0, 1.0])
    ).unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone()).unwrap();
    let command_buffer =
        builder.build().expect("failed to build command buffer");
    
    // submit command buffer and wait for execution to finish
    let finished =
        command_buffer.execute(queue.clone())
        .expect("failed to execute command buffer");
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    // read and save reasulting image
    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        1024, 1024, &buffer_content[..]
    ).unwrap();
    image.save("image.png").unwrap();

    // read and print buffer
    print!("data:");
    let mut i: u32 = 0;
    for x in buffer_content.iter() {
        if i == 0 {
            print!(" {}", x);
            i = 1023;
        };
        i -= 1;
    };
    println!();
}