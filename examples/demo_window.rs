extern crate amethyst;
extern crate amethyst_imgui;
use amethyst::{
    ecs::{ReadExpect, Resources, SystemData},
    prelude::*,
    renderer::{
        rendy::{
            factory::Factory,
            graph::{
                render::{RenderGroupDesc, SubpassBuilder},
                GraphBuilder,
            },
            hal::{format::Format, image},
        },
        types::DefaultBackend,
        GraphCreator, RenderingSystem,
    },
    utils::application_root_dir,
    window::{ScreenDimensions, Window, WindowBundle},
};

use amethyst_imgui::{imgui, DrawImguiDesc};

#[derive(Default, Clone, Copy)]
pub struct ImguiUseSystem;
impl<'s> amethyst::ecs::System<'s> for ImguiUseSystem {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
        amethyst_imgui::with(|ui| {
            ui.window(imgui::im_str!("Hello world"))
                .size((300.0, 100.0), imgui::ImGuiCond::FirstUseEver)
                .build(|| {
                    ui.text(imgui::im_str!("Hello world!"));
                    ui.text(imgui::im_str!("こんにちは世界！"));
                    ui.text(imgui::im_str!("This...is...imgui-rs!"));
                    ui.separator();
                    let mouse_pos = ui.imgui().mouse_pos();
                    ui.text(imgui::im_str!(
                        "Mouse Position: ({:.1},{:.1})",
                        mouse_pos.0,
                        mouse_pos.1
                    ));
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
    amethyst::start_logger(Default::default());
    let app_root = application_root_dir()?;
    let display_config_path = app_root.join("examples/display.ron");

    let game_data = GameDataBuilder::default()
        .with_bundle(WindowBundle::from_config_path(display_config_path))?
        .with(amethyst_imgui::System::default(), "imgui_begin", &[])
        .with_barrier()
        .with(ImguiUseSystem::default(), "imgui_use", &["imgui_begin"])
        .with_barrier()
        .with_thread_local(RenderingSystem::<DefaultBackend, _>::new(
            ExampleGraph::new(),
        ));

    Application::build("/", Example)?.build(game_data)?.run();

    Ok(())
}

struct ExampleGraph {
    dimensions: Option<ScreenDimensions>,
    surface_format: Option<Format>,
    dirty: bool,
}

impl ExampleGraph {
    pub fn new() -> Self {
        Self {
            dimensions: None,
            surface_format: None,
            dirty: true,
        }
    }
}

impl GraphCreator<DefaultBackend> for ExampleGraph {
    fn rebuild(&mut self, res: &Resources) -> bool {
        // Rebuild when dimensions change, but wait until at least two frames have the same.
        let new_dimensions = res.try_fetch::<ScreenDimensions>();
        use std::ops::Deref;
        if self.dimensions.as_ref() != new_dimensions.as_ref().map(|d| d.deref()) {
            self.dirty = true;
            self.dimensions = new_dimensions.map(|d| d.clone());
            return false;
        }
        return self.dirty;
    }

    fn builder(
        &mut self,
        factory: &mut Factory<DefaultBackend>,
        res: &Resources,
    ) -> GraphBuilder<DefaultBackend, Resources> {
        use amethyst::renderer::rendy::{
            graph::present::PresentNode,
            hal::command::{ClearDepthStencil, ClearValue},
        };

        self.dirty = false;

        let window = <ReadExpect<'_, Window>>::fetch(res);
        let surface = factory.create_surface(&window);
        // cache surface format to speed things up
        let surface_format = *self
            .surface_format
            .get_or_insert_with(|| factory.get_surface_format(&surface));

        let mut graph_builder = GraphBuilder::new();
        let dimensions = self.dimensions.as_ref().unwrap();

        let window_kind =
            image::Kind::D2(dimensions.width() as u32, dimensions.height() as u32, 1, 1);

        let color = graph_builder.create_image(
            window_kind,
            1,
            surface_format,
            Some(ClearValue::Color([0.34, 0.36, 0.52, 1.0].into())),
        );

        let depth = graph_builder.create_image(
            window_kind,
            1,
            Format::D32Sfloat,
            Some(ClearValue::DepthStencil(ClearDepthStencil(1.0, 0))),
        );

        let imgui = graph_builder.add_node(
            SubpassBuilder::new()
                .with_group(DrawImguiDesc::default().builder())
                .with_color(color)
                .with_depth_stencil(depth)
                .into_pass(),
        );

        let _present = graph_builder
            .add_node(PresentNode::builder(factory, surface, color).with_dependency(imgui));

        graph_builder
    }
}
