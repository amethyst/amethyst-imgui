# Usage:
1. Add this to your Stage:
```rust
	.with_pass(amethyst_imgui::DrawUi::default())
```
2. Add this to your `handle_event`:
```rust
	amethyst_imgui::handle_imgui_events(data.world, &event);
```
3. Add this to the beginning of your main `update`:
```rust
	let ui = amethyst_imgui::open_frame(state.world);
	if let Some(ui) = ui {
		ui.show_demo_window(&mut true);
	}
```
4. Add this to the end of your main `update`:
```rust
	if let Some(ui) = ui { amethyst_imgui::close_frame(ui) }
```
