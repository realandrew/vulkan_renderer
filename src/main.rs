pub mod vulkan;

use std::time::Instant;

use vulkan::{app::*, vertex::Vertex, vertex_buffer::VertexBuffer, index_buffer::IndexBuffer};
use winit::{event::WindowEvent};

const WINDOW_TITLE: &'static str = "Andrew's Rust-based Vulkan Renderer";

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let eventloop = winit::event_loop::EventLoop::new(); // Create a winit event loop
  let window = winit::window::WindowBuilder::new()
    .with_title(WINDOW_TITLE)
    .with_min_inner_size(winit::dpi::PhysicalSize::new(1, 1)) // Having a size of 0 is valid for some platforms but not for Vulkan extents
    //.with_inner_size(winit::dpi::LogicalSize::new(width, height))
    .build(&eventloop)
    .expect("Failed to create window!"); // Create a winit window

  let mut app = VulkanApp::init(window)?; // Create a vulkan app instance
  let mut now = Instant::now();

  let mut r_color = 0.0;
  let mut g_color = 0.0;
  let mut b_color = 0.0;
  let mut x_pos = 0.0;
  let mut target = 1.0;
  let mut pos_target = 1.0;

  let vb_two = VertexBuffer::new(&app.instance, &app.physical_device, &app.device, VertexBuffer::get_size_for_num_verts(3));
  
  let index_buff_1 = IndexBuffer::new(&app.instance, &app.physical_device, &app.device, IndexBuffer::get_size_for_num_indices(6));

  app.vertex_buffers.push(vb_two);
  app.index_buffers.push(index_buff_1);

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
    winit::event::Event::MainEventsCleared => {
      // doing the work here (later)
      app.window.request_redraw();
    }
    winit::event::Event::RedrawRequested(_) => {
      let delta_time = now.elapsed().as_secs_f32() * 1000.0;
      now = Instant::now();
      let fps = ((1000.0/delta_time) * 10.0).round() / 10.0; // Divide by 10^(num digits after decimal). So 10 for 1 digit, 100 for 2 digits, etc.
      //println!("FPS: {:.0}", fps);
      app.set_window_title(&format!("{} - FPS: {:.0} ({:.3}ms)", WINDOW_TITLE, fps.round(), delta_time));

      // Render here
      if r_color >= 1.0 {
        target = -1.0;
      } else if r_color <= 0.0 {
        target = 1.0;
      }

      if x_pos >= 0.5 {
        pos_target = -1.0;
      } else if x_pos <= -0.5 {
        pos_target = 1.0;
      }

      r_color = r_color + (target * (delta_time/1000.0));
      g_color = g_color + (target * (delta_time/1000.0));
      b_color = b_color + (target * (delta_time/1000.0));

      x_pos = x_pos + ((pos_target / 2.0) * (delta_time/1000.0));

      let vertices: [Vertex; 4] = [
        Vertex {
          pos: [-0.5, -0.5, 0.0, 1.0],
          color: [1.0, 0.0, 0.0, 1.0],
        },
        Vertex {
          pos: [0.5, -0.5, 0.0, 1.0],
          color: [0.0, 1.0, 0.0, 1.0],
        },
        Vertex {
          pos: [0.5, 0.5, 0.0, 1.0],
          color: [0.0, 0.0, 1.0, 1.0],
        },
        Vertex {
          pos: [-0.5, 0.5, 0.0, 1.0],
          color: [1.0, 1.0, 1.0, 1.0],
        },
      ];

      let indices: [u32; 6] = [0, 1, 2, 2, 3, 0]; // Can also use u16

      let vertices_two: [Vertex; 3] = [
          Vertex {
              pos: [x_pos, 0.5, 0.0, 1.0],
              color: [1.0, 1.0, 1.0, 0.4],//color: [1.0, 0.0, 0.0, 1.0],
          },
          Vertex {
            pos: [0.5 + x_pos, -0.5, 0.0, 1.0],
              color: [1.0, 1.0, 1.0, 0.4],
          },
          Vertex {
            pos: [-0.5 + x_pos, -0.5, 0.0, 1.0],
              color: [1.0, 1.0, 1.0, 0.4],
          },
      ];

      // Update the vertex buffer
      let vb_one = app.vertex_buffers.get_mut(0).expect("Failed to fetch vertex buffer!");
      vb_one.update_buffer(&app.device, &vertices);
      let vb_two = app.vertex_buffers.get_mut(1).expect("Failed to fetch vertex buffer!");
      vb_two.update_buffer(&app.device, &vertices_two);
      
      let ib_two = app.index_buffers.get_mut(0).expect("Failed to fetch index buffer!");
      ib_two.update_buffer(&app.device, &indices);

      VulkanApp::fill_commandbuffers(&app.commandbuffers, &app.device, &app.renderpass, &app.swapchain, 
        &app.pipeline, &app.get_vertex_buffers(), app.index_buffers.get(0)).expect("Failed to write commands!");

      app.draw_frame();
    }
    // Ignore other events
    _ => {}
  });

  Ok(())
}