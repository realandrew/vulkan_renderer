use ash::vk;
use ash::vk::DebugUtilsMessengerCreateInfoEXT;
use gpu_allocator::vulkan::*;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::{
  KhrGetPhysicalDeviceProperties2Fn, KhrPortabilityEnumerationFn, KhrPortabilitySubsetFn,
};

use super::surface::*;
use super::command_pool::*;
use super::queue::*;
use super::pipeline::*;
use super::swapchain::*;
use super::debug_utils::*;
use super::vertex_buffer::*;
use super::vertex::*;
use super::index_buffer::*;
use super::physical_device::*;
use super::logical_device::*;
use super::renderable::*;
use super::render_pass::*;

// Stores what we need to use Vulkan to render our graphics (including the window)
pub struct VulkanApp {
  pub window: winit::window::Window,
  pub entry: ash::Entry,
  pub is_framebuffer_resized: bool,
  pub instance: ash::Instance,
  pub debug: std::mem::ManuallyDrop<VulkanDebugInfo>,
  pub surface: std::mem::ManuallyDrop<VulkanSurface>,
  pub physical_device: vk::PhysicalDevice,
  pub physical_device_properties: vk::PhysicalDeviceProperties,
  pub physical_device_features: vk::PhysicalDeviceFeatures,
  pub queue_families: QueueFamilies,
  pub queues: Queues,
  pub device: ash::Device,
  pub swapchain: VulkanSwapchain,
  pub renderpass: vk::RenderPass,
  pub pipeline: Pipeline,
  pub pools: Pools,
  pub commandbuffers: Vec<vk::CommandBuffer>,
  pub allocator: std::mem::ManuallyDrop<Allocator>,
  pub renderables: Vec<Renderable>,
}

impl VulkanApp {
  pub fn init(window: winit::window::Window) -> Result<VulkanApp, Box<dyn std::error::Error>> {
      let entry = ash::Entry::linked(); // Statically link the Vulkan library at compile time

      let layer_names = vec!["VK_LAYER_KHRONOS_validation"]; // Enable the validation layer
      let instance = VulkanApp::init_instance(&entry, &layer_names, &window).0.expect("Failed to initialize instance!"); // Create the instance
      let debug = VulkanDebugInfo::init(&entry, &instance)?; // Create the debug info
      let surface = VulkanSurface::init(&window, &entry, &instance)?; // Create the surface

      // Find the most suitable physical device
      let (physical_device, physical_device_properties, physical_device_features) = PhysicalDevice::pick_physical_device(&instance).expect("No suitable physical device found!");

      // Find the most suitable queue families on the physical device
      let queue_families = QueueFamilies::init(&instance, physical_device, &surface)?;

      // Create the logical device
      let (logical_device, queues) = LogicalDevice::init_device_and_queues(&instance, physical_device, &queue_families, &layer_names)?;

      // Create the swapchain
      let mut swapchain = VulkanSwapchain::init(&instance, physical_device, &logical_device, &surface, &queue_families, &queues)?;

      // Create the render pass
      let renderpass = RenderPass::init_renderpass(&logical_device, physical_device, swapchain.surface_format.format)?;

      // Create the framebuffers
      swapchain.create_framebuffers(&logical_device, renderpass)?;

      // Create the pipeline
      let pipeline = Pipeline::init(&logical_device, &swapchain, &renderpass)?;

      // Create the command pools
      let pools = Pools::init(&logical_device, &queue_families)?;

      let buffer_device_address = false; // Check for and enable buffer device address support at creation time
      let mut allocator = Allocator::new(&AllocatorCreateDesc {
        instance: instance.clone(),
        device: logical_device.clone(),
        physical_device,
        debug_settings: Default::default(),
        buffer_device_address: buffer_device_address,  // Ideally, check the BufferDeviceAddressFeatures struct.
      }).expect("Failed to create allocator!");
      allocator.report_memory_leaks(log::Level::Info);

      // Create the command buffers (one for each framebuffer)
      let commandbuffers = VulkanApp::create_commandbuffers(&logical_device, &pools, swapchain.amount_of_images)?;

      // Fill the command buffers
      VulkanApp::fill_commandbuffers(
          &commandbuffers,
          &logical_device,
          &renderpass,
          &swapchain,
          &pipeline,
          &vec![],
      )?;

      Ok(VulkanApp {
          window,
          entry,
          is_framebuffer_resized: false,
          instance,
          debug: std::mem::ManuallyDrop::new(debug),
          surface: std::mem::ManuallyDrop::new(surface),
          physical_device,
          physical_device_properties,
          physical_device_features,
          queue_families,
          queues,
          device: logical_device,
          swapchain,
          renderpass,
          pipeline,
          pools,
          commandbuffers,
          allocator: std::mem::ManuallyDrop::new(allocator),
          renderables: vec![],
      })
  }

