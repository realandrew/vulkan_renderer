# Vulkan Renderer

This is a renderer capable of displaying 2D and 3D graphics in real time. It's powered by the Vulkan graphics API and written in the Rust programming language.

Created by Andrew Mitchell.

My purpose with this repo is to open source the results of what I'm making whilst learning Vulkan and furthering my Rust skills.

Currently not licensed, I plan to license it under the MIT license once it's in a state I'm happy with.

## Build Steps

Using the standard `cargo build --release` is sufficient. However, you will need to copy the resources folder to the directory containing the built executable (likely `/target/release`) or the executable will panic due to not being able to find its runtime resources (textures and such).
