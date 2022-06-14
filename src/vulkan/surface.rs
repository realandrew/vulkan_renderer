use ash::vk;

// Stored the things needed for a Vulkan surface
pub struct VulkanSurface {
  pub surface: vk::SurfaceKHR,
  pub loader: ash::extensions::khr::Surface,
}

impl VulkanSurface {
  pub fn init(window: &winit::window::Window, entry: &ash::Entry, instance: &ash::Instance) -> Result<VulkanSurface, vk::Result> {
    // Create a surface for the window (ash-window does this in one line, otherwise we'd have to write winit code for each platform)
    let surface = unsafe { ash_window::create_surface(&entry, &instance, &window, None).unwrap() };
    let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance); // Create the surface loader

    Ok(VulkanSurface {
      surface,
      loader: surface_loader,
    })
  }
  // Get the surface capabilities (needed to create a swapchain)
  pub fn get_capabilities(&self, physical_device: vk::PhysicalDevice) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
    unsafe { self.loader.get_physical_device_surface_capabilities(physical_device, self.surface) }
  }
  // Get the surface presentation modes
  pub fn get_present_modes(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::PresentModeKHR>, vk::Result> {
    unsafe { self.loader.get_physical_device_surface_present_modes(physical_device, self.surface) }
  }
  // Get the surface format-color space pairs (needed to create a swapchain)
  pub fn get_formats(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
    unsafe { self.loader.get_physical_device_surface_formats(physical_device, self.surface) }
  }
  // Check if the queue family supports presentation on this surface
  pub fn get_physical_device_surface_support(&self, physical_device: vk::PhysicalDevice, queue_family_index: usize) -> Result<bool, vk::Result> {
    unsafe { self.loader.get_physical_device_surface_support(physical_device, queue_family_index as u32, self.surface) }
  }
}

impl Drop for VulkanSurface {
  fn drop(&mut self) {
    unsafe {
      self.loader.destroy_surface(self.surface, None); // Destroy the surface
    }
  }
}