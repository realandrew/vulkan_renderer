# Design Notes

* A design decision I'm currently making is to use the vk-shader-macros crate to compile my glsl shaders to SPIR-V automatically.
  * We could instead use glslc (or any other glsl to SPIR-V compiler) to compile them to .spv files and load those directly

* Once I get to actually abstracting the Vulkan backend with a renderer class, I want it namespaced such that there is some global stuff in the renderer itself, and then underneath that namespace it should be separated into 2D and 3D namespaces. For example Renderer::GetDriverVersion() vs Renderer::3D::Model vs Renderer::2D::Sprite.
  * This makes it clear what belongs to what and keeps everything clear and neatly organized
  * This also enables simultaneous work on both renderers without much conflict between them
  * Also good for the initial work being done as I plan to have the 2D renderer complete with basic features well before I start on 3D in any real fashion
  * 2D and 3D can be intermixed in the same scene. In fact, this will often be the case as most UI and text elements will likely make use of the 2D renderer.

* My goal with designing this is that although Vulkan is the primary backend, and for much of the development will likely be the only backend, I want to be able to use other graphics APIs as the backend without completely rewriting this. At some point I plan to expand upon what I accomplish here to make a basic game engine.

## 2D renderer

My plan for the 2D renderer is simple. It can efficiently draw quads. Primary this means starting the drawing with something like BeginScene, calling DrawQuad (or Sprite.Draw which calls this) for each object in the scene (so a bunch of times). Remember a quad can represent any sprite/shape by using a texture that has alpha/transparency. Then EndScene() is called and all these draws are actually batched together into one or just a few actual GPU draw calls. Possibly some (probably frustrum) culling will take place first. This should yield a pretty good, performant, and energy efficient 2D renderer I can start using to make my game. At some point I will probably also have to update to draw particles, but those can probably be drawn either as quads too or just as points.

* Efficient 2D batch Renderer
* Support sprite sheets / texture atlases (i.e. put many smaller textures (like 32x32) into 1 big texture (probably 4096 x 4096 at most))
* Should support instanced rendering
* Draw textured quads
* Draw custom shapes via lists of vertices and indices
* Support some form of anti-aliasing/multi-sampling
* Animation support (likely via putting all the frames into 1 sprite sheet and then walking through those frame by frame every x update cycles)
* UI and font rendering via egui
  * Otherwise we'd have to write our own layout system, font loader and system to pack it into a texture atlas, etc. egui works fine for this purpose, no need to reinvent the wheel

Somewhat later we'll also need:

* A post processing system
* A particles system
* A lighting system

### Perfomance goals

* Initial version should support rendering at least 100K textured sprites at 60 FPS (at least on my hardware - which is an i5 7600K, GTX 1060 6GB, and at 1920 x 1080 resolution)
  * Ideally with at least 1K different textures, but as little as 500 would still satisify this goal since more than a few hundred textures is rare. (Most modern graphics hardware has around 32 texture slots, so for every 32 textures on that hardware we'd have to make another draw call, so 1K textures is at least 32 draw calls)
* Before I use this in a production game I would really like that to be at least 500K textured sprites at 60 FPS
* Ideally, after lots of optimization and the renderer is more stabilized we should be able to render 1 million sprites at 60 FPS

### Future plans/ideas

* Support for other types of animation (for example large player textures may not fit into a spritesheet, so we can also support a custom format that encodes the delta changes of the pixels each frame into a compressed format, basically we take frame 0 as the original frame and then apply whatever delta frame we're at in the animation cycle to the original frame to get the correct frame). This isn't needed for the games I'm planning, or for many 2D games, so I'm not going to complete this for a long while.
