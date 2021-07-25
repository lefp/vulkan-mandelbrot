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

mod cs { // cs probably stands for "compute shader"
    vulkano_shaders::shader!{
        ty: "compute",
        src: "
#version 450

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0) buffer Data {
    uint data[];
} buf;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    buf.data[idx] *= 12;
}"
    }
}

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
            .to_ascii_lowercase().contains("intel")
        )
        .expect("failed to find specified device");
    
    // get a queue family that supports graphics
    let queue_family =
        physical_device.queue_families()
        .find(|&q| q.supports_graphics())
        .expect("failed to find a graphical queue family");
    
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

    // create data buffer
    let data = 0..65536;
    let data_buffer =
        CpuAccessibleBuffer::from_iter(
            device.clone(), BufferUsage::all(), false, data
        )
        .expect("failed to create source buffer");
    
    // load shader and create compute pipeline
    let shader = cs::Shader::load(device.clone())
        .expect("failed to create shader module");
    let compute_pipeline = Arc::new(
        ComputePipeline::new(
            device.clone(), &shader.main_entry_point(), &(), None
        )
        .expect("failed to create compute pipeline")
    );

    // DEBUG
    println!("{}", device.physical_device().properties()
        .max_uniform_buffer_range.unwrap()
    );
    // return;

    // create descriptor set
    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(
        PersistentDescriptorSet::start(layout.clone())
            .add_buffer(data_buffer.clone()).unwrap().build().unwrap()
    );

    // create command buffer
    let mut builder =
        AutoCommandBufferBuilder::primary(
            device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit
        )
        .unwrap();
    builder.dispatch(
        [1024, 1, 1], compute_pipeline.clone(), set.clone(), (), None
    ).unwrap();
    let command_buffer =
        builder.build().expect("failed to build command buffer");
    
    // submit command buffer and wait for execution to finish
    let finished =
        command_buffer.execute(queue.clone())
        .expect("failed to execute command buffer");
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    // read and print buffers
    print!("data:");
    for x in data_buffer.read().expect("failed to read data buffer").iter() {
        print!(" {}", x);
    };
    println!();
}