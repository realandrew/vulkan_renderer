// Our first fragment shader
// A fragment is essentially a pixel on the screen.
#version 450 // Vulkan shaders utilize the GLSL 450 core

// Inputs
layout (location = 0) in vec2 uv; // Input texture cord variable (location=0)

// Uniforms
layout(set=0,binding=0) uniform sampler2D texturesampler; // Should be set=1 if we use set 1 for other uniform

// Outputs
layout (location = 0) out vec4 color; // Color output variable (location=0)

void main() {
  color = texture(texturesampler,uv); // RGBA Color
}