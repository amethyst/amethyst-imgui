[![Crates.io](https://img.shields.io/crates/v/amethyst-imgui.svg)](https://crates.io/crates/amethyst-imgui/)

# Usage:
1. Add this to your Stage:
```rust
.with_pass(amethyst_imgui::DrawUi::default())
```
2. Add this to `GameDataBuilder`:
```rust
GameDataBuilder::default()
	.with(amethyst_imgui::BeginFrame::default(), "imgui_begin", &[])
	.with_barrier()
	// --- everything else ---
	.with_barrier()
	.with(amethyst_imgui::EndFrame::default(), "imgui_end", &["imgui_begin"]);
```
3. Use it in any `System`:
```rust
amethyst_imgui::with(|ui| {
	ui.show_demo_window(&mut true);
});
```

# Example:
```
cargo run --example demo_window
```
