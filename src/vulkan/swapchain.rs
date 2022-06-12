use ash::vk;
use super::surface::*;
use super::queue::*;

// Stores the things needed for a Vulkan Swapchain (that is, a series of images that can be drawn on and then presented to the screen)
// We are currently using a triple buffered queue
// TODO: Allow for setting the number of images in the swapchain
pub struct VulkanSwapchain {
  pub swapchain_loader: ash::extensions::khr::Swapchain,
  pub swapchain: vk::SwapchainKHR,
  pub images: Vec<vk::Image>,
  pub imageviews: Vec<vk::ImageView>,
  pub framebuffers: Vec<vk::Framebuffer>,
  pub surface_format: vk::SurfaceFormatKHR,
  pub extent: vk::Extent2D,
  pub image_available: Vec<vk::Semaphore>,
  pub rendering_finished: Vec<vk::Semaphore>,
  pub may_begin_drawing: Vec<vk::Fence>, // A fence is used to synchronize CPU-GPU operations
  pub amount_of_images: usize,
  pub current_image: usize,
}

impl VulkanSwapchain {
  pub fn init(
      instance: &ash::Instance,
      physical_device: vk::PhysicalDevice,
      logical_device: &ash::Device,
      surface: &VulkanSurface,
      queue_families: &QueueFamilies,
      queues: &Queues,
  ) -> Result<VulkanSwapchain, vk::Result> {
      let surface_capabilities = surface.get_capabilities(physical_device)?; // Get the surface capabilities
      let extent = surface_capabilities.current_extent; // Get the current extent (the size of the surface)
      let surface_present_modes = surface.get_present_modes(physical_device)?; // Get the surface presentation modes
      let surface_format = *surface.get_formats(physical_device)?.first().unwrap(); // Get the surface formats
      let queuefamilies = [queue_families.graphics.unwrap()]; // Use the graphics queue family
      let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
          .surface(surface.surface) // The surface to create the swapchain for
          .min_image_count( // 3 images are needed for triple buffering. Use the largest between 3 and min supported, as well as the smallest between 3 and the max supported
              3.max(surface_capabilities.min_image_count)
                  .min(surface_capabilities.max_image_count),
          )
          .image_format(surface_format.format) // Use the first format supported by the surface
          .image_color_space(surface_format.color_space) // Use the first color space supported by the surface
          .image_extent(extent) // Use the current extent (width & height) of the surface (change later when resizing)
          .image_array_layers(1) // We only have one layer, more than one is for steroscopic 3D and VR, etc
          .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT) // We want to use the image as a color attachment
          .image_sharing_mode(vk::SharingMode::EXCLUSIVE) // We don't want to share the images with other queues (we access images from one queue at a time)
          .queue_family_indices(&queuefamilies) // Using the graphics queue
          .pre_transform(surface_capabilities.current_transform) // Use the current transform (we don't need to rotate or scale yet so use the identity transform)
          .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE) // We don't need to use alpha blending with other windows
          .present_mode(vk::PresentModeKHR::FIFO); // We want to use the FIFO present mode, show images in order as created, waiting for the next vblank
      let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, logical_device);
      let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };
      let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
      let amount_of_images = swapchain_images.len();
      let mut swapchain_imageviews = Vec::with_capacity(swapchain_images.len());
      for image in &swapchain_images { // Create an image view for each image in the swapchain
          let subresource_range = vk::ImageSubresourceRange::builder()
              .aspect_mask(vk::ImageAspectFlags::COLOR)
              .base_mip_level(0)
              .level_count(1)
              .base_array_layer(0)
              .layer_count(1);
          let imageview_create_info = vk::ImageViewCreateInfo::builder()
              .image(*image)
              .view_type(vk::ImageViewType::TYPE_2D) // Type (1D, 2D, 3D, Cube Map, etc.)
              .format(vk::Format::B8G8R8A8_UNORM) // Format (should match the swapchain format)
              .subresource_range(*subresource_range); // Subresource range (we currently care about the color aspect only, not depth, so mip_level and array_layers are 0/ignored)
          let imageview =
              unsafe { logical_device.create_image_view(&imageview_create_info, None) }?;
          swapchain_imageviews.push(imageview);
      }

      let mut image_available = vec![];
      let mut rendering_finished = vec![];
      let mut may_begin_drawing = vec![];
      let semaphoreinfo = vk::SemaphoreCreateInfo::builder();
      let fenceinfo = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
      for _ in 0..amount_of_images {
          let semaphore_available = unsafe { logical_device.create_semaphore(&semaphoreinfo, None)? };
          let semaphore_finished = unsafe { logical_device.create_semaphore(&semaphoreinfo, None)? };
          image_available.push(semaphore_available);
          rendering_finished.push(semaphore_finished);
          let fence = unsafe { logical_device.create_fence(&fenceinfo, None)? };
          may_begin_drawing.push(fence);
      }

      Ok(VulkanSwapchain {
          swapchain_loader,
          swapchain,
          images: swapchain_images,
          imageviews: swapchain_imageviews,
          framebuffers: vec![],
          surface_format,
          extent,
          amount_of_images,
          current_image: 0,
          image_available,
          rendering_finished,
          may_begin_drawing,
      })
  }

  pub fn create_framebuffers(&mut self, logical_device: &ash::Device, renderpass: vk::RenderPass) -> Result<(), vk::Result> {
      for iv in &self.imageviews {
          let iview = [*iv];
          let framebuffer_info  = vk::FramebufferCreateInfo::builder()
              .render_pass(renderpass)
              .attachments(&iview)
              .width(self.extent.width)
              .height(self.extent.height)
              .layers(1);
          let framebuffer = unsafe { logical_device.create_framebuffer(&framebuffer_info, None) }?;
          self.framebuffers.push(framebuffer);
      }
      Ok(())
  }

  pub unsafe fn cleanup(&mut self, logical_device: &ash::Device) {
      for fence in &self.may_begin_drawing {
          logical_device.destroy_fence(*fence, None);
      }
      for semaphore in &self.image_available {
          logical_device.destroy_semaphore(*semaphore, None); // Destroy image available semaphores
      }
      for semaphore in &self.rendering_finished {
          logical_device.destroy_semaphore(*semaphore, None); // Destroy rendering semaphores
      }
      for fb in &self.framebuffers { // Destroy all the framebuffers
          logical_device.destroy_framebuffer(*fb, None);
      }

      for iv in &self.imageviews { // Destroy the image views
          logical_device.destroy_image_view(*iv, None);
      }

      self.swapchain_loader.destroy_swapchain(self.swapchain, None); // Destroy the swapchain
  }
}