  // Initialize Vulkan instance
  pub fn init_instance(entry: &ash::Entry, layer_names: &[&str], window: &winit::window::Window) -> (Result<ash::Instance, vk::Result>, DebugUtilsMessengerCreateInfoEXT) {
      let enginename = std::ffi::CString::new("Quasar Engine").unwrap(); // Create a CString with the name of the engine
      let appname = std::ffi::CString::new("Andrew's Vulkan Renderer").unwrap();

      // Set the application info
      let app_info = vk::ApplicationInfo::builder()
          .application_name(&appname)
          .application_version(vk::make_api_version(0, 0, 1, 0))
          .engine_name(&enginename)
          .engine_version(vk::make_api_version(0, 0, 1, 0))
          .api_version(vk::make_api_version(0, 1, 0, 106)); // Highest Vulkan version we intentionally support

      // Get info to enable validation layers
      let layer_names_c: Vec<std::ffi::CString> = layer_names
              .iter()
              .map(|&ln| std::ffi::CString::new(ln).unwrap())
              .collect();
      let layer_name_pointers: Vec<*const i8> = layer_names_c
          .iter()
          .map(|layer_name| layer_name.as_ptr())
          .collect();

      // Get info about which extensions to enable
      let mut extension_name_pointers: Vec<*const i8> =
          vec![
              ash::extensions::ext::DebugUtils::name().as_ptr(),
          ];
      let required_surface_extensions = ash_window::enumerate_required_extensions(&window).unwrap().iter().map(|ext| *ext).collect::<Vec<*const i8>>();
      extension_name_pointers.extend(required_surface_extensions.iter());

      #[cfg(any(target_os = "macos", target_os = "ios"))]
      {
        extension_name_pointers.push(KhrPortabilityEnumerationFn::name().as_ptr());
        extension_name_pointers.push(KhrGetPhysicalDeviceProperties2Fn::name().as_ptr()); // Required by VK_HKR_portability_subset
      }

      println!("Extensions in use: ");
      for ext in extension_name_pointers.iter() {
          println!("\t{}", unsafe { std::ffi::CStr::from_ptr(*ext).to_str().unwrap() });
      }

      // Setup debug messenger for validation layers
      // TODO: Switch this to VulkanDebugInfo
      let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT {
          message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
              //| vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
              //| vk::DebugUtilsMessageSeverityFlagsEXT::INFO
              | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
          message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
              | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
              | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
          pfn_user_callback: Some(vulkan_debug_utils_callback),
          ..Default::default()
      };

      let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
        vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
      } else {
        vk::InstanceCreateFlags::default()
      };

      // Actually create the Vulkan instance
      let create_info = vk::InstanceCreateInfo::builder()
          .push_next(&mut debugcreateinfo)
          .application_info(&app_info)
          .enabled_layer_names(&layer_name_pointers)
          .enabled_extension_names(&extension_name_pointers)
          .flags(create_flags);

