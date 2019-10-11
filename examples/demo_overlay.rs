extern crate amethyst;
extern crate amethyst_imgui;
use amethyst::{
	ecs::ReadExpect,
	input::{InputBundle, StringBindings},
	prelude::*,
	renderer::{bundle::RenderingBundle, types::DefaultBackend, RenderToWindow},
	utils::application_root_dir,
};
use amethyst_imgui::{imgui, imgui::im_str, RenderImgui};
use std::sync::{Arc, Mutex};

const DISTANCE: f32 = 10.0;

fn is_mouse_pos_valid(mouse_pos: [f32; 2]) -> bool {
	// Mouse position is set to [f32::MAX, f32::MAX] when invalid
	((std::f32::MAX - mouse_pos[0].abs()) > std::f32::EPSILON) && ((std::f32::MAX - mouse_pos[1].abs()) > std::f32::EPSILON)
}

#[derive(Clone, Copy)]
pub struct DemoSystem {
	corner: i32,
	open: bool,
}

impl Default for DemoSystem {
	fn default() -> Self { DemoSystem { corner: 0, open: true } }
}

impl<'s> amethyst::ecs::System<'s> for DemoSystem {
	type SystemData = (ReadExpect<'s, Arc<Mutex<amethyst_imgui::ImguiContextWrapper>>>,);

	fn run(&mut self, (context,): Self::SystemData) {
		let imgui = &mut context.lock().unwrap().0;
		let io = imgui.io();

		let mut corner = self.corner;
		let mut open = self.open;

		let mut window_pos = [DISTANCE, DISTANCE];
		let mut window_pos_pivot = [0.0, 0.0];

		if corner != -1 {
			if (corner & 1) != 0 {
				window_pos[0] = io.display_size[0] - DISTANCE;
			}
			if (corner & 2) != 0 {
				window_pos[1] = io.display_size[1] - DISTANCE;
			}
			if (corner & 1) != 0 {
				window_pos_pivot[0] = 1.0;
			}
			if (corner & 2) != 0 {
				window_pos_pivot[1] = 1.0;
			}
		}

		amethyst_imgui::with(|ui| {
			let title = im_str!("Example: Simple overlay");
			let mut window = imgui::Window::new(&title)
				.bg_alpha(0.35)
				.movable(corner == -1)
				.no_decoration()
				.always_auto_resize(true)
				.save_settings(false)
				.focus_on_appearing(false)
				.no_nav()
				.opened(&mut open);
			if corner != -1 {
				window = window
					.position(window_pos, imgui::Condition::Always)
					.position_pivot(window_pos_pivot);
			}
			window.build(ui, || {
				ui.text("Simple overlay\nin the corner of the screen");
				ui.separator();
				if is_mouse_pos_valid(io.mouse_pos) {
					ui.text(&format!("Mouse Position: {:.1}, {:.1}", io.mouse_pos[0], io.mouse_pos[1]));
				} else {
					ui.text("Mouse Position: <invalid>");
				}
				let label = im_str!("Location");
				ui.menu(&label, true, || unsafe {
					if imgui::MenuItem::new(&im_str!("Custom")).selected(corner == -1).build(ui) {
						corner = -1;
					}
					if imgui::MenuItem::new(&im_str!("Top-Left")).selected(corner == 0).build(ui) {
						corner = 0;
					}
					if imgui::MenuItem::new(&ImString::new("Top-Right")).selected(corner == 1).build(ui) {
						corner = 1;
					}
					if imgui::MenuItem::new(&ImString::new("Bottom-Left")).selected(corner == 2).build(ui) {
						corner = 2;
					}
					if imgui::MenuItem::new(&ImString::new("Bototm-Right")).selected(corner == 3).build(ui) {
						corner = 3;
					}
					if imgui::MenuItem::new(&ImString::new("Close")).build(ui) {
						self.open = false;
					}
					self.corner = corner;
				});
			});
		});
	}
}

struct Example;
impl SimpleState for Example {}

fn main() -> amethyst::Result<()> {
	amethyst::start_logger(Default::default());
	let app_root = application_root_dir()?;
	let display_config_path = app_root.join("examples/display.ron");

	let game_data = GameDataBuilder::default()
		.with_barrier()
		.with(DemoSystem::default(), "imgui_use", &[])
		.with_bundle(InputBundle::<StringBindings>::default())?
		.with_bundle(
			RenderingBundle::<DefaultBackend>::new()
				.with_plugin(RenderToWindow::from_config_path(display_config_path).with_clear([0.34, 0.36, 0.52, 1.0]))
				.with_plugin(RenderImgui::<StringBindings>::default()),
		)?;

	Application::build("/", Example)?.build(game_data)?.run();

	Ok(())
}
