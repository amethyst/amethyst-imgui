#![feature(custom_attribute)]
#![allow(clippy::type_complexity, dead_code)]

use amethyst::{
	assets::{AssetStorage, Handle, Loader},
	ecs::{prelude::*, ReadExpect},
	renderer::{
		rendy::{
			hal::{
				format::Format,
				image::{self, Anisotropic, Filter, PackedColor, SamplerInfo, WrapMode},
			},
			texture::TextureBuilder,
		},
		types::TextureData,
		Texture,
	},
	window::{ScreenDimensions, Window},
};

mod pass;

pub use imgui;
use imgui::{FontConfig, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
pub use pass::DrawImguiDesc;

pub struct ImguiState {
	pub platform: WinitPlatform,
	pub config: ImguiConfig,
	pub textures: Vec<Handle<Texture>>,
}

#[derive(Clone, Debug)]
pub struct ImguiConfig {
	pub ini: Option<String>,
	pub screen_dimensions: Option<ScreenDimensions>,
	pub font: FontConfig,
}
impl Default for ImguiConfig {
	fn default() -> Self {
		Self {
			font: FontConfig {
				size_pixels: 13.,
				..FontConfig::default()
			},
			ini: None,
			screen_dimensions: None,
		}
	}
}
impl ImguiState {
	pub fn new(res: &amethyst::ecs::Resources, config: ImguiConfig) -> Self {
		type SetupData<'a> = (
			Read<'a, AssetStorage<Texture>>,
			ReadExpect<'a, ScreenDimensions>,
			ReadExpect<'a, Window>,
		);
		SetupData::setup(res);

		// Initialize everything
		let mut imgui = imgui::Context::create();
		let mut platform = WinitPlatform::init(&mut imgui);
		platform.attach_window(imgui.io_mut(), &res.fetch::<Window>(), HiDpiMode::Default);

		imgui.set_ini_filename(config.ini.as_ref().map(|i| imgui::ImString::new(i)));
		imgui.fonts().add_font(&[FontSource::DefaultFontData { config: Some(config.font) }]);

		let texture_handle = {
			let handle = imgui.fonts().build_rgba32_texture();

			let loader = res.fetch_mut::<Loader>();
			let texture_storage = res.fetch_mut::<AssetStorage<Texture>>();

			let texture_builder = TextureBuilder::new()
				.with_data_width(handle.width)
				.with_data_height(handle.height)
				.with_kind(image::Kind::D2(handle.width, handle.height, 1, 1))
				.with_view_kind(image::ViewKind::D2)
				.with_sampler_info(SamplerInfo {
					min_filter: Filter::Linear,
					mag_filter: Filter::Linear,
					mip_filter: Filter::Linear,
					wrap_mode: (WrapMode::Clamp, WrapMode::Clamp, WrapMode::Clamp),
					lod_bias: 0.0.into(),
					lod_range: std::ops::Range {
						start: 0.0.into(),
						end: 1000.0.into(),
					},
					comparison: None,
					border: PackedColor(0),
					anisotropic: Anisotropic::Off,
				})
				.with_raw_data(handle.data, Format::Rgba8Unorm);

			let tex: Handle<Texture> = loader.load_from_data(TextureData(texture_builder), (), &texture_storage);
			tex
		};

		Self {
			imgui,
			platform,
			config,
			textures: vec![texture_handle],
		}
	}
}
