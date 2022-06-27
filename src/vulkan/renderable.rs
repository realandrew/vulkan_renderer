use ash::vk;
use gpu_allocator::vulkan::Allocator;

use super::{vertex_buffer::VertexBuffer, index_buffer::IndexBuffer, vertex::Vertex, textured_vertex::TexturedVertex, pipeline::Pipeline};

pub struct Renderable {
  pub vertex_buffers: Vec<VertexBuffer>,
  pub index_buffer: Option<IndexBuffer>,
  pub is_textured: bool,
}

impl Renderable {
  pub fn new(
    device: &ash::Device,
    allocator: &mut Allocator,
    vertex_count: usize,
    index_count: usize,
  ) -> Result<Renderable, vk::Result> {
    let mut vertex_buffers = vec![];
    let mut vert_buff = VertexBuffer::new(device, allocator, VertexBuffer::get_size_for_num_verts(vertex_count));
    vertex_buffers.push(vert_buff);
    if index_count > 0 {
        let mut index_buff = IndexBuffer::new(device, allocator, IndexBuffer::get_size_for_num_indices(index_count));
        Ok(Renderable {
          vertex_buffers,
          index_buffer: Some(index_buff),
          is_textured: false,
        })
    } else {
      Ok(Renderable {
        vertex_buffers,
        index_buffer: None,
        is_textured: false,
      })
    }
  }

  pub fn new_quad(
    device: &ash::Device,
    allocator: &mut Allocator,
  ) -> Result<Renderable, vk::Result> {
    let lb = TexturedVertex {
      pos: [-1.0, 1.0, 0.0, 1.0],
      tex_coord: [0.0, 1.0],
    }; //lb: left-bottom
    let lt = TexturedVertex {
      pos: [-1.0, -1.0, 0.0, 1.0],
      tex_coord: [0.0, 0.0],
    };
    let rb = TexturedVertex {
      pos: [1.0, 1.0, 0.0, 1.0],
      tex_coord: [1.0, 1.0],
    };
    let rt = TexturedVertex {
      pos: [1.0, -1.0, 0.0, 1.0],
      tex_coord: [1.0, 0.0],
    };
    let vertices = vec![lb, lt, rb, rt];
    let mut vertex_buffers = vec![];
    let mut vert_buff = VertexBuffer::new(device, allocator, VertexBuffer::get_size_for_num_verts(4));
    vert_buff.update_textured_buffer(&device, &vertices);
    vertex_buffers.push(vert_buff);
    let mut index_buff = IndexBuffer::new(device, allocator, IndexBuffer::get_size_for_num_indices(6));
    index_buff.update_buffer(device, &vec![
      1, 0, 2, 2, 3, 1,
    ]);
    Ok(Renderable {
      vertex_buffers,
      index_buffer: Some(index_buff),
      is_textured: true,
    })
  }

  pub fn update_vertices_buffer(&mut self, device: &ash::Device, data: &[Vertex]) {
    self.vertex_buffers[0].update_buffer(device, data);
  }

  pub fn update_indices_buffer(&mut self, device: &ash::Device, data: &[u32]) {
    match self.index_buffer {
      Some(ref mut index_buff) => {
        index_buff.update_buffer(device, data);
      },
      None => {
        println!("Tried to update indices buffer on a renderable created without an index buffer!");
      },
    }
  }

  pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
    for vertex_buffer in &mut self.vertex_buffers {
      vertex_buffer.destroy(device, allocator);
    }
    if let Some(index_buffer) = &mut self.index_buffer {
      index_buffer.destroy(device, allocator);
    }
  }

  pub fn get_vertex_buffers(&self) -> Vec<&VertexBuffer> {
    //&self.vertex_buffers.iter().collect()
    let mut vbs: Vec<&VertexBuffer> = Vec::new();
    for vb in &self.vertex_buffers {
      vbs.push(vb);
    }
    vbs
  }

  pub fn get_index_buffers(&self) -> Vec<&IndexBuffer> {
    //&self.vertex_buffers.iter().collect()
    let mut ibs: Vec<&IndexBuffer> = Vec::new();
    for ib in &self.index_buffer {
      ibs.push(ib);
    }
    ibs
  }
}