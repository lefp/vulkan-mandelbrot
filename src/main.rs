use vulkano::instance::{Instance, InstanceExtensions, PhysicalDevice};
use vulkano::Version;
use vulkano::device::{Device, DeviceExtensions, Features};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage,
                              PrimaryCommandBuffer};
use vulkano::sync::GpuFuture;

fn main() {
    let instance =
        Instance::new(
            None, Version::V1_2, &InstanceExtensions::none(), None
        )
        .expect("failed to create instance");
    
    // get physical device (here we try for specifically the nvidia card)
    let physical_device =
        PhysicalDevice::enumerate(&instance)
        .find(|dev|
            dev.properties().device_name.as_ref()
            .expect("encountered unnamed device")
            .to_ascii_lowercase().contains("nvidia")
        )
        .expect("failed to find nvidia device");
    
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

    // create data buffers
    let source_data = 0..64;
    let source_buffer =
        CpuAccessibleBuffer::from_iter(
            device.clone(), BufferUsage::all(), false, source_data
        )
        .expect("failed to create source buffer");
    
    let dest_data = (0..64).map(|_| 0);
    let dest_buffer =
        CpuAccessibleBuffer::from_iter(
            device.clone(), BufferUsage::all(), false, dest_data
        )
        .expect("failed to create destination buffer");
    
    // create command buffer
    let mut builder =
        AutoCommandBufferBuilder::primary(
            device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit
        )
        .unwrap();
    builder.copy_buffer(source_buffer.clone(), dest_buffer.clone()).unwrap();
    let command_buffer =
        builder.build().expect("failed to build command buffer");
    
    // submit command buffer and wait for execution to finish
    let finished =
        command_buffer.execute(queue.clone())
        .expect("failed to execute command buffer");
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    // read and print buffers
    print!("src:");
    for x in source_buffer.read().expect("failed to read src buffer").iter() {
        print!(" {}", x);
    };
    println!();
    print!("dst:");
    for x in dest_buffer.read().expect("failed to read dst buffer").iter() {
        print!(" {}", x);
    };
    println!();
}