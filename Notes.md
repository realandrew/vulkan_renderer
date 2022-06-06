# Design Notes

* A design decision I'm currently making is to use the vk-shader-macros crate to compile my glsl shaders to SPIR-V automatically.
  * We could instead use glslc to compile them to .spv files and load those directly
