// Our first fragment shader
// A fragment is essentially a pixel on the screen.
#version 450 // Vulkan shaders utilize the GLSL 450 core

// Inputs
layout (location = 0) in vec3 in_color; // Input color variable (location=0)

// Outputs
layout (location = 0) out vec4 color; // Color output variable (location=0)

void main() {
  color = vec4(in_color, 1.0); // RGBA Color
}