
use ash::{vk};
use memoffset::offset_of;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub(crate) struct Vertex {
  pub pos: [f32; 4],
  pub color: [f32; 4],
}
impl Vertex {
  pub fn get_binding_description() -> [vk::VertexInputBindingDescription; 1] {
    [vk::VertexInputBindingDescription {
      binding: 0,
      stride: std::mem::size_of::<Vertex>() as u32,
      input_rate: vk::VertexInputRate::VERTEX,
    }]
  }

  pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
    [
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 0,
        format: vk::Format::R32G32B32A32_SFLOAT,
        offset: offset_of!(Vertex, pos) as u32,
      },
      vk::VertexInputAttributeDescription {
        binding: 0,
        location: 1,
        format: vk::Format::R32G32B32A32_SFLOAT,
        offset: offset_of!(Vertex, color) as u32,
      },
    ]
  }
}