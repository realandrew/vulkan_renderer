use ash::vk;
use ash::vk::DebugUtilsMessengerCreateInfoEXT;

use super::surface::*;
use super::command_pool::*;
use super::queue::*;
use super::pipeline::*;
use super::swapchain::*;
use super::debug_utils::*;
use super::vertex::*;

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
  pub vertex_buffer: vk::Buffer,
  pub vertex_buffer_memory: vk::DeviceMemory,
}

impl VulkanApp {
  pub fn init(window: winit::window::Window) -> Result<VulkanApp, Box<dyn std::error::Error>> {
      let entry = ash::Entry::linked(); // Statically link the Vulkan library at compile time

      let layer_names = vec!["VK_LAYER_KHRONOS_validation"]; // Enable the validation layer
      let instance = VulkanApp::init_instance(&entry, &layer_names, &window).0.expect("Failed to initialize instance!"); // Create the instance
      let debug = VulkanDebugInfo::init(&entry, &instance)?; // Create the debug info
      let surface = VulkanSurface::init(&window, &entry, &instance)?; // Create the surface

      // Find the most suitable physical device
      let (physical_device, physical_device_properties, physical_device_features) = VulkanApp::pick_physical_device(&instance).expect("No suitable physical device found!");

      // Find the most suitable queue families on the physical device
      let queue_families = QueueFamilies::init(&instance, physical_device, &surface)?;

      // Create the logical device
      let (logical_device, queues) = VulkanApp::init_device_and_queues(&instance, physical_device, &queue_families, &layer_names)?;

      // Create the swapchain
      let mut swapchain = VulkanSwapchain::init(&instance, physical_device, &logical_device, &surface, &queue_families, &queues)?;

      // Create the render pass
      let renderpass = VulkanApp::init_renderpass(&logical_device, physical_device, swapchain.surface_format.format)?;

      // Create the framebuffers
      swapchain.create_framebuffers(&logical_device, renderpass)?;

      // Create the pipeline
      let pipeline = Pipeline::init(&logical_device, &swapchain, &renderpass)?;

      // Create the command pools
      let pools = Pools::init(&logical_device, &queue_families)?;

      let vertices: [Vertex; 3] = [
          Vertex {
              pos: [0.0, -0.5, 0.0, 1.0],
              color: [1.0, 0.0, 0.0, 1.0],
          },
          Vertex {
              pos: [0.5, 0.5, 0.0, 1.0],
              color: [0.0, 1.0, 0.0, 1.0],
          },
          Vertex {
              pos: [-0.5, 0.5, 0.0, 1.0],
              color: [0.0, 0.0, 1.0, 1.0],
          },
      ];

      // Create the vertex buffer
      let (vertex_buffer, vertex_buffer_memory) = VulkanApp::create_vertex_buffer(&instance, &logical_device, physical_device, &vertices);

      // Create the command buffers (one for each framebuffer)
      let commandbuffers = VulkanApp::create_commandbuffers(&logical_device, &pools, swapchain.amount_of_images)?;

      // Fill the command buffers
      VulkanApp::fill_commandbuffers(
          &commandbuffers,
          &logical_device,
          &renderpass,
          &swapchain,
          &pipeline,
          &vertex_buffer,
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
          vertex_buffer,
          vertex_buffer_memory,
      })
  }

  // Pick the best available Vulkan physical device. This means the highest rated one that is suitable.
  pub fn pick_physical_device(instance: &ash::Instance) -> Option<(vk::PhysicalDevice, vk::PhysicalDeviceProperties, vk::PhysicalDeviceFeatures)> {
      let phys_devs = unsafe { instance.enumerate_physical_devices().expect("Could not enumerate physical devices!") }; // Get all physical devices
      let mut phys_dev: vk::PhysicalDevice = vk::PhysicalDevice::null(); // Create a null physical device
      let mut current_score = 0.0; // Create a score variable
      for p in &phys_devs { // For each physical device
          let score = VulkanApp::rate_physical_device(instance, p);
          if score > current_score { // If the score is higher than the current score, set the physical device to this one
              current_score = score;
              phys_dev = *p;
          }
      }
      if phys_dev == vk::PhysicalDevice::null() { // If the physical device is null, return None (this means no suitable devices were found)
          return None;
      } else {
          let props = unsafe { instance.get_physical_device_properties(phys_dev) }; // Get the properties of the physical device
          let feats = unsafe { instance.get_physical_device_features(phys_dev) }; // Get the features of the physical device
          let device_name = String::from(
              unsafe { std::ffi::CStr::from_ptr(props.device_name.as_ptr()) }
                  .to_str()
                  .unwrap(),
          ); // Get the name of the physical device

          println!("Picked physical device {} with score {}!", device_name, current_score);
          return Some((phys_dev, props, feats));
      }
  }

  // Rate device based on its properties (whether its discrete, integrated, etc; how many queues it has, etc)
  // We also check if the device is suitable at all for our needs (Check for hard requirements [things like if it supports geometry shaders, certain extensions, etc])
  pub fn rate_physical_device(instance: &ash::Instance, device: &vk::PhysicalDevice) -> f32 {
      let props = unsafe { instance.get_physical_device_properties(*device) }; // Get the properties of the physical device
      //dbg!(props);
      let features = unsafe { instance.get_physical_device_features(*device) }; // Get the features of the physical device
      //dbg!(features);
      let queuefamilyproperties = unsafe { instance.get_physical_device_queue_family_properties(*device) }; // Get the queue family properties of the physical device
      //dbg!(&queuefamilyproperties);
      
      let mut score = 0.0;

      if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU { // Dedicated local GPU
          score += 1000.0;
      } else if props.device_type == vk::PhysicalDeviceType::VIRTUAL_GPU { // Unknown GPU connected through virtual machine (likely dedicated)
          score += 500.0;
      } else if props.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU { // Integrated local GPU
          score += 250.0;
      }
      // 0 score for CPU and OTHER types

      // Maximum possible size of textures affects graphics quality
      score += props.limits.max_image_dimension2_d as f32;

      // Application can't function without geometry shaders
      if features.geometry_shader < 1 { // Features are either 0 (not supported) or 1 (supported)
          return 0.0;
      }

      let mut found_graphics_queue = false; // We need a graphics queue
      let mut found_transfer_queue = false; // We need a transfer queue
      for (_index, qfam) in queuefamilyproperties.iter().enumerate() {
          if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::GRAPHICS) { // We need a graphics queue with at least one queue
              found_graphics_queue = true;
          }
          if qfam.queue_count > 0 && qfam.queue_flags.contains(vk::QueueFlags::TRANSFER) { // We need a transfer queue with at least one queue
              found_transfer_queue = true;
          }
      }

      if !found_graphics_queue || !found_transfer_queue {
          return 0.0;
      }

      score
  }

  // Initialize Vulkan instance
  pub fn init_instance(entry: &ash::Entry, layer_names: &[&str], window: &winit::window::Window) -> (Result<ash::Instance, vk::Result>, DebugUtilsMessengerCreateInfoEXT) {
      let enginename = std::ffi::CString::new("Ryoko Engine").unwrap(); // Create a CString with the name of the engine
      let appname = std::ffi::CString::new("Rust <3 Vulkan").unwrap();

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
      println!("Using extensions: {:?}", extension_name_pointers);

      // Setup debug messenger for validation layers
      let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT {
          message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
              //| vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
              | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
              | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
          message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
              | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
              | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
          pfn_user_callback: Some(vulkan_debug_utils_callback),
          ..Default::default()
      };

      // Actually create the Vulkan instance
      let create_info = vk::InstanceCreateInfo::builder()
          .push_next(&mut debugcreateinfo)
          .application_info(&app_info)
          .enabled_layer_names(&layer_name_pointers)
          .enabled_extension_names(&extension_name_pointers);

      unsafe { (entry.create_instance(&create_info, None), debugcreateinfo) }
  }

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
          vec![ash::extensions::khr::Swapchain::name().as_ptr()];

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

  pub fn init_renderpass(logical_device: &ash::Device, physical_device: vk::PhysicalDevice, format: vk::Format) -> Result<vk::RenderPass, vk::Result> {
      let attachments = [vk::AttachmentDescription::builder()
          .format(format) // Format must be sample as the swapchain
          .load_op(vk::AttachmentLoadOp::CLEAR) // What to do when the attachment is first loaded (clear it)
          .store_op(vk::AttachmentStoreOp::STORE) // What to do when the renderpass is complete (store it)
          .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
          .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
          .initial_layout(vk::ImageLayout::UNDEFINED) // The initial layout of the attachment (how the data is stored in memory)
          .final_layout(vk::ImageLayout::PRESENT_SRC_KHR) // The final layout of the attachment (ready for presentation)
          .samples(vk::SampleCountFlags::TYPE_1) // Samples per pixel for the attachment (1 means no anti-aliasing)
          .build()
      ];

      let color_attachment_references = [vk::AttachmentReference {
          attachment: 0,
          layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, // Use a layout that is optimal for color attachments
      }]; // Attach this attachment to the color attachment point as attachment 0

      // Grab a subpass (a render pass is a collection of subpasses), FYI this is only for graphics pipelines, not for compute pipelines
      let subpasses = [vk::SubpassDescription::builder()
              .color_attachments(&color_attachment_references)
              .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS).build()];

      // Define subpass dependencies (how the subpasses are connected if we have multiple subpasses)
      let subpass_dependencies = [vk::SubpassDependency::builder()
          .src_subpass(vk::SUBPASS_EXTERNAL)
          .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
          .dst_subpass(0)
          .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
          .dst_access_mask(
              vk::AccessFlags::COLOR_ATTACHMENT_READ | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
          )
          .build()];

      // Set up the render pass
      let renderpass_info = vk::RenderPassCreateInfo::builder()
          .attachments(&attachments)
          .subpasses(&subpasses)
          .dependencies(&subpass_dependencies);

      // Create the render pass
      let renderpass = unsafe { logical_device.create_render_pass(&renderpass_info, None)? };

      Ok(renderpass)
  }

  pub fn cleanup_renderpass(logical_device: &ash::Device, renderpass: vk::RenderPass) {
      unsafe {
          logical_device.destroy_render_pass(renderpass, None);
      }
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
      self.device.destroy_render_pass(self.renderpass, None); // Destroy the render pass
      self.swapchain.cleanup(&self.device); // Destroy the swapchain
    }

    // Create the swapchain
    self.swapchain = VulkanSwapchain::init(&self.instance, self.physical_device, &self.device, &self.surface, &self.queue_families, &self.queues).expect("Failed to recreate swapchain [swapchain recreation].");

    // Create the render pass
    self.renderpass = VulkanApp::init_renderpass(&self.device, self.physical_device, self.swapchain.surface_format.format).expect("Failed to recreate renderpass [swapchain recreation].");

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
      &self.vertex_buffer,
    ).expect("Failed to fill commandbuffers [swapchain recreation].");

    println!("Swapchain recreated!");
  }

  // A method to actually perform our renderpass
  pub fn fill_commandbuffers(commandbuffers: &[vk::CommandBuffer], logical_device: &ash::Device, renderpass: &vk::RenderPass, swapchain: &VulkanSwapchain, pipeline: &Pipeline, vb: &vk::Buffer) -> Result<(), vk::Result> {
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
            // Choose (bind) our graphics pipeline
            logical_device.cmd_bind_pipeline(
                commandbuffer, 
                vk::PipelineBindPoint::GRAPHICS, 
                pipeline.pipeline,
            );
            // Draw the vertices
            logical_device.cmd_bind_vertex_buffers(commandbuffer, 0, &[*vb], &[0]);
            // TODO: Automatically set vertex count based on active buffer
            logical_device.cmd_draw(commandbuffer, 3, 1, 0, 0); // This is literally our draw command
            // End the renderpass
            logical_device.cmd_end_render_pass(commandbuffer);
            // End the command buffer
            logical_device.end_command_buffer(commandbuffer)?;
        }
    }
    Ok(())
  }

  // Create the vertex buffer
  pub fn create_vertex_buffer(
      instance: &ash::Instance,
      device: &ash::Device,
      physical_device: vk::PhysicalDevice,
      vertices: &[Vertex],
  ) -> (vk::Buffer, vk::DeviceMemory) {
      let vertex_buffer_create_info = vk::BufferCreateInfo {
          s_type: vk::StructureType::BUFFER_CREATE_INFO,
          p_next: std::ptr::null(),
          flags: vk::BufferCreateFlags::empty(),
          size: std::mem::size_of_val(&vertices) as u64,
          usage: vk::BufferUsageFlags::VERTEX_BUFFER,
          sharing_mode: vk::SharingMode::EXCLUSIVE,
          queue_family_index_count: 0,
          p_queue_family_indices: std::ptr::null(),
      };

      let vertex_buffer = unsafe {
          device
              .create_buffer(&vertex_buffer_create_info, None)
              .expect("Failed to create Vertex Buffer")
      };

      let mem_requirements = unsafe { device.get_buffer_memory_requirements(vertex_buffer) };
      let mem_properties =
          unsafe { instance.get_physical_device_memory_properties(physical_device) };
      let required_memory_flags: vk::MemoryPropertyFlags =
          vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
      let memory_type = VulkanApp::find_memory_type(
          mem_requirements.memory_type_bits,
          required_memory_flags,
          mem_properties,
      );

      let allocate_info = vk::MemoryAllocateInfo {
          s_type: vk::StructureType::MEMORY_ALLOCATE_INFO,
          p_next: std::ptr::null(),
          allocation_size: mem_requirements.size,
          memory_type_index: memory_type,
      };

      let vertex_buffer_memory = unsafe {
          device
              .allocate_memory(&allocate_info, None)
              .expect("Failed to allocate vertex buffer memory!")
      };

      unsafe {
          // Bind the vertex buffer memory to the vertex buffer
          device
              .bind_buffer_memory(vertex_buffer, vertex_buffer_memory, 0)
              .expect("Failed to bind Buffer");

          // Copy the vertex data to the vertex buffer memory
          let data_ptr = device
              .map_memory(
                  vertex_buffer_memory,
                  0,
                  vertex_buffer_create_info.size,
                  vk::MemoryMapFlags::empty(),
              )
              .expect("Failed to Map Memory") as *mut Vertex;

          data_ptr.copy_from_nonoverlapping(vertices.as_ptr(), vertices.len());

          device.unmap_memory(vertex_buffer_memory);
      }

      (vertex_buffer, vertex_buffer_memory)
  }

  pub fn find_memory_type(
      type_filter: u32,
      required_properties: vk::MemoryPropertyFlags,
      mem_properties: vk::PhysicalDeviceMemoryProperties,
  ) -> u32 {
      for (i, memory_type) in mem_properties.memory_types.iter().enumerate() {
          //if (type_filter & (1 << i)) > 0 && (memory_type.property_flags & required_properties) == required_properties {
          //    return i as u32
          // }

          // same implementation
          if (type_filter & (1 << i)) > 0
              && memory_type.property_flags.contains(required_properties)
          {
              return i as u32;
          }
      }

      panic!("Failed to find suitable memory type!")
  }
}

impl Drop for VulkanApp {
  fn drop(&mut self) {
      unsafe {
          self.device.device_wait_idle().expect("Failed to wait for device idle!"); // Wait for the device to be idle before cleaning up

          self.device.destroy_buffer(self.vertex_buffer, None);
          self.device.free_memory(self.vertex_buffer_memory, None);

          // TODO: Track which buffer came from which pool
          self.device.free_command_buffers(self.pools.graphics_command_pool, &self.commandbuffers);

          self.pools.cleanup(&self.device); // Cleanup the command pool resources
          self.pipeline.cleanup(&self.device); // Clean up the pipeline
          self.device.destroy_render_pass(self.renderpass, None); // Destroy the render pass
          self.swapchain.cleanup(&self.device); // Destroy the swapchain
          self.device.destroy_device(None); // Destroy the logical device
          std::mem::ManuallyDrop::drop(&mut self.surface); // Destroy the surfaces
          std::mem::ManuallyDrop::drop(&mut self.debug); // Destroy the debug info
          self.instance.destroy_instance(None) // Destroy the instance
      };
  }
}