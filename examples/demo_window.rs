extern crate amethyst;
extern crate amethyst_imgui;

use amethyst::{
	ecs::{ReadExpect, Write},
	prelude::*,
	renderer::{DisplayConfig, DrawFlat2D, Pipeline, RenderBundle, Stage},
	utils::application_root_dir,
};

use amethyst_imgui::{imgui, ImguiState};

#[derive(Default, Clone, Copy)]
pub struct ImguiUseSystem;
impl<'s> amethyst::ecs::System<'s> for ImguiUseSystem {
	type SystemData = ();

	fn run(&mut self, _: Self::SystemData) {
		amethyst_imgui::with(|ui| {
			let root_dock = ui.dockspace_over_viewport(None, imgui::ImGuiDockNodeFlags::PassthruDockspace);

			ui.window(imgui::im_str!("Hello world"))
				.size((300.0, 100.0), imgui::ImGuiCond::FirstUseEver)
				.dockspace_id(root_dock, imgui::ImGuiCond::FirstUseEver)
				.build(|| {
					ui.text(imgui::im_str!("Hello world!"));
					ui.text(imgui::im_str!("こんにちは世界！"));
					ui.text(imgui::im_str!("This...is...imgui-rs!"));
					ui.separator();
					let mouse_pos = ui.imgui().mouse_pos();
					ui.text(imgui::im_str!("Mouse Position: ({:.1},{:.1})", mouse_pos.0, mouse_pos.1));
				});

			ui.show_demo_window(&mut true);
		});
	}
}

struct Example;
impl SimpleState for Example {
	fn on_start(&mut self, _: StateData<'_, GameData<'_, '_>>) {}
}

fn main() -> amethyst::Result<()> {
	amethyst::start_logger(amethyst::LoggerConfig::default());

	let pipe = Pipeline::build().with_stage(
		Stage::with_backbuffer()
			.clear_target([0.1, 0.1, 0.1, 1.0], 1.0)
			.with_pass(DrawFlat2D::new())
			.with_pass(amethyst_imgui::DrawUi::default().docking()),
	);

	let game_data = GameDataBuilder::default()
		.with(amethyst_imgui::BeginFrame::default(), "imgui_begin", &[])
		.with_barrier()
		.with(ImguiUseSystem::default(), "imgui_use", &[])
		.with_bundle(RenderBundle::new(pipe, Some(DisplayConfig::default())))?
		.with_barrier()
		.with(amethyst_imgui::EndFrame::default(), "imgui_end", &[]);

	Application::build("/", Example)?.build(game_data)?.run();

	Ok(())
}
