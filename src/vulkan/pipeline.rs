use ash::vk;
use ash::vk::DescriptorSetLayout;
use super::swapchain::*;
use super::textured_vertex::TexturedVertex;
use super::vertex::*;

// The pipeline defines the shaders, input and output data, and the pipeline layout
// which defines the binding of the shaders to the pipeline.
// Pipelines are fixed after creation, but you can have multiple pipelines
#[derive(Clone)]
pub struct Pipeline {
  pub pipeline: vk::Pipeline,
  pub layout: vk::PipelineLayout,
  pub descriptor_set_layouts: Vec<DescriptorSetLayout>,
}

impl Pipeline {
  pub fn cleanup(&self, logical_device: &ash::Device) {
    unsafe {
      for dsl in &self.descriptor_set_layouts {
        logical_device.destroy_descriptor_set_layout(*dsl, None);
      }
      logical_device.destroy_pipeline(self.pipeline, None); // Destroy the pipeline
      logical_device.destroy_pipeline_layout(self.layout, None); // Destroy the pipeline layout
    }
  }

  pub fn init(logical_device: &ash::Device, swapchain: &VulkanSwapchain, renderpass: &vk::RenderPass) -> Result<Pipeline, vk::Result> {
    let mainfunctionname = std::ffi::CString::new("main").unwrap();

    // Define the items being included in the pipeline
    let vertexshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
      vk_shader_macros::include_glsl!("shaders/shader.vert", kind: vert), // Kind is redundant with the file extension, but it's here for clarity
    );
    let vertexshader_module = unsafe { logical_device.create_shader_module(&vertexshader_createinfo, None)? };
    let fragmentshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
      vk_shader_macros::include_glsl!("shaders/shader.frag", kind: frag), // Kind is redundant with the file extension, but it's here for clarity
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
    let vertex_attrib_descs = Vertex::get_attribute_descriptions(); /*[vk::VertexInputAttributeDescription {
        location: 0, // Location of the attribute in the shader
        binding: 0, // Binding of the attribute in the shader (e.g. different for color and position for example)
        offset: 0, // Offset of the attribute in the vertex struct (in bytes)
        format: vk::Format::R32G32B32A32_SFLOAT, // Four 32-bit floats (R G B A)
    }];*/

    // What to pass as input to the vertex shader
    let vertex_binding_descs = Vertex::get_binding_description(); /*[vk::VertexInputBindingDescription {
        binding: 0, // Binding of the attribute in the shader (e.g. different for color and position for example)
        stride: 16, // Stride of the attribute in the vertex struct (in bytes)
        input_rate: vk::VertexInputRate::VERTEX, // Data changes from vertex to vertex, other option is INSTANCE for instanced rendering
    }];*/

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
      .vertex_attribute_descriptions(&vertex_attrib_descs)
      .vertex_binding_descriptions(&vertex_binding_descs);

    // Specify how to interpret the vertex data
    let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
      .topology(vk::PrimitiveTopology::TRIANGLE_LIST); // Switch between POINT_LIST and TRIANGLE_LIST

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
      descriptor_set_layouts: vec![],
    })
  }

  pub fn init_textured(
    logical_device: &ash::Device,
    swapchain: &VulkanSwapchain,
    renderpass: &vk::RenderPass,
  ) -> Result<Pipeline, vk::Result> {
    let mainfunctionname = std::ffi::CString::new("main").unwrap();

    // Define the items being included in the pipeline
    let vertexshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
      vk_shader_macros::include_glsl!("shaders/shader_textured.vert", kind: vert), // Kind is redundant with the file extension, but it's here for clarity
    );
    let vertexshader_module = unsafe { logical_device.create_shader_module(&vertexshader_createinfo, None)? };
    let fragmentshader_createinfo = vk::ShaderModuleCreateInfo::builder().code(
      vk_shader_macros::include_glsl!("shaders/shader_textured.frag", kind: frag), // Kind is redundant with the file extension, but it's here for clarity
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
    let vertex_attrib_descs = TexturedVertex::get_attribute_descriptions();

    // What to pass as input to the vertex shader
    let vertex_binding_descs = TexturedVertex::get_binding_description();

    let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
      .vertex_attribute_descriptions(&vertex_attrib_descs)
      .vertex_binding_descriptions(&vertex_binding_descs);

    // Specify how to interpret the vertex data
    let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
      .topology(vk::PrimitiveTopology::TRIANGLE_LIST); // Switch between POINT_LIST and TRIANGLE_LIST

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

    /*let descriptorset_layout_binding_descs0 = [vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build()];
    let descriptorset_layout_info0 = vk::DescriptorSetLayoutCreateInfo::builder()
        .bindings(&descriptorset_layout_binding_descs0);
    let descriptorsetlayout0 = unsafe {
        logical_device.create_descriptor_set_layout(&descriptorset_layout_info0, None)
    }?;*/

    let descriptorset_layout_binding_descs1 = [vk::DescriptorSetLayoutBinding::builder()
      .binding(0)
      .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
      .descriptor_count(1)
      .stage_flags(vk::ShaderStageFlags::FRAGMENT)
      .build()];
    let descriptorset_layout_info1 = vk::DescriptorSetLayoutCreateInfo::builder()
      .bindings(&descriptorset_layout_binding_descs1);
    let descriptorsetlayout1 = unsafe {
      logical_device.create_descriptor_set_layout(&descriptorset_layout_info1, None)
    }?;
    let desclayouts = vec![descriptorsetlayout1];

    // Create the pipeline layout info (defines data attached to the pipeline but not the vertices)
    let pipelinelayout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&desclayouts);
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
      descriptor_set_layouts: desclayouts,
    })
  }
}