extern crate amethyst;
extern crate amethyst_imgui;
use amethyst::{
	assets::{AssetLoaderSystemData, AssetStorage, Handle, Loader},
	ecs::prelude::*,
	input::{InputBundle, StringBindings},
	prelude::*,
	renderer::{
		bundle::RenderingBundle,
		rendy::texture::image::{self, load_from_image},
		types::{DefaultBackend, TextureData},
		RenderToWindow,
		Texture,
	},
	utils::application_root_dir,
};

use amethyst_imgui::{imgui::im_str, ImguiStatePtr, RenderImgui};

pub struct DemoSystem {
	image_handle: Handle<Texture>,
}
impl<'s> amethyst::ecs::System<'s> for DemoSystem {
	type SystemData = ();

	fn run(&mut self, _: Self::SystemData) {
		amethyst_imgui::with(|ui| {
			imgui::Window::new(im_str!("Demo Custom Texture")).build(ui, || {
				ui.text("Hello World");
				imgui::Image::new(imgui::TextureId::from(self.image_handle.id() as usize), [256.0, 256.0]).build(ui);
			});
		});
	}
}

#[derive(Default)]
struct DemoSystemDesc;
impl<'a, 'b> SystemDesc<'a, 'b, DemoSystem> for DemoSystemDesc {
	fn build(self, world: &mut World) -> DemoSystem {
		<DemoSystem as System<'_>>::SystemData::setup(world);

		let file = std::fs::File::open(application_root_dir().unwrap().join("amethyst_emblem.png")).unwrap();
		let image_reader = std::io::BufReader::new(&file);

		let texture_builder = load_from_image(
			image_reader,
			image::ImageTextureConfig {
				generate_mips: true,
				..Default::default()
			},
		)
		.unwrap();

		let image_handle = world.exec(|loader: AssetLoaderSystemData<'_, Texture>| loader.load_from_data(TextureData(texture_builder), ()));
		{
			let context_mutex = world.fetch::<ImguiStatePtr>();
			let mut context = context_mutex.lock().unwrap();
			context.textures.push(image_handle.clone());
		}

		DemoSystem { image_handle }
	}
}

struct Example;
impl SimpleState for Example {}

fn main() -> amethyst::Result<()> {
	amethyst::start_logger(Default::default());
	let app_root = application_root_dir()?;
	let display_config_path = app_root.join("examples/display.ron");

	let game_data = GameDataBuilder::default()
		.with_bundle(InputBundle::<StringBindings>::default())?
		.with_bundle(
			RenderingBundle::<DefaultBackend>::new()
				.with_plugin(RenderToWindow::from_config_path(display_config_path)?.with_clear([0.34, 0.36, 0.52, 1.0]))
				.with_plugin(RenderImgui::<StringBindings>::default()),
		)?
		.with_system_desc(DemoSystemDesc::default(), "imgui_use", &[]);

	Application::build("/", Example)?.build(game_data)?.run();

	Ok(())
}