      unsafe { (entry.create_instance(&create_info, None), debugcreateinfo) }
  }

  // Creates the desired number of command buffers
  pub fn create_commandbuffers(logical_device: &ash::Device, pools: &Pools, amount: usize) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
      let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
          .command_pool(pools.graphics_command_pool)
          .command_buffer_count(amount as u32);
          //.level(vk::CommandBufferLevel::PRIMARY);

      unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info) }
  }

  pub fn draw_frame(&mut self) {
    self.swapchain.current_image = (self.swapchain.current_image + 1) % self.swapchain.amount_of_images as usize; // Acquire the next image in the swapchain

    let (image_index, _is_sub_optimal) = unsafe {
      let result = self.swapchain.swapchain_loader.acquire_next_image(
        self.swapchain.swapchain, // The swapchain to acquire an image from
        std::u64::MAX, // How long to wait for the image (nanoseconds)
        self.swapchain.image_available[self.swapchain.current_image], // The semaphore to signal when the image is ready to be used
        vk::Fence::null(), // A fence to signal when the image is acquired (must have either a semaphore or fence)
      );
      match result {
        Ok(image_index) => image_index,
        Err(vk_result) => match vk_result {
            vk::Result::ERROR_OUT_OF_DATE_KHR => {
                self.recreate_swapchain();
                return;
            }
            _ => panic!("Failed to acquire Swap Chain Image!"),
        },
      }
    };

    unsafe {
      // Wait for our fence to signal that we can render to the image
      self.device.wait_for_fences(
        &[self.swapchain.may_begin_drawing[self.swapchain.current_image]], // The fence to wait for
        true, // If true wait for all fences, if false wait for at least one fence
        std::u64::MAX, // How long to wait for the fences (nanoseconds)
      ).expect("Fence wait failed!");
    }

    // Begin rendering

    // Draw to the image
    let semaphores_available = [self.swapchain.image_available[self.swapchain.current_image]];
    let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
    let semaphores_finished = [self.swapchain.rendering_finished[self.swapchain.current_image]];
    let commandbuffers = [self.commandbuffers[image_index as usize]];
    let submit_info = [vk::SubmitInfo::builder()
      .wait_semaphores(&semaphores_available)
      .wait_dst_stage_mask(&waiting_stages)
      .command_buffers(&commandbuffers)
      .signal_semaphores(&semaphores_finished)
      .build()];

    unsafe {
      // Reset the fence to signal that we can begin drawing to the image
      self.device.reset_fences(
        &[self.swapchain.may_begin_drawing[self.swapchain.current_image]], // The fences to reset
      ).expect("Fence reset failed!");

      self.device.queue_submit(
        self.queues.graphics_queue, 
        &submit_info, 
        self.swapchain.may_begin_drawing[self.swapchain.current_image],
      ).expect("Failed to submit command buffer!");
    }

    // Present the image
    let swapchains = [self.swapchain.swapchain];
    let indices = [image_index];
    let present_info = vk::PresentInfoKHR::builder()
      .wait_semaphores(&semaphores_finished)
      .swapchains(&swapchains)
      .image_indices(&indices);
    
    let result = unsafe { 
      self.swapchain.swapchain_loader.queue_present(self.queues.graphics_queue, &present_info) // TODO: Use a present queue here
    };

    let is_resized = match result {
      Ok(_) => self.is_framebuffer_resized,
      Err(vk_result) => match vk_result {
        vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => true,
        _ => panic!("Failed to present swapchain image!"),
      },
    };

    if is_resized {
      self.is_framebuffer_resized = false;
      self.recreate_swapchain();
    }
  }

  // TODO: There may be a small memory leak here. I saw this because when the window is resized a bunch of times memory usage goes up slightly without dropping.
  pub fn recreate_swapchain(&mut self) {
    // Recreate the swapchain
    unsafe {
      self.device
          .device_wait_idle()
          .expect("Failed to wait device idle (recreate swapchain)!")
    };

    unsafe {
      // TODO: Track which buffer came from which pool
      self.device.free_command_buffers(self.pools.graphics_command_pool, &self.commandbuffers);

      self.pools.cleanup(&self.device); // Cleanup the command pool resources
      self.pipeline.cleanup(&self.device); // Clean up the pipeline
      //self.device.destroy_render_pass(self.renderpass, None); // Destroy the render pass
      RenderPass::cleanup_renderpass(&self.device, self.renderpass);
      self.swapchain.cleanup(&self.device); // Destroy the swapchain
    }

    // Create the swapchain
    self.swapchain = VulkanSwapchain::init(&self.instance, self.physical_device, &self.device, &self.surface, &self.queue_families, &self.queues).expect("Failed to recreate swapchain [swapchain recreation].");

    // Create the render pass
    self.renderpass = RenderPass::init_renderpass(&self.device, self.physical_device, self.swapchain.surface_format.format).expect("Failed to recreate renderpass [swapchain recreation].");

    // Create the framebuffers
    self.swapchain.create_framebuffers(&self.device, self.renderpass).expect("Failed to recreate framebuffers [swapchain recreation].");

    // Create the pipeline
    self.pipeline = Pipeline::init(&self.device, &self.swapchain, &self.renderpass).expect("Failed to recreate pipeline [swapchain recreation].");

    // Create the command pools
    self.pools = Pools::init(&self.device, &self.queue_families).expect("Failed to recreate command pools [swapchain recreation].");

    // Create the command buffers (one for each framebuffer)
    self.commandbuffers = VulkanApp::create_commandbuffers(&self.device, &self.pools, self.swapchain.amount_of_images).expect("Failed to recreate commandbuffers [swapchain recreation].");

    // Fill the command buffers
    VulkanApp::fill_commandbuffers(
      &self.commandbuffers,
      &self.device,
      &self.renderpass,
      &self.swapchain,
      &self.pipeline,
      &self.renderables,
    ).expect("Failed to fill commandbuffers [swapchain recreation].");

    println!("Swapchain recreated!");
  }

  // A method to actually perform our renderpass
  pub fn fill_commandbuffers(
    commandbuffers: &[vk::CommandBuffer], logical_device: &ash::Device, renderpass: &vk::RenderPass, swapchain: &VulkanSwapchain, 
    pipeline: &Pipeline, renderables: &Vec<Renderable>,
  ) -> Result<(), vk::Result> {
    unsafe {
      // Wait for our fence to signal that we can write to the command buffer
      logical_device.wait_for_fences(
        &[swapchain.may_begin_drawing[swapchain.current_image]], // The fence to wait for
        true, // If true wait for all fences, if false wait for at least one fence
        std::u64::MAX, // How long to wait for the fences (nanoseconds)
      ).expect("Fence wait failed!");
    }
    
    for (i, &commandbuffer) in commandbuffers.iter().enumerate() {
      let commandbuffer_begininfo = vk::CommandBufferBeginInfo::builder(); // Start recording a command buffer
      unsafe {
          logical_device.begin_command_buffer(commandbuffer, &commandbuffer_begininfo)?; // Begin the command buffer
      }

      // Clear color
      let clear_values = [vk::ClearValue {
          color: vk::ClearColorValue {
              float32: [0.0, 0.0, 0.08, 1.0],
          },
      }];

      // Setup a renderpass
      let renderpass_begininfo = vk::RenderPassBeginInfo::builder()
        .render_pass(*renderpass)
        .framebuffer(swapchain.framebuffers[i])
        .render_area(vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent,
        })
        .clear_values(&clear_values);

      unsafe {
        // Start the renderpass
        logical_device.cmd_begin_render_pass(
            commandbuffer,
            &renderpass_begininfo,
            vk::SubpassContents::INLINE, // Commands for the first subpass are provided inline, not in a secondary command buffer
        );

        for (_i, renderable) in renderables.iter().enumerate() {
          // Choose (bind) our graphics pipeline
          logical_device.cmd_bind_pipeline(
            commandbuffer, 
            vk::PipelineBindPoint::GRAPHICS, 
            pipeline.pipeline,
          );
          match &renderable.index_buffer {
            Some(index_buffer) => {
              // Bind the index buffer (unlike vertex buffers, can only have 1 index buffer bound at a time)
              logical_device.cmd_bind_index_buffer(
                  commandbuffer,
                  index_buffer.get_buffer(),
                  0,
                  vk::IndexType::UINT32, // Can also be UINT16
              );

              // Draw the vertices
              for vb in &renderable.vertex_buffers {
                logical_device.cmd_bind_vertex_buffers(
                    commandbuffer,
                    0,
                    &[vb.get_buffer()],
                    &[0],
              );
              logical_device.cmd_draw_indexed(
                commandbuffer,
                index_buffer.get_indice_count(), // Num verts to draw
                1, // Not using instanced drawing
                0, // We start at the first index within the index buffer
                0, // We start at the first vertex in the vertex buffer
                0 // Not using instanced drawing so no offset here
              );
            }
            },
            None => {
              // Draw the vertices
              for vb in &renderable.vertex_buffers {
                logical_device.cmd_bind_vertex_buffers(
                  commandbuffer,
                  0,
                  &[vb.get_buffer()],
                  &[0],
                );
                logical_device.cmd_draw(
                  commandbuffer,
                  vb.get_vert_count(),
                  1,
                  0,
                  0,
                );
              }
            }
          }
        }

        // End the renderpass
        logical_device.cmd_end_render_pass(commandbuffer);
        // End the command buffer
        logical_device.end_command_buffer(commandbuffer)?;
      }
    }
    Ok(())
  }

  pub fn set_window_title(&self, title: &str) {
    self.window.set_title(title);
  }
}

impl Drop for VulkanApp {
  fn drop(&mut self) {
      unsafe {
          self.device.device_wait_idle().expect("Failed to wait for device idle!"); // Wait for the device to be idle before cleaning up

          for rb in &mut self.renderables {
            rb.destroy(&self.device, &mut self.allocator);
          }

          // TODO: Track which buffer came from which pool
          self.device.free_command_buffers(self.pools.graphics_command_pool, &self.commandbuffers);

          self.pools.cleanup(&self.device); // Cleanup the command pool resources
          self.pipeline.cleanup(&self.device); // Clean up the pipeline
          self.device.destroy_render_pass(self.renderpass, None); // Destroy the render pass
          self.swapchain.cleanup(&self.device); // Destroy the swapchain
          std::mem::ManuallyDrop::drop(&mut self.allocator); // Explicitly drop before destruction of device and instance.
          self.device.destroy_device(None); // Destroy the logical device
          std::mem::ManuallyDrop::drop(&mut self.surface); // Destroy the surfaces
          std::mem::ManuallyDrop::drop(&mut self.debug); // Destroy the debug info
          self.instance.destroy_instance(None) // Destroy the instance
      };
  }
}