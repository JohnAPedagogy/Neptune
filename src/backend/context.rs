//! Instance, physical/logical device, queue, and the allocators everything
//! else borrows from.

use std::sync::Arc;

use vulkano::VulkanLibrary;
use vulkano::command_buffer::allocator::{
    StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo,
};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags,
};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::swapchain::Surface;
use winit::event_loop::EventLoop;

/// Everything device-level that outlives any one window or frame.
pub(crate) struct VulkanContext {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    pub command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    pub descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
}

/// The device extensions Neptune requires.
fn required_device_extensions() -> DeviceExtensions {
    DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    }
}

/// Creates the Vulkan instance, enabling exactly the surface extensions this
/// platform's windowing system needs.
pub(crate) fn create_instance(event_loop: &EventLoop<()>) -> Arc<Instance> {
    let library = VulkanLibrary::new()
        .expect("no Vulkan library found — install a Vulkan-capable GPU driver");
    let enabled_extensions = Surface::required_extensions(event_loop)
        .expect("failed to query the surface extensions this platform needs");

    Instance::new(
        library,
        InstanceCreateInfo {
            enabled_extensions,
            ..Default::default()
        },
    )
    .expect("failed to create Vulkan instance")
}

impl VulkanContext {
    /// Picks a physical device able to present to `surface`, opens a logical
    /// device on it, and builds the shared allocators.
    pub(crate) fn new(instance: &Arc<Instance>, surface: &Arc<Surface>) -> Self {
        let extensions = required_device_extensions();
        let (physical_device, queue_family_index) =
            select_physical_device(instance, surface, &extensions);

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: extensions,
                ..Default::default()
            },
        )
        .expect("failed to create logical device");

        let queue = queues.next().expect("logical device has no queues");

        VulkanContext {
            memory_allocator: Arc::new(StandardMemoryAllocator::new_default(device.clone())),
            command_buffer_allocator: Arc::new(StandardCommandBufferAllocator::new(
                device.clone(),
                StandardCommandBufferAllocatorCreateInfo::default(),
            )),
            descriptor_set_allocator: Arc::new(StandardDescriptorSetAllocator::new(
                device.clone(),
                Default::default(),
            )),
            device,
            queue,
        }
    }
}

/// Prefers a discrete GPU that supports both the swapchain extension and
/// presentation to our surface.
fn select_physical_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("failed to enumerate physical devices")
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, surface).unwrap_or(false)
                })
                .map(|i| (p.clone(), i as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no Vulkan device can present to this window")
}
