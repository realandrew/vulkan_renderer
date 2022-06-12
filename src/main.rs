pub mod vulkan;

use ash::{vk::{self, DebugUtilsMessengerCreateInfoEXT}};

use vulkan::app::*;

const WINDOW_TITLE: &'static str = "Andrew's Rust-based Vulkan Renderer";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let eventloop = winit::event_loop::EventLoop::new(); // Create a winit event loop
    let window = winit::window::WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        //.with_inner_size(winit::dpi::LogicalSize::new(width, height))
        .build(&eventloop)
        .expect("Failed to create window!"); // Create a winit window

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