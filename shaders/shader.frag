// Our first fragment shader
// A fragment is essentially a pixel on the screen.
#version 450 // Vulkan shaders utilize the GLSL 450 core

layout (location=0) out vec4 color; // Color output variable (location=0)

void main() {
  color = vec4(1.0, 0.0, 0.0, 1.0); // Red (RGBA)
}