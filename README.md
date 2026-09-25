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

## Resource manager

`resource_manager::ResourceManager` loads relative paths from a configured directory,
keeps recently used file contents in memory, and evicts the least recently used
resources when the byte budget is exceeded:

```rust
let mut resources = ResourceManager::new("assets", 16 * 1024 * 1024);
let shader = resources.load("shaders/example.spv")?;
```

Absolute paths and paths that traverse outside the resource directory are rejected.
