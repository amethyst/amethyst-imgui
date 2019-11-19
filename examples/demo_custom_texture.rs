extern crate amethyst;
extern crate amethyst_imgui;
use amethyst::{
    assets::{AssetStorage, Loader},
    core::legion::*,
    input::{InputBundle, StringBindings},
    prelude::*,
    renderer::{
        legion::{bundle::RenderingBundle, plugins::RenderToWindow},
        rendy::texture::image::{self, load_from_image},
        types::{DefaultBackend, TextureData},
        Texture,
    },
    utils::application_root_dir,
};
use amethyst_imgui::{
    imgui::{self, im_str},
    RenderImgui,
};
use std::sync::{Arc, Mutex};

fn demo_system(
    world: &mut amethyst::core::legion::world::World,
) -> Box<dyn amethyst::core::legion::schedule::Schedulable> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(application_root_dir().unwrap().join("amethyst_emblem.png")).unwrap();
    let mut image_reader = std::io::BufReader::new(&file);

    let texture_builder = load_from_image(
        image_reader,
        image::ImageTextureConfig {
            generate_mips: true,
            ..Default::default()
        },
    )
    .unwrap();

    let handle = {
        let loader = world.resources.get_mut::<Loader>().unwrap();

        let storage = world.resources.get_mut::<AssetStorage<Texture>>().unwrap();
        loader.load_from_data(TextureData(texture_builder), (), &storage)
    };

    {
        let context_mutex = world
            .resources
            .get_mut::<Arc<Mutex<amethyst_imgui::ImguiState>>>()
            .unwrap();
        let mut context = context_mutex.lock().unwrap();
        context.textures.push(handle.clone());
    }

    SystemBuilder::<()>::new("DemoSystem").build(move |_, _, _, _| {
        amethyst_imgui::with(|ui| {
            imgui::Window::new(im_str!("Demo Custom Texture")).build(ui, || {
                ui.text("Hello World");
                imgui::Image::new(imgui::TextureId::from(handle.id() as usize), [256.0, 256.0])
                    .build(ui);
            });
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
        .migration_sync_bundle(amethyst::core::legion::Syncer::default())
        .migration_sync_bundle(amethyst::renderer::legion::Syncer::<DefaultBackend>::default())
        .migration_sync_bundle(amethyst::window::legion::Syncer::default())
        .migration_sync_bundle(amethyst::input::legion::Syncer::<StringBindings>::default())
        .migration_with_bundle(
            RenderingBundle::<DefaultBackend>::default()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)
                        .with_clear([0.0, 0.0, 0.0, 1.0]),
                )
                .with_plugin(RenderImgui::<StringBindings>::default()),
        )
        .migration_with_system(Stage::Begin, demo_system);

    Application::build("/", Example)?.build(game_data)?.run();

    Ok(())
}
