use ash::{vk};
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

use super::{vertex::Vertex, textured_vertex::TexturedVertex};

pub struct VertexBuffer {
  pub buffer: vk::Buffer,
  pub allocation: Allocation,
  vert_count: u32,
  is_textured: bool,
}

impl VertexBuffer {
  pub fn new(device: &ash::Device, allocator: &mut Allocator, size: u64) -> VertexBuffer {
    let vertex_buffer_create_info = vk::BufferCreateInfo::builder()
      .size(size)
      .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
      .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let vert_buff = unsafe {
        device
            .create_buffer(&vertex_buffer_create_info, None)
            .expect("Failed to create Vertex Buffer")
    };

    let mem_requirements = unsafe { device.get_buffer_memory_requirements(vert_buff) };
    let location = MemoryLocation::CpuToGpu;

    let allocation = allocator.allocate(&AllocationCreateDesc {
      requirements: mem_requirements,
      location,
      linear: true, // Buffers are always linear
      name: "Vertex Buffer",
    }).expect("Failed to allocate memory for vertex buffer!");

    unsafe {
        // Bind the vertex buffer memory to the vertex buffer
        device
            .bind_buffer_memory(vert_buff,  allocation.memory(), allocation.offset())
            .expect("Failed to bind vertex buffer");
    }

    VertexBuffer {
      buffer: vert_buff,
      allocation: allocation,
      vert_count: 0,
      is_textured: false,
    }
  }

  pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
    unsafe {
      device.destroy_buffer(self.buffer, None);
    }
    allocator.free(std::mem::take(&mut self.allocation)).expect("Failed to free vertex buffer memory!");
    drop(self);
  }

  /// Returns the size for the number of vertices (in bytes)
  pub fn get_size_for_num_verts(num_verts: usize) -> u64 {
    (num_verts * std::mem::size_of::<Vertex>()) as u64
  }

  pub fn update_buffer(&mut self, device: &ash::Device, data: &[Vertex]) {
    let dst = self.allocation.mapped_ptr().unwrap().cast().as_ptr();
    unsafe {
      std::ptr::copy_nonoverlapping(
          data.as_ptr(),
          dst,
          data.len(),
      );
    }
    self.vert_count = data.len() as u32;
    self.is_textured = true;
    //println!("Updated vertex buffer with {} vertices", self.vert_count);
  }

  pub fn update_textured_buffer(&mut self, device: &ash::Device, data: &[TexturedVertex]) {
    let dst = self.allocation.mapped_ptr().unwrap().cast().as_ptr();
    unsafe {
      std::ptr::copy_nonoverlapping(
          data.as_ptr(),
          dst,
          data.len(),
      );
    }
    self.vert_count = data.len() as u32;
    self.is_textured = false;
    //println!("Updated vertex buffer with {} vertices", self.vert_count);
  }

  pub fn get_buffer(&self) -> vk::Buffer {
    self.buffer
  }

  pub fn get_memory(&self) -> vk::DeviceMemory {
    unsafe { self.allocation.memory() }
  }

  pub fn get_size(&self) -> vk::DeviceSize {
    self.allocation.size()
  }

  pub fn get_offset(&self) -> vk::DeviceSize {
    self.allocation.offset()
  }

  pub fn get_vert_count(&self) -> u32 {
    self.vert_count
  }
}