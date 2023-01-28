# Vulkan Renderer

This is a renderer capable of displaying 2D and 3D graphics in real time. It's powered by the Vulkan graphics API and written in the Rust programming language.

Created by Andrew Mitchell.

My purpose with this repo is to open source the results of what I'm making whilst learning Vulkan and furthering my Rust skills.

Currently not licensed, I plan to license it under the MIT license once it's in a state I'm happy with.

## Build Steps

Using the standard `cargo build --release` is sufficient. However, you will need to copy the resources folder to the directory containing the built executable (likely `/target/release`) or the executable will panic due to not being able to find its runtime resources (textures and such).

## Platform support

Windows has the most support as it's my main development system and the most used OS for desktop gamers. Specifically "modern" Windows which I currently consider as being Windows 10 and 11 (basically from 8.1+ but 8/8.1 has no usage). It should work fine on Windows 7 too but it's not tested on it.

Linux has secondary support. This means it's officially supported and stable releases should work fine on it. However it has less testing than Windows and alpha/beta releases are allowed to break it (but must be fixed prior to releasing another stable).

MacOS has essentially semi-official support. I do my best to support it via MoltenVK, but it may well not work correctly and doesn't get as much testing as other platforms. Stable releases are allowed to break on MacOS, although I will do my best to prevent that. Also it's important to note that MoltenVK works via the Vulkan portability extension. This means it only supports a subset of the Vulkan standard, and is missing some arguably pretty important features (such as geometry shaders).

### Mac OS

Note, I had trouble getting this to run on Mac OS for awhile. I kept getting a linking error related to a lib called lvulkan. If you are using the LunarG SDK this can be fixed by selecting the "System Global Installation" component during install. This installs the system wide ICD, layers, and SDK tools to /usr/local.
