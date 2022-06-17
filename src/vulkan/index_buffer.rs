use ash::{vk};

use super::app::VulkanApp;

pub struct IndexBuffer {
  buffer: vk::Buffer,
  memory: vk::DeviceMemory,
  size: vk::DeviceSize,
  indice_count: u32,
}

impl IndexBuffer {
  pub fn new(instance: &ash::Instance, physical_device: &vk::PhysicalDevice, device: &ash::Device, size: u64) -> IndexBuffer {
    let index_buffer_create_info = vk::BufferCreateInfo {
      s_type: vk::StructureType::BUFFER_CREATE_INFO, // The type of this struct
      p_next: std::ptr::null(), // Optional
      flags: vk::BufferCreateFlags::empty(), // Optional
      size: size, // std::mem::size_of_val(&vertices) as u64, // Size of the buffer in bytes (must be greater than 0)
      usage: vk::BufferUsageFlags::INDEX_BUFFER, // Buffer will be used as a vertex buffer
      sharing_mode: vk::SharingMode::EXCLUSIVE, // Only one queue will use this buffer at a time
      queue_family_index_count: 0, // Used for sharingMode == SharingMode::CONCURRENT
      p_queue_family_indices: std::ptr::null(), // Used for sharingMode == SharingMode::CONCURRENT
    };

    let index_buffer = unsafe {
        device
            .create_buffer(&index_buffer_create_info, None)
            .expect("Failed to create Index Buffer")
    };

    let mem_requirements = unsafe { device.get_buffer_memory_requirements(index_buffer) };
    let mem_properties =
        unsafe { instance.get_physical_device_memory_properties(*physical_device) };
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

    let index_buffer_mem = unsafe {
        device
            .allocate_memory(&allocate_info, None)
            .expect("Failed to allocate index buffer memory!")
    };

    unsafe {
        // Bind the vertex buffer memory to the vertex buffer
        device
            .bind_buffer_memory(index_buffer, index_buffer_mem, 0)
            .expect("Failed to bind index Buffer");
    }

    IndexBuffer {
      buffer: index_buffer,
      memory: index_buffer_mem,
      size: size,
      indice_count: 0,
    }
  }

  pub fn destroy(&self, device: &ash::Device) {
    unsafe {
      device.destroy_buffer(self.buffer, None);
      device.free_memory(self.memory, None);
    }
  }

  /// Returns the size for the number of u32 indices (in bytes)
  pub fn get_size_for_num_indices(num_indices: usize) -> u64 {
    (num_indices * std::mem::size_of::<u32>()) as u64
  }

  pub fn update_buffer(&mut self, device: &ash::Device, data: &[u32]) {
    unsafe {
      // Copy the vertex data to the vertex buffer memory
      let data_ptr = device
      .map_memory(
          self.memory,
          0,
          self.size,
          vk::MemoryMapFlags::empty(),
      )
      .expect("Failed to map memory (index buffer)") as *mut u32;

      data_ptr.copy_from_nonoverlapping(data.as_ptr(), data.len());

      device.unmap_memory(self.memory);
    }
    self.indice_count = (std::mem::size_of_val(data) / std::mem::size_of::<u32>()) as u32;
  }

  pub fn get_buffer(&self) -> vk::Buffer {
    self.buffer
  }

  pub fn get_memory(&self) -> vk::DeviceMemory {
    self.memory
  }

  pub fn get_size(&self) -> vk::DeviceSize {
    self.size
  }

  pub fn get_indice_count(&self) -> u32 {
    self.indice_count
  }
}