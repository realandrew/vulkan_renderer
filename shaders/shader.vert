// Our first vertex shader
// A vertex is just that: a vertex = a position in 3D space (x, y, z)
#version 450 // Vulkan shaders utilize the GLSL 450 core

// Inputs
layout(location = 0) in vec4 in_position;
layout(location = 1) in vec4 in_color;

// Outputs
layout (location=0) out vec4 out_color; // Note variables are defined by their location, not their names

out gl_PerVertex
{
    vec4 gl_Position;
    //float gl_PointSize;
};

void main() {
    // gl_PointSize is a built-in variable in GLSL that sets the size of the point
    //gl_PointSize = 10.0;
    // gl_Position is a special variable that is used to store the final position of the vertex
    gl_Position = in_position;

    out_color = in_color; //vec4(0.0, 0.6, 1.0, 1.0); // This is the color of the vertice(s)
}