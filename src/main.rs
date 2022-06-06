use ash::{vk::{self, DebugUtilsMessengerCreateInfoEXT}};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eventloop = winit::event_loop::EventLoop::new(); // Create a winit event loop
    let window = winit::window::Window::new(&eventloop)?; // Create a winit window

    let mut app = VulkanApp::init(window)?; // Create a vulkan app instance

    // Run the event loop
    eventloop.run(move |event, _, controlflow| match event {
        winit::event::Event::WindowEvent { event: winit::event::WindowEvent::CloseRequested, .. } => {
            *controlflow = winit::event_loop::ControlFlow::Exit;
        }
        winit::event::Event::MainEventsCleared => {
            // doing the work here (later)
            app.window.request_redraw();
        }
        winit::event::Event::RedrawRequested(_) => {
            //render here (later)
            app.swapchain.current_image = (app.swapchain.current_image + 1) % app.swapchain.amount_of_images as usize; // Acquire the next image in the swapchain

            let (image_index, _) = unsafe {
                app.swapchain.swapchain_loader.acquire_next_image(
                    app.swapchain.swapchain, // The swapchain to acquire an image from
                    std::u64::MAX, // How long to wait for the image (nanoseconds)
                    app.swapchain.image_available[app.swapchain.current_image], // The semaphore to signal when the image is ready to be used
                    vk::Fence::null(), // A fence to signal when the image is acquired (must have either a semaphore or fence)
                ).expect("Image acquisition failed!")
            };

            unsafe {
                // Wait for our fence to signal that we can render to the image
                app.device.wait_for_fences(
                    &[app.swapchain.may_begin_drawing[app.swapchain.current_image]], // The fence to wait for
                    true, // If true wait for all fences, if false wait for at least one fence
                    std::u64::MAX, // How long to wait for the fences (nanoseconds)
                ).expect("Fence wait failed!");

                // Reset the fence to signal that we can begin drawing to the image
                app.device.reset_fences(
                    &[app.swapchain.may_begin_drawing[app.swapchain.current_image]], // The fences to reset
                ).expect("Fence reset failed!");
            }

            // Begin rendering

            // Draw to the image
            let semaphores_available = [app.swapchain.image_available[app.swapchain.current_image]];
            let waiting_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let semaphores_finished = [app.swapchain.rendering_finished[app.swapchain.current_image]];
            let commandbuffers = [app.commandbuffers[image_index as usize]];
            let submit_info = [vk::SubmitInfo::builder()
                .wait_semaphores(&semaphores_available)
                .wait_dst_stage_mask(&waiting_stages)
                .command_buffers(&commandbuffers)
                .signal_semaphores(&semaphores_finished)
                .build()];

            unsafe {
                app.device.queue_submit(
                    app.queues.graphics_queue, 
                    &submit_info, 
                    app.swapchain.may_begin_drawing[app.swapchain.current_image],
                ).expect("Failed to submit command buffer!");
            }

            // Present the image
            let swapchains = [app.swapchain.swapchain];
            let indices = [image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(&semaphores_finished)
                .swapchains(&swapchains)
                .image_indices(&indices);
            unsafe {
                app.swapchain.swapchain_loader.queue_present(
                    app.queues.graphics_queue, 
                    &present_info
                ).expect("Failed to present swapchain image!");
            }

        }
        _ => {}
    });

    Ok(())
}

// Used for printing Vulkan debug layer messages
unsafe extern "system" fn vulkan_debug_utils_callback(
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

// Pick the best available Vulkan physical device. This means the highest rated one that is suitable.
fn pick_physical_device(instance: &ash::Instance) -> Option<(vk::PhysicalDevice, vk::PhysicalDeviceProperties, vk::PhysicalDeviceFeatures)> {
    let phys_devs = unsafe { instance.enumerate_physical_devices().expect("Could not enumerate physical devices!") }; // Get all physical devices
    let mut phys_dev: vk::PhysicalDevice = vk::PhysicalDevice::null(); // Create a null physical device
    let mut current_score = 0.0; // Create a score variable
    for p in &phys_devs { // For each physical device
        let score = rate_physical_device(instance, p);
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
fn rate_physical_device(instance: &ash::Instance, device: &vk::PhysicalDevice) -> f32 {
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
fn init_instance(entry: &ash::Entry, layer_names: &[&str], window: &winit::window::Window) -> (Result<ash::Instance, vk::Result>, DebugUtilsMessengerCreateInfoEXT) {
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

fn init_device_and_queues(instance: &ash::Instance, physical_device: vk::PhysicalDevice, queue_families: &QueueFamilies, layer_names: &[&str]) -> Result<(ash::Device, Queues), vk::Result> {
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

fn init_renderpass(logical_device: &ash::Device, physical_device: vk::PhysicalDevice, format: vk::Format) -> Result<vk::RenderPass, vk::Result> {
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

// Creates the desired number of command buffers
fn create_commandbuffers(logical_device: &ash::Device, pools: &Pools, amount: usize) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
        .command_pool(pools.graphics_command_pool)
        .command_buffer_count(amount as u32);
        //.level(vk::CommandBufferLevel::PRIMARY);

    unsafe { logical_device.allocate_command_buffers(&command_buffer_allocate_info) }
}

fn fill_commandbuffers(commandbuffers: &[vk::CommandBuffer], logical_device: &ash::Device, renderpass: &vk::RenderPass, swapchain: &VulkanSwapchain, pipeline: &Pipeline) -> Result<(), vk::Result> {
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
            logical_device.cmd_draw(commandbuffer, 1, 1, 0, 0);
            // End the renderpass
            logical_device.cmd_end_render_pass(commandbuffer);
            // End the command buffer
            logical_device.end_command_buffer(commandbuffer)?;
        }
    }
    Ok(())
}

// Stores the things needed for debugging with Vulkan Validation layers
struct VulkanDebugInfo {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanDebugInfo {
    fn init(entry: &ash::Entry, instance: &ash::Instance) -> Result<VulkanDebugInfo, vk::Result> {
        // Set the desired debug info
        let mut debugcreateinfo = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
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

// Stored the things needed for a Vulkan surface
struct VulkanSurface {
    surface: vk::SurfaceKHR,
    loader: ash::extensions::khr::Surface,
}

impl VulkanSurface {
    fn init(window: &winit::window::Window, entry: &ash::Entry, instance: &ash::Instance) -> Result<VulkanSurface, vk::Result> {
        // Create a surface for the window (ash-window does this in one line, otherwise we'd have to write winit code for each platform)
        let surface = unsafe { ash_window::create_surface(&entry, &instance, &window, None).unwrap() };
        let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance); // Create the surface loader

        Ok(VulkanSurface {
            surface,
            loader: surface_loader,
        })
    }
    // Get the surface capabilities (needed to create a swapchain)
    fn get_capabilities(&self, physical_device: vk::PhysicalDevice) -> Result<vk::SurfaceCapabilitiesKHR, vk::Result> {
        unsafe { self.loader.get_physical_device_surface_capabilities(physical_device, self.surface) }
    }
    // Get the surface presentation modes
    fn get_present_modes(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::PresentModeKHR>, vk::Result> {
        unsafe { self.loader.get_physical_device_surface_present_modes(physical_device, self.surface) }
    }
    // Get the surface format-color space pairs (needed to create a swapchain)
    fn get_formats(&self, physical_device: vk::PhysicalDevice) -> Result<Vec<vk::SurfaceFormatKHR>, vk::Result> {
        unsafe { self.loader.get_physical_device_surface_formats(physical_device, self.surface) }
    }
    // Check if the queue family supports presentation on this surface
    fn get_physical_device_surface_support(&self, physical_device: vk::PhysicalDevice, queue_family_index: usize) -> Result<bool, vk::Result> {
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

// Stores the specified queue families for a physical device.
// Recommened use is to find prefer queue family for each use case and store their index in the struct.
struct QueueFamilies {
    graphics: Option<u32>,
    transfer: Option<u32>,
}

impl QueueFamilies {
    fn init(instance: &ash::Instance, physical_device: vk::PhysicalDevice, surface: &VulkanSurface) -> Result<QueueFamilies, vk::Result> {
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
struct Queues {
    graphics_queue: vk::Queue,
    transfer_queue: vk::Queue,
}

// Stores the things needed for a Vulkan Swapchain (that is, a series of images that can be drawn on and then presented to the screen)
// We are currently using a triple buffered queue
// TODO: Allow for setting the number of images in the swapchain
struct VulkanSwapchain {
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    imageviews: Vec<vk::ImageView>,
    framebuffers: Vec<vk::Framebuffer>,
    surface_format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    image_available: Vec<vk::Semaphore>,
    rendering_finished: Vec<vk::Semaphore>,
    may_begin_drawing: Vec<vk::Fence>, // A fence is used to synchronize CPU-GPU operations
    amount_of_images: u32,
    current_image: usize,
}

impl VulkanSwapchain {
    fn init(
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
        let amount_of_images = swapchain_images.len() as u32;
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

    fn create_framebuffers(&mut self, logical_device: &ash::Device, renderpass: vk::RenderPass) -> Result<(), vk::Result> {
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

    unsafe fn cleanup(&mut self, logical_device: &ash::Device) {
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

// The pipeline defines the shaders, input and output data, and the pipeline layout
// which defines the binding of the shaders to the pipeline.
// Pipelines are fixed after creation, but you can have multiple pipelines
struct Pipeline {
    pipeline: vk::Pipeline,
    layout: vk::PipelineLayout
}

impl Pipeline {
    fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_pipeline(self.pipeline, None); // Destroy the pipeline
            logical_device.destroy_pipeline_layout(self.layout, None); // Destroy the pipeline layout
        }
    }

    fn init(logical_device: &ash::Device, swapchain: &VulkanSwapchain, renderpass: &vk::RenderPass) -> Result<Pipeline, vk::Result> {
        let mainfunctionname = std::ffi::CString::new("main").unwrap();

        // Define the items being included in the pipeline
        let vertexshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
            vk_shader_macros::include_glsl!("./shaders/shader.vert", kind: vert), // Kind is redundant with the file extension, but it's here for clarity
        );
        let vertexshader_module = unsafe { logical_device.create_shader_module(&vertexshader_createinfo, None)? };
        let fragmentshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
            vk_shader_macros::include_glsl!("./shaders/shader.frag", kind: frag), // Kind is redundant with the file extension, but it's here for clarity
        );
        let fragmentshader_module = unsafe { logical_device.create_shader_module(&fragmentshader_createinfo, None)? };
        let vertexshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vertexshader_module)
            .name(&mainfunctionname);
        let fragmentshader_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(fragmentshader_module)
            .name(&mainfunctionname);

        // Create the shader stages
        let shader_stages = [vertexshader_stage.build(), fragmentshader_stage.build()];

        // What to pass as input to the vertex shader
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder();

        // Specify how to interpret the vertex data
        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::POINT_LIST);

        // Create the viewport info
        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: swapchain.extent.width as f32,
            height: swapchain.extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        // Create the scissor info (disables drawing outside of the viewport)
        let scissors = [vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: swapchain.extent,
        }];

        // Set the viewport
        let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(&viewports)
            .scissors(&scissors);

        // Create the rasterizer info (defines how the pixels are rasterized / how to draw the polygons)
        let rasterizer_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .line_width(1.0) // Set the line width
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE) // Set the front face to be counter-clockwise
            .cull_mode(vk::CullModeFlags::NONE) // We don't want to cull (ignore) anything
            .polygon_mode(vk::PolygonMode::FILL); // We want to fill the polygons, we could also draw wireframe polygons using lines
        
        // Create the multisampling info (defines how to sample the pixels), we don't want to use multisampling (1 sample per pixel)
        let multisampler_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        
        // Create the depth stencil info (defines how to handle the depth buffer). Essentially, we want alpha/trasparency to be handled as normal
        let colourblend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA) // αsrc+(1-α)dst is essentially linearly blending the source and destination by the alpha
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .alpha_blend_op(vk::BlendOp::ADD)
            .color_write_mask(
                vk::ColorComponentFlags::R
                    | vk::ColorComponentFlags::G
                    | vk::ColorComponentFlags::B
                    | vk::ColorComponentFlags::A,
            )
            .build()];
        
        let colourblend_info =
            vk::PipelineColorBlendStateCreateInfo::builder().attachments(&colourblend_attachments);

        // Create the pipeline layout info (defines data attached to the pipeline but not the vertices)
        let pipelinelayout_info = vk::PipelineLayoutCreateInfo::builder();
        let pipelinelayout = unsafe { logical_device.create_pipeline_layout(&pipelinelayout_info, None) }?;
        // Create the pipeline info (defines the data attached to the pipeline and the vertices)
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_info)
            .rasterization_state(&rasterizer_info)
            .multisample_state(&multisampler_info)
            .color_blend_state(&colourblend_info)
            .layout(pipelinelayout)
            .render_pass(*renderpass)
            .subpass(0);
        
        // Create the pipeline
        let graphicspipeline = unsafe {
            logical_device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[pipeline_info.build()],
                    None,
                )
                .expect("A problem with the pipeline creation") // Note that we can create multiple pipelines here, but we only need one right now
                // Note this is expensive to do, we should do it only during start up and loading screens if possible
                // We can even cache old pipelines and reuse them, but we aren't for now
        }[0];
        unsafe {
            // Destroy the shader modules, they are engrained into the pipeline and thus no longer needed
            logical_device.destroy_shader_module(fragmentshader_module, None);
            logical_device.destroy_shader_module(vertexshader_module, None);
        }
        Ok(Pipeline {
            pipeline: graphicspipeline,
            layout: pipelinelayout,
        })
    }
}

// Command Pools are used to allocate command buffers and are associated with a QueueFamily
// We batch commands into a command buffer and then submit them to the queue
struct Pools {
    graphics_command_pool: vk::CommandPool,
    transfer_command_pool: vk::CommandPool,
}

impl Pools {
    // Create the command pools
    fn init(logical_device: &ash::Device, queue_families: &QueueFamilies) -> Result<Pools, vk::Result> {
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
    fn cleanup(&self, logical_device: &ash::Device) {
        unsafe {
            logical_device.destroy_command_pool(self.graphics_command_pool, None); // Destroy the graphics command pool
            logical_device.destroy_command_pool(self.transfer_command_pool, None); // Destroy the transfer command pool
        }
    }
}

// Stores what we need to use Vulkan to render our graphics (including the window)
struct VulkanApp {
    window: winit::window::Window,
    entry: ash::Entry,
    instance: ash::Instance,
    debug: std::mem::ManuallyDrop<VulkanDebugInfo>,
    surface: std::mem::ManuallyDrop<VulkanSurface>,
    physical_device: vk::PhysicalDevice,
    physical_device_properties: vk::PhysicalDeviceProperties,
    physical_device_features: vk::PhysicalDeviceFeatures,
    queue_families: QueueFamilies,
    queues: Queues,
    device: ash::Device,
    swapchain: VulkanSwapchain,
    renderpass: vk::RenderPass,
    pipeline: Pipeline,
    pools: Pools,
    commandbuffers: Vec<vk::CommandBuffer>,
}

impl VulkanApp {
    fn init(window: winit::window::Window) -> Result<VulkanApp, Box<dyn std::error::Error>> {
        let entry = ash::Entry::linked(); // Statically link the Vulkan library at compile time

        let layer_names = vec!["VK_LAYER_KHRONOS_validation"]; // Enable the validation layer
        let instance = init_instance(&entry, &layer_names, &window).0.expect("Failed to initialize instance!"); // Create the instance
        let debug = VulkanDebugInfo::init(&entry, &instance)?; // Create the debug info
        let surface = VulkanSurface::init(&window, &entry, &instance)?; // Create the surface

        // Find the most suitable physical device
        let (physical_device, physical_device_properties, physical_device_features) = pick_physical_device(&instance).expect("No suitable physical device found!");

        // Find the most suitable queue families on the physical device
        let queue_families = QueueFamilies::init(&instance, physical_device, &surface)?;

        // Create the logical device
        let (logical_device, queues) = init_device_and_queues(&instance, physical_device, &queue_families, &layer_names)?;

        // Create the swapchain
        let mut swapchain = VulkanSwapchain::init(&instance, physical_device, &logical_device, &surface, &queue_families, &queues)?;

        // Create the render pass
        let renderpass = init_renderpass(&logical_device, physical_device, swapchain.surface_format.format)?;

        // Create the framebuffers
        swapchain.create_framebuffers(&logical_device, renderpass)?;

        // Create the pipeline
        let pipeline = Pipeline::init(&logical_device, &swapchain, &renderpass)?;

        // Create the command pools
        let pools = Pools::init(&logical_device, &queue_families)?;

        // Create the command buffers (one for each framebuffer)
        let commandbuffers = create_commandbuffers(&logical_device, &pools, swapchain.framebuffers.len())?;

        // Fill the command buffers
        fill_commandbuffers(
            &commandbuffers,
            &logical_device,
            &renderpass,
            &swapchain,
            &pipeline,
        )?;

        Ok(VulkanApp {
            window,
            entry,
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
        })
    }
}

impl Drop for VulkanApp {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().expect("Failed to wait for device idle!"); // Wait for the device to be idle before cleaning up
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
