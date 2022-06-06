// Our first vertex shader
// A vertex is just that: a vertex = a position in 3D space (x, y, z)
#version 450 // Vulkan shaders utilize the GLSL 450 core

void main() {
    // gl_PointSize is a built-in variable in GLSL that sets the size of the point
    gl_PointSize = 2.0;
    // gl_Position is a special variable that is used to store the final position of the vertex
    gl_Position = vec4(0.0, 0.0, 0.0, 1.0); // This is the position of the vertice(s), the fourth value is the weight of the vertex (1.0 is a point, 0.0 is a line, etc.)
}