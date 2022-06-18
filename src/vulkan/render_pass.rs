use ash::vk;

pub struct RenderPass {}

impl RenderPass {
  pub fn init_renderpass(logical_device: &ash::Device, physical_device: vk::PhysicalDevice, format: vk::Format) -> Result<vk::RenderPass, vk::Result> {
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

  pub fn cleanup_renderpass(logical_device: &ash::Device, renderpass: vk::RenderPass) {
    unsafe {
        logical_device.destroy_render_pass(renderpass, None);
    }
  }
}