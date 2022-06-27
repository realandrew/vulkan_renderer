use ash::{vk};
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

pub struct IndexBuffer {
  buffer: vk::Buffer,
  allocation: Allocation,
  indice_count: u32,
}

impl IndexBuffer {
  pub fn new(device: &ash::Device, allocator: &mut Allocator, size: u64) -> IndexBuffer {
    let index_buffer_create_info = vk::BufferCreateInfo::builder()
      .size(size)
      .usage(vk::BufferUsageFlags::INDEX_BUFFER)
      .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let index_buffer = unsafe {
        device
            .create_buffer(&index_buffer_create_info, None)
            .expect("Failed to create index buffer")
    };

    let mem_requirements = unsafe { device.get_buffer_memory_requirements(index_buffer) };
    let location = MemoryLocation::CpuToGpu;

    let allocation = allocator.allocate(&AllocationCreateDesc {
      requirements: mem_requirements,
      location,
      linear: true, // Buffers are always linear
      name: "Index Buffer",
    }).expect("Failed to allocate memory for index buffer!");

    unsafe {
        // Bind the vertex buffer memory to the vertex buffer
        device
            .bind_buffer_memory(index_buffer,  allocation.memory(), allocation.offset())
            .expect("Failed to bind index buffer");
    }

    IndexBuffer {
      buffer: index_buffer,
      allocation: allocation,
      indice_count: 0,
    }
  }

  pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
    unsafe {
      device.destroy_buffer(self.buffer, None);
    }
    allocator.free(std::mem::take(&mut self.allocation)).expect("Failed to free vertex buffer memory!");
  }

  /// Returns the size for the number of u32 indices (in bytes)
  pub fn get_size_for_num_indices(num_indices: usize) -> u64 {
    (num_indices * std::mem::size_of::<u32>()) as u64
  }

  pub fn update_buffer(&mut self, device: &ash::Device, data: &[u32]) {
    let dst = self.allocation.mapped_ptr().unwrap().cast().as_ptr();
    unsafe {
      std::ptr::copy_nonoverlapping(
          data.as_ptr(),
          dst,
          data.len(),
      );
    }
    self.indice_count = data.len() as u32;
    //println!("Updated index buffer with {} indices", self.indice_count);
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

  pub fn get_indice_count(&self) -> u32 {
    self.indice_count
  }
}