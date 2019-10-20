extern crate amethyst;
extern crate amethyst_imgui;
use amethyst::{
	core::legion::Stage,
	input::{InputBundle, StringBindings},
	prelude::*,
	renderer::{
		legion::{bundle::RenderingBundle, plugins::RenderToWindow},
		types::DefaultBackend,
	},
	utils::application_root_dir,
};

use amethyst_imgui::RenderImgui;

fn demo_system(_: &mut amethyst::core::legion::world::World) -> Box<dyn amethyst::core::legion::schedule::Schedulable> {
	SystemBuilder::<()>::new("DemoSystem").build(move |_, _, _, _| {
		amethyst_imgui::with(|ui| {
			ui.show_demo_window(&mut true);
		});
	})
}

struct Example;
impl SimpleState for Example {}

fn main() -> amethyst::Result<()> {
	amethyst::start_logger(Default::default());
	let app_root = application_root_dir()?;
	let display_config_path = app_root.join("examples/display.ron");

	let game_data = GameDataBuilder::default()
		.with_bundle(InputBundle::<StringBindings>::default())?
		.migration_with_system(Stage::Begin, demo_system)
		.migration_sync_bundle(amethyst::core::legion::Syncer::default())
		.migration_sync_bundle(amethyst::renderer::legion::Syncer::<DefaultBackend>::default())
		.migration_sync_bundle(amethyst::window::legion::Syncer::default())
		.migration_sync_bundle(amethyst::input::legion::Syncer::<StringBindings>::default())
		.migration_with_bundle(
			RenderingBundle::<DefaultBackend>::default()
				.with_plugin(RenderToWindow::from_config_path(display_config_path).with_clear([0.0, 0.0, 0.0, 1.0]))
				.with_plugin(RenderImgui::<StringBindings>::default()),
		);

	Application::build("/", Example)?.build(game_data)?.run();

	Ok(())
}
