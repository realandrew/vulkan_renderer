use ash::vk;
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

use super::app::VulkanApp;

pub struct Texture {
  pub image: image::RgbaImage,
  pub vk_image: vk::Image,
  pub imageview: vk::ImageView,
  pub allocation: Allocation,
  pub sampler: vk::Sampler,
}

impl Texture {
  pub fn from_file<P: AsRef<std::path::Path>>(path: P, app: &mut VulkanApp) -> Self {
    // Load image being used as the texture
    let image = image::open(path)
      .map(|img| img.to_rgba8())
      .expect("Unable to open image for texture creation!");

    let (width, height) = image.dimensions();

    let img_create_info = vk::ImageCreateInfo::builder()
      .image_type(vk::ImageType::TYPE_2D)
      .extent(vk::Extent3D {
          width,
          height,
          depth: 1,
      })
      .mip_levels(1)
      .array_layers(1)
      .format(vk::Format::R8G8B8A8_SRGB)
      .samples(vk::SampleCountFlags::TYPE_1)
      .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED);

    let vk_image = unsafe { app.device.create_image(&img_create_info, None).expect("Failed to create image for texture!") };

    let vk_image_mem_req = unsafe { app.device.get_image_memory_requirements(vk_image) };

    let alloc_create_info = AllocationCreateDesc {
      location: gpu_allocator::MemoryLocation::GpuOnly,
      linear: false,
      name: "Texture",
      requirements: vk_image_mem_req
    };

    let image_alloc = app.allocator.allocate(&alloc_create_info).expect("Failed to allocate image memory for texture!");

    unsafe { app.device.bind_image_memory(vk_image, image_alloc.memory(), image_alloc.offset()).expect("Failed to bind memory to vk_image during texture creation!") };

    // We want to be able to "view" the image we created
    let view_create_info = vk::ImageViewCreateInfo::builder()
      .image(vk_image)
      .view_type(vk::ImageViewType::TYPE_2D)
      .format(vk::Format::R8G8B8A8_SRGB)
      .subresource_range(vk::ImageSubresourceRange { // We only care about the color layer
          aspect_mask: vk::ImageAspectFlags::COLOR,
          level_count: 1,
          layer_count: 1,
          ..Default::default()
      });
    let imageview = unsafe { app.device.create_image_view(&view_create_info, None) }
      .expect("Failed to create image view for texture!");

