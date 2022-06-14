use ash::vk;
use super::surface::*;

// Stores the specified queue families for a physical device.
// Recommened use is to find prefer queue family for each use case and store their index in the struct.
pub struct QueueFamilies {
  pub graphics: Option<u32>,
  pub transfer: Option<u32>,
}

impl QueueFamilies {
  pub fn init(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface: &VulkanSurface) -> Result<QueueFamilies, vk::Result> {
    let mut queue_families = QueueFamilies {
      graphics: None,
      transfer: None,
    };

    let queue_family_properties = unsafe { instance.get_physical_device_queue_family_properties(physical_device) }; // Get the queue family properties
    //dbg!(&queuefamilyproperties);
    let mut found_graphics_q_index = None; // We need a graphics queue
    let mut found_transfer_q_index = None; // We need a transfer queue
    for (index, qfam) in queue_family_properties.iter().enumerate() {
      if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) && // We need a graphics queue with at least one queue
        unsafe { surface.loader.get_physical_device_surface_support(physical_device, index as u32, surface.surface).unwrap() } // Make sure we have surface support (not it's possible that the graphics queue doesn't support this, only the graphics queue)
      {
              found_graphics_q_index = Some(index as u32);
      }
      if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER) { // We need a transfer queue with at least one queue
        // Use first transfer queue found, if there are multiple then prefer the one without graphics support as it's likely to be faster/dedicated hardware
        if found_transfer_q_index.is_none() || !qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS)
        {
          found_transfer_q_index = Some(index as u32);
        }
      }
    }

    queue_families.graphics = found_graphics_q_index;
    queue_families.transfer = found_transfer_q_index;

    Ok(queue_families)
  }
}

// Stores a set of queues (one for each queue family type). Remember you can have more than one queue per family type (so may need multiple instances of this).
pub struct Queues {
  pub graphics_queue: vk::Queue,
  pub transfer_queue: vk::Queue,
}