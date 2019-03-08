extern crate amethyst;
extern crate amethyst_imgui;

use amethyst::{
    ecs::{Write, ReadExpect},
    prelude::*,
    renderer::{DisplayConfig, DrawFlat2D, Pipeline, RenderBundle, Stage, },
    utils::application_root_dir,
};

use amethyst_imgui::ImguiState;
use amethyst_imgui::imgui;

#[derive(Default)]
pub struct ImguiBeginFrameSystem;
impl ImguiBeginFrameSystem {
    pub fn open_frame<'ui>(
        &mut self,
        dimensions: &amethyst::renderer::ScreenDimensions,
        time: &amethyst::core::timing::Time,
        imgui_state: &mut Option<ImguiState>,
    ) -> Option<&'ui imgui::Ui<'ui>>
    {
        let dimensions: &amethyst::renderer::ScreenDimensions = &dimensions;
        let time: &amethyst::core::timing::Time = &time;

        if dimensions.width() <= 0. || dimensions.height() <= 0. {
            return None;
        }

        let imgui = match imgui_state {
            Some(x) => &mut x.imgui,
            _ => return None,
        };

        let frame = imgui.frame(imgui::FrameSize::new(f64::from(dimensions.width()), f64::from(dimensions.height()), 1.), time.delta_seconds());
        std::mem::forget(frame);
        unsafe { imgui::Ui::current_ui() }
    }
}
impl<'s> amethyst::ecs::System<'s> for ImguiBeginFrameSystem {
    type SystemData = (
        ReadExpect<'s, amethyst::renderer::ScreenDimensions>,
        ReadExpect<'s, amethyst::core::timing::Time>,
        Write<'s, Option<ImguiState>>,
    );

    fn run(&mut self, (dimensions, time, mut imgui_state, ): Self::SystemData) {
        self.open_frame(&dimensions, &time, &mut imgui_state);
    }
}

#[derive(Default)]
pub struct ImguiEndFrameSystem;
impl<'s> amethyst::ecs::System<'s> for ImguiEndFrameSystem {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
        unsafe {
            if let Some(ui) = imgui::Ui::current_ui() {
                (ui as *const imgui::Ui).read_volatile();
                let root_dock = ui.dockspace_over_viewport(None, imgui::ImGuiDockNodeFlags::PassthruDockspace );

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
            }
        };
    }
}

struct Example;
impl SimpleState for Example {
    fn on_start(&mut self, _: StateData<'_, GameData<'_, '_>>) {
    }
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let resources = application_root_dir()?.join("examples/simple_image/resources");
    let config = DisplayConfig::load(resources.join("display_config.ron"));
    let pipe = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.1, 0.1, 0.1, 1.0], 1.0)
            .with_pass(DrawFlat2D::new())
            .with_pass(amethyst_imgui::DrawUi::default().docking()),
    );

    let game_data = GameDataBuilder::default()
        // All systems which utilize imgui need to be dependent on this to make sure to call AFTER the frame has started
        .with(ImguiBeginFrameSystem::default(), "imgui_begin_frame", &[])
        // ImguiEndFrame needs to be dependent on all systems which use imgui, to make sure its called at the end of the frame
        .with(ImguiEndFrameSystem::default(), "imgui_end_frame", &["imgui_begin_frame"])
        .with_bundle(RenderBundle::new(pipe, Some(config)))?;

    let mut game = Application::build(resources, Example)?.build(game_data)?;
    game.run();

    Ok(())
}