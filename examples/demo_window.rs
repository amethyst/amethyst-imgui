extern crate amethyst;
extern crate amethyst_imgui;
use amethyst::{
	input::{InputBundle, StringBindings},
	prelude::*,
	renderer::{bundle::RenderingBundle, types::DefaultBackend, RenderToWindow},
	utils::application_root_dir,
};

use amethyst_imgui::RenderImgui;

#[derive(Default, Clone, Copy)]
pub struct DemoSystem;
impl<'s> amethyst::ecs::System<'s> for DemoSystem {
	type SystemData = ();

	fn run(&mut self, _: Self::SystemData) {
		amethyst_imgui::with(|ui| {
			ui.show_demo_window(&mut true);
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
				.with_plugin(RenderToWindow::from_config_path(display_config_path)?.with_clear([0.34, 0.36, 0.52, 1.0]))
				.with_plugin(RenderImgui::<StringBindings>::default()),
		)?;

	Application::build("/", Example)?.build(game_data)?.run();

	Ok(())
}