    // How should we sample the texture? We want a linear interpolation. NEAREST is another popular option.
    let sampler_info = vk::SamplerCreateInfo::builder()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR);
    let sampler = unsafe { app.device.create_sampler(&sampler_info, None) }.expect("Failed to create sampler for texture!");

    let data = image.clone().into_raw();

    // Create the actual texture buffer
    let texture_buffer_create_info = vk::BufferCreateInfo::builder()
      .size(data.len() as u64)
      .usage(vk::BufferUsageFlags::TRANSFER_SRC)
      .sharing_mode(vk::SharingMode::EXCLUSIVE);
    let texture_buff = unsafe {
      app.device
        .create_buffer(&texture_buffer_create_info, None)
        .expect("Failed to create texture Buffer")
    };
    let texture_buff_mem_requirements = unsafe { app.device.get_buffer_memory_requirements(texture_buff) };
    let location = MemoryLocation::CpuToGpu;
    let texture_buff_allocation = app.allocator.allocate(&AllocationCreateDesc {
      requirements: texture_buff_mem_requirements,
      location,
      linear: true, // Buffers are always linear
      name: "Texture Buffer",
    }).expect("Failed to allocate memory for texture buffer!");
    unsafe {
        // Bind the vertex buffer memory to the vertex buffer
        app.device
            .bind_buffer_memory(texture_buff,  texture_buff_allocation.memory(), texture_buff_allocation.offset())
            .expect("Failed to bind texture buffer");
    }

    // Copy image to the texture buffer
    let dst = texture_buff_allocation.mapped_ptr().unwrap().cast().as_ptr();
    unsafe {
      std::ptr::copy_nonoverlapping(
          data.as_ptr(),
          dst,
          data.len(),
      );
    }

    // Now we need to transfer the data from the texture buffer to the vk_image holding the texture
    // To do this we need to use command buffers
    let commandbuf_allocate_info = vk::CommandBufferAllocateInfo::builder()
      .command_pool(app.pools.graphics_command_pool)
      .command_buffer_count(1);
    let copycmdbuffer = unsafe {
      app
        .device
        .allocate_command_buffers(&commandbuf_allocate_info)
    }
    .unwrap()[0];

    let cmdbegininfo = vk::CommandBufferBeginInfo::builder()
      .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    unsafe {
      app
        .device
        .begin_command_buffer(copycmdbuffer, &cmdbegininfo)
    }.expect("Failed to begin command buffer during texture creation!");

    // Start commands

    // Change image layout using a barrier
    let barrier = vk::ImageMemoryBarrier::builder()
    .image(vk_image)
    .src_access_mask(vk::AccessFlags::empty())
    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
    .old_layout(vk::ImageLayout::UNDEFINED)
    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
    .subresource_range(vk::ImageSubresourceRange {
      aspect_mask: vk::ImageAspectFlags::COLOR,
      base_mip_level: 0,
      level_count: 1,
      base_array_layer: 0,
      layer_count: 1,
    })
    .build();
    unsafe {
      app.device.cmd_pipeline_barrier(
        copycmdbuffer,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[barrier],
      )
    };

    let image_subresource = vk::ImageSubresourceLayers {
      aspect_mask: vk::ImageAspectFlags::COLOR,
      mip_level: 0,
      base_array_layer: 0,
      layer_count: 1,
    };
    let region = vk::BufferImageCopy {
      buffer_offset: 0,
      buffer_row_length: 0,
      buffer_image_height: 0,
      image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
      image_extent: vk::Extent3D {
          width,
          height,
          depth: 1,
      },
      image_subresource,
      ..Default::default()
    };
    unsafe {
      app.device.cmd_copy_buffer_to_image(
        copycmdbuffer,
        texture_buff,
        vk_image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[region],
      );
    }

    // Once again change image layout now that the data has been copied
    let barrier = vk::ImageMemoryBarrier::builder()
      .image(vk_image)
      .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
      .dst_access_mask(vk::AccessFlags::SHADER_READ)
      .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
      .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
      .subresource_range(vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
      })
      .build();
    unsafe {
      app.device.cmd_pipeline_barrier(
        copycmdbuffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::FRAGMENT_SHADER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[barrier],
      )
    };

    // End commands

    unsafe { app.device.end_command_buffer(copycmdbuffer) }.expect("Failed to end command buffer during texture creation!");
    let submit_infos = [vk::SubmitInfo::builder()
      .command_buffers(&[copycmdbuffer])
      .build()];
    let fence = unsafe {
      app
        .device
        .create_fence(&vk::FenceCreateInfo::default(), None)
    }.expect("Failed to create fence during texture creation!");
    unsafe {
      app
          .device
          .queue_submit(app.queues.graphics_queue, &submit_infos, fence)
    }.expect("Failed to submit to command buffer during texture creation!");
    unsafe { app.device.wait_for_fences(&[fence], true, std::u64::MAX) }.expect("Failed to wait for fences during texture creation!");
    unsafe { app.device.destroy_fence(fence, None) };
    unsafe { app.device.destroy_buffer(texture_buff, None) }; // Free texture buffer as it's no longer needed now that it's contents is in the vk_image
    app.allocator.free(texture_buff_allocation).expect("Failed to free texture buffer allocation during texture creation!"); // Same goes for the texture buffer allocation
    unsafe {
      app
        .device
        .free_command_buffers(app.pools.graphics_command_pool, &[copycmdbuffer]) // Free the command pool
    };

    Texture {
      image,
      vk_image,
      imageview,
      allocation: image_alloc,
      sampler,
    }
  }

  pub fn destroy(&mut self, device: &ash::Device, allocator: &mut Allocator) {
    unsafe {
      device.destroy_image_view(self.imageview, None);
      device.destroy_image(self.vk_image, None);
      allocator.free(std::mem::take(&mut self.allocation)).expect("Failed to free texture vk_image memory allocation on destroy!");
      device.destroy_sampler(self.sampler, None);
    }
  }
}