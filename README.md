# Ash Vulkan Triangle

A minimal Vulkan render loop written in Rust using [`ash`](https://crates.io/crates/ash) and [`winit`](https://crates.io/crates/winit). It creates a window, selects a graphics/present queue, configures a swapchain, records command buffers, and draws a three-color triangle.

## Requirements

- Rust toolchain
- Vulkan loader and a Vulkan-capable driver
- Vulkan validation layers (recommended for debug builds)

Run it with:

```text
cargo run
```

The build script compiles `shaders/triangle.vert` and `shaders/triangle.frag` to SPIR-V using `shaderc`, so no external shader compiler is needed.

## Camera controls

- `W` / `A` / `S` / `D` move the camera forward, left, backward, and right.
- Mouse movement changes the camera yaw and pitch.
- The camera position is printed once per rendered frame.

## Scene graph

The renderer owns a scene graph containing a camera and entities. Each entity has a
transform, a mesh descriptor, and a visibility flag. Every visible entity is drawn
each frame with its transform and color sent to the vertex shader through push
constants. `Scene::demo()` creates two visible triangles to demonstrate the setup.

## Resource manager

`resource_manager::ResourceManager` loads relative paths from a configured directory,
keeps recently used file contents in memory, and evicts the least recently used
resources when the byte budget is exceeded:

```rust
let mut resources = ResourceManager::new("assets", 16 * 1024 * 1024);
let shader = resources.load("shaders/example.spv")?;
```

Absolute paths and paths that traverse outside the resource directory are rejected.

## Seeded simplex noise

`noise::simplex(seed, x, y)` returns a deterministic 2D height value in the
range `[-1.0, 1.0]`. The same seed and coordinates always produce the same
result, while changing the seed produces a different noise field:

```rust
let height = ash_vulkan_triangle::noise::simplex(42, 12.5, -3.25);
```

The demo scene uses seed `42` to place its two triangles at deterministic noise
heights and prints those values when the window starts.

## Chunked planet terrain

`terrain::PlanetTerrain` starts one planet divided into six cube faces with
`4 x 4` tiles per face. Each tile is projected onto the sphere, displaced by
seeded simplex noise, and stored as an indexed mesh. `Scene::update_terrain`
streams tiles whose centers are within the load distance of the camera and
retains a hysteresis band until they move beyond the unload distance. The
current loaded tile count is printed with the camera position each frame.
