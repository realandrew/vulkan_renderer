use ash::vk;
use super::queue::*;

// Command Pools are used to allocate command buffers and are associated with a QueueFamily
// We batch commands into a command buffer and then submit them to the queue
pub struct Pools {
  pub graphics_command_pool: vk::CommandPool,
  pub transfer_command_pool: vk::CommandPool,
}

impl Pools {
  // Create the command pools
  pub fn init(logical_device: &ash::Device, queue_families: &QueueFamilies) -> Result<Pools, vk::Result> {
    // Create the graphics command pool
    let graphics_command_pool_info = vk::CommandPoolCreateInfo::builder()
        .queue_family_index(queue_families.graphics.unwrap())
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
    let graphics_command_pool = unsafe {
        logical_device
            .create_command_pool(&graphics_command_pool_info, None)
            .expect("A problem with the command pool creation")
    };

    // Create the transfer command pool
    let transfer_command_pool_info = vk::CommandPoolCreateInfo::builder()
        .queue_family_index(queue_families.transfer.unwrap())
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
    let transfer_command_pool = unsafe {
        logical_device
            .create_command_pool(&transfer_command_pool_info, None)
            .expect("A problem with the command pool creation")
    };

    Ok(Pools {
        graphics_command_pool,
        transfer_command_pool,
    })
  }

  // Cleanup the command pool resources
  pub fn cleanup(&self, logical_device: &ash::Device) {
    unsafe {
      logical_device.destroy_command_pool(self.graphics_command_pool, None); // Destroy the graphics command pool
      logical_device.destroy_command_pool(self.transfer_command_pool, None); // Destroy the transfer command pool
    }
  }
}