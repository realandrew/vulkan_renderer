use ash::vk;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::{
  KhrPortabilitySubsetFn,
};

use super::queue::*;

pub struct LogicalDevice {}

impl LogicalDevice {
  pub fn init_device_and_queues(instance: &ash::Instance, physical_device: vk::PhysicalDevice, queue_families: &QueueFamilies, layer_names: &[&str]) -> Result<(ash::Device, Queues), vk::Result> {
    // Turn the layer names into proper format
    let layer_names_c: Vec<std::ffi::CString> = layer_names
        .iter()
        .map(|&ln| std::ffi::CString::new(ln).unwrap())
        .collect();
    let layer_name_pointers: Vec<*const i8> = layer_names_c
        .iter()
        .map(|layer_name| layer_name.as_ptr())
        .collect();

    let priorities = [1.0f32]; // We only have one queue of each type, so we set the priority to 1.0. Priority is a float between 0.0 and 1.0, with 0.0 being the lowest priority.
    let queue_infos = [ // We want a graphics and transfer queue
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.graphics.unwrap())
            .queue_priorities(&priorities)
            .build(),
        vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_families.transfer.unwrap())
            .queue_priorities(&priorities)
            .build(),
    ];

    // Get info about device extensions
    let device_extension_name_pointers: Vec<*const i8> =
        vec![
            ash::extensions::khr::Swapchain::name().as_ptr(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            KhrPortabilitySubsetFn::name().as_ptr(),
        ];

    // Create the logical device
    let device_create_info = vk::DeviceCreateInfo::builder()
        .queue_create_infos(&queue_infos)
        .enabled_extension_names(&device_extension_name_pointers)
        .enabled_layer_names(&layer_name_pointers);
    let logical_device =
        unsafe { instance.create_device(physical_device, &device_create_info, None)? };

    // Get the queues
    let graphics_queue =
        unsafe { logical_device.get_device_queue(queue_families.graphics.unwrap(), 0) };
    let transfer_queue =
        unsafe { logical_device.get_device_queue(queue_families.transfer.unwrap(), 0) };

    Ok((
        logical_device,
        Queues {
            graphics_queue,
            transfer_queue,
        },
    ))
  }
}