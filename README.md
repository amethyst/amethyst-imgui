[![Crates.io](https://img.shields.io/crates/v/amethyst-imgui.svg)](https://crates.io/crates/amethyst-imgui/)

# Usage has changed in 0.3.0.

1. include the 0.11 render pass, `DrawImguiDesc`, with the appropriate config.
2. Render windows using the `amethyst_imgui::with` function.
```rust
amethyst_imgui::with(|ui| {
	ui.show_demo_window(&mut true);
});
```

No systems or other shinanigans are required now. All imgui state management occurs within the imgui render pass.