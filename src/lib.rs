#![feature(custom_attribute)]

#![allow(clippy::type_complexity)]

use amethyst::{
	assets::{Loader, AssetStorage, Handle},
	core::{
		shrev::EventChannel,
	},
	renderer::{
		Texture,
	},
	ecs::{prelude::*, ReadExpect},
	window::{ScreenDimensions},
	winit::Event,
};
pub use imgui;
use imgui::{FontGlyphRange, ImFontConfig, ImGui};

mod pass;
pub use pass::DrawImguiDesc;

pub struct ImguiState {
	pub imgui: ImGui,
	pub mouse_state: MouseState,
	pub size: (u16, u16),
	pub config: ImguiConfig,
	pub textures: Vec<Handle<Texture>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImguiConfig {
	pub font_size: f32,
	ini: Option<String>,
	font: Vec<u8>,
}
impl Default for ImguiConfig {
	fn default() -> Self {
		Self {
			font: include_bytes!("../mplus-1p-regular.ttf").to_vec(),
			font_size: 13.,
			ini: None,
		}
	}
}
impl ImguiState {
	pub fn new(res: &mut amethyst::ecs::Resources, config: ImguiConfig) -> Self {
		type SetupData<'a> = (Read<'a, AssetStorage<Texture>>);
		SetupData::setup(res);

		// Initialize everything
		let mut imgui = ImGui::init();

		imgui.set_ini_filename(config.ini.as_ref().map(|i| imgui::ImString::new(i) ));


		let _ = imgui.fonts().add_font_with_config(
			&config.font,
			ImFontConfig::new()
				.oversample_h(1)
				.pixel_snap_h(true)
				.size_pixels(config.font_size)
				.rasterizer_multiply(1.75),
			&FontGlyphRange::japanese(),
		);

		let _ = imgui.fonts().add_default_font_with_config(
			ImFontConfig::new()
				.merge_mode(true)
				.oversample_h(1)
				.pixel_snap_h(true)
				.size_pixels(config.font_size),
		);

		let texture_handle = imgui.prepare_texture(|handle| {
			let loader = res.fetch_mut::<Loader>();
			let texture_storage = res.fetch_mut::<AssetStorage<Texture>>();

			use amethyst::renderer::{
				types::TextureData,
				rendy::{
					texture::TextureBuilder,
					hal::image
				}
			};

			use amethyst::renderer::rendy::{
				hal::image::{Anisotropic, PackedColor, SamplerInfo, WrapMode, Filter},
				hal::format::Format,
				texture::image::{Repr, TextureKind},
			};

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
				.with_raw_data(handle.pixels, Format::Rgba8Unorm);

			let tex: Handle<Texture> = loader.load_from_data(
				TextureData(texture_builder),
				(),
				&texture_storage,
			);
			tex
		});


		Self {
			imgui,
			mouse_state: MouseState::default(),
			size: (1024, 1024),
			config,
			textures: vec![texture_handle],
		}
	}
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct MouseState {
	pos: (i32, i32),
	pressed: (bool, bool, bool),
	wheel: f32,
}

pub fn with(f: impl FnOnce(&imgui::Ui)) {
	unsafe {
		if let Some(ui) = imgui::Ui::current_ui() {
			(ui as *const imgui::Ui<'_>).read_volatile();
			f(ui);
		}
	}
}

/*
fn update_mouse_cursor(imgui: &ImGui, messages: &mut WindowMessages) {
	let mouse_cursor = imgui.mouse_cursor();
	if imgui.mouse_draw_cursor() || mouse_cursor == ImGuiMouseCursor::None {
		messages.send_command(move |win| win.hide_cursor(true));
	} else {
		messages.send_command(move |win| {
			win.hide_cursor(false);
			win.set_cursor(match mouse_cursor {
				ImGuiMouseCursor::None => unreachable!("mouse_cursor was None!"),
				ImGuiMouseCursor::Arrow => MouseCursor::Arrow,
				ImGuiMouseCursor::TextInput => MouseCursor::Text,
				ImGuiMouseCursor::ResizeAll => MouseCursor::Move,
				ImGuiMouseCursor::ResizeNS => MouseCursor::NsResize,
				ImGuiMouseCursor::ResizeEW => MouseCursor::EwResize,
				ImGuiMouseCursor::ResizeNESW => MouseCursor::NeswResize,
				ImGuiMouseCursor::ResizeNWSE => MouseCursor::NwseResize,
				ImGuiMouseCursor::Hand => MouseCursor::Hand,
			});
		});
	}
}
*/

#[derive(Default)]
pub struct BeginFrame {
	reader: Option<ReaderId<Event>>,
}
impl<'s> amethyst::ecs::System<'s> for BeginFrame {
	type SystemData = (
		Read<'s, EventChannel<Event>>,
		ReadExpect<'s, ScreenDimensions>,
		ReadExpect<'s, amethyst::core::timing::Time>,
		WriteExpect<'s, ImguiState>,
	);

	fn setup(&mut self, res: &mut amethyst::ecs::Resources) {
		Self::SystemData::setup(res);

		self.reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());

		let state = ImguiState::new(res, ImguiConfig::default());
		res.insert(state);

	}

	fn run(&mut self, (events, dimensions, time, mut imgui_state): Self::SystemData) {
		let dimensions: &ScreenDimensions = &dimensions;
		let time: &amethyst::core::timing::Time = &time;

		if dimensions.width() <= 0. || dimensions.height() <= 0. {
			return;
		}

		if imgui_state.size.0 != dimensions.width() as u16 || imgui_state.size.1 != dimensions.height() as u16 {
			imgui_state.size = (dimensions.width() as u16, dimensions.height() as u16);
		}

		//if let Some(path) = &ini_path.0 {
		//	imgui_state.imgui.set_ini_filename(Some(imgui::ImString::new(path.clone())));
		//}

		let dpi = dimensions.hidpi_factor();
		for event in events.read(self.reader.as_mut().unwrap()) {
			// TODO: This is broken because of winit versions
		    //	imgui_winit_support::handle_event(&mut imgui_state.imgui, event, dpi as f64, dpi as f64);
		}
		//update_mouse_cursor(&imgui_state.imgui, &mut window_messages);

		let frame = imgui_state.imgui.frame(
			imgui::FrameSize::new(f64::from(dimensions.width()), f64::from(dimensions.height()), dpi),
			time.delta_seconds(),
		);
		std::mem::forget(frame);
	}
}

#[derive(Default, Clone, Copy)]
pub struct EndFrame;
impl<'s> amethyst::ecs::System<'s> for EndFrame {
	type SystemData = ();

	fn run(&mut self, _: Self::SystemData) { with(|_| {}); }
}
