pub mod vulkan;

use std::time::Instant;
use rand::Rng;

use ash::vk;

use vulkan::{app::*, vertex::Vertex};
use winit::event::WindowEvent;

const WINDOW_TITLE: &'static str = "Andrew's Rust-based Vulkan Renderer";

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let eventloop = winit::event_loop::EventLoop::new(); // Create a winit event loop
  let window = winit::window::WindowBuilder::new()
    .with_title(WINDOW_TITLE)
    //.with_inner_size(winit::dpi::LogicalSize::new(width, height))
    .build(&eventloop)
    .expect("Failed to create window!"); // Create a winit window

  let mut app = VulkanApp::init(window)?; // Create a vulkan app instance
  let mut now = Instant::now();
  let mut rng = rand::thread_rng();

  let mut r_color = 0.0;
  let mut g_color = 0.0;
  let mut b_color = 0.0;
  let mut target = 1.0;

  // Run the event loop
  eventloop.run(move |event, _, controlflow| match event {
    winit::event::Event::WindowEvent { event, .. } => match event {
      WindowEvent::CloseRequested => {
        *controlflow = winit::event_loop::ControlFlow::Exit;
      }
      WindowEvent::Resized(size) => {
        println!("Window resized to {}px x {}px", size.width, size.height);
      }
      // Ignore other window events
      _ => {}
    }
    /*winit::event::Event::WindowEvent { event: winit::event::WindowEvent::CloseRequested, .. } => {
      *controlflow = winit::event_loop::ControlFlow::Exit;
    }*/
    winit::event::Event::MainEventsCleared => {
      // doing the work here (later)
      app.window.request_redraw();
    }
    winit::event::Event::RedrawRequested(_) => {
      let deltaTime = now.elapsed().as_secs_f32() * 1000.0;
      now = Instant::now();
      // Render here
      let FPS = ((1000.0/deltaTime) * 10.0).round() / 10.0; // Divide by 10^(num digits after decimal). So 10 for 1 digit, 100 for 2 digits, etc.
      println!("FPS: {:.0}", FPS);

      if (r_color >= 1.0) {
        target = -1.0;
      } else if (r_color <= 0.0) {
        target = 1.0;
      }

      r_color = r_color + (target * (deltaTime/1000.0));
      g_color = g_color + (target * (deltaTime/1000.0));
      b_color = b_color + (target * (deltaTime/1000.0));

      let vertices: [Vertex; 3] = [
          Vertex {
              pos: [0.0, -0.5, 0.0, 1.0],
              color: [r_color, 0.0, 0.0, 1.0],//color: [1.0, 0.0, 0.0, 1.0],
          },
          Vertex {
              pos: [0.5, 0.5, 0.0, 1.0],
              color: [0.0, b_color, 0.0, 1.0],
          },
          Vertex {
              pos: [-0.5, 0.5, 0.0, 1.0],
              color: [0.0, 0.0, g_color, 1.0],
          },
      ];

      // Create the vertex buffer
      let prev_vert_buff = app.vertex_buffer;
      let prev_vert_buff_mem = app.vertex_buffer_memory;
      // TODO: Add a way to fill an existing buffer and use that here
      let (vertex_buffer, vertex_buffer_memory) = VulkanApp::create_vertex_buffer(&app.instance, &app.device, app.physical_device, &vertices);

      app.vertex_buffer = vertex_buffer;
      app.vertex_buffer_memory = vertex_buffer_memory;

      VulkanApp::fill_commandbuffers(&app.commandbuffers, &app.device, &app.renderpass, &app.swapchain, &app.pipeline, &app.vertex_buffer).expect("Failed to write commands!");
      
      unsafe {
        app.device.destroy_buffer(prev_vert_buff, None);
        app.device.free_memory(prev_vert_buff_mem, None);
      }

      app.draw_frame();
    }
    // Ignore other events
    _ => {}
  });

  Ok(())
}