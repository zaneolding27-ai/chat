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
