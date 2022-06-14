
use ash::vk;

// Stores the things needed for debugging with Vulkan Validation layers
pub struct VulkanDebugInfo {
  pub loader: ash::extensions::ext::DebugUtils,
  pub messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanDebugInfo {
  pub fn init(entry: &ash::Entry, instance: &ash::Instance) -> Result<VulkanDebugInfo, vk::Result> {
      // Set the desired debug info
      let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT::builder()
          .message_severity(
              vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                  //| vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                  | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                  | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
          )
          .message_type(
              vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                  | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                  | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
          )
          .pfn_user_callback(Some(vulkan_debug_utils_callback));

      let loader = ash::extensions::ext::DebugUtils::new(entry, instance); // Create the debug loader
      let messenger = unsafe { loader.create_debug_utils_messenger(&debugcreateinfo, None)? }; // Create the debug messenger

      Ok(VulkanDebugInfo { loader, messenger })
  }
}

impl Drop for VulkanDebugInfo {
  fn drop(&mut self) {
      unsafe {
          self.loader
              .destroy_debug_utils_messenger(self.messenger, None) // Destroy the debug messenger
      };
  }
}

// Used for printing Vulkan debug layer messages
pub unsafe extern "system" fn vulkan_debug_utils_callback(
  message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
  message_type: vk::DebugUtilsMessageTypeFlagsEXT,
  p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
  _p_user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
  let message = std::ffi::CStr::from_ptr((*p_callback_data).p_message);
  let severity = format!("{:?}", message_severity).to_lowercase();
  let ty = format!("{:?}", message_type).to_lowercase();
  println!("[Vulkan Debug][{}][{}] {:?}", severity, ty, message);
  vk::FALSE
}