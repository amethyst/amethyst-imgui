# Usage:
1. Add `.with_pass(amethyst_imgui::DrawUi::new())` to your Stage
1. Add this to your `handle_event`:
```rust
	let imgui_state: &mut Option<amethyst_imgui::ImguiState> = &mut data.world.write_resource::<Option<amethyst_imgui::ImguiState>>();
	if let Some(ref mut imgui_state) = imgui_state {
		amethyst_imgui::handle_imgui_events(imgui_state, &event);
	}
```
1. Add this to your main `update`:
```rust
	let imgui_state: &mut Option<amethyst_imgui::ImguiState> = &mut state.world.write_resource::<Option<amethyst_imgui::ImguiState>>();
	if let Some(ref mut imgui_state) = imgui_state {
		imgui_state.run_ui = Some(Box::new(move |ui: &mut imgui::Ui<'_>| {
			ui.show_demo_window(&mut true);
			ui.window(im_str!("TEST WINDOW WOOO")).build(|| {
				ui.text(im_str!("{}", seconds));
			});
		}));
	}
```
