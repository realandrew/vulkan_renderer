
use ash::{vk};
use memoffset::offset_of;

#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct TexturedVertex {
  pub pos: [f32; 4], // Position (X, Y, Z, Normalized)
  pub tex_coord: [f32; 2], // Texture Coordinate (X, Y)
}
impl TexturedVertex {
  pub fn get_binding_description() -> [vk::VertexInputBindingDescription; 1] {
    [vk::VertexInputBindingDescription {
      binding: 0,
      stride: std::mem::size_of::<TexturedVertex>() as u32,
      input_rate: vk::VertexInputRate::VERTEX,
    }]
  }

  pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
    [
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 0,
        format: vk::Format::R32G32B32A32_SFLOAT,
        offset: offset_of!(TexturedVertex, pos) as u32,
      },
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 1,
        format: vk::Format::R32G32B32A32_SFLOAT,
        offset: offset_of!(TexturedVertex, tex_coord) as u32,
      },
    ]
  }
}