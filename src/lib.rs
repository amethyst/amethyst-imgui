#![allow(clippy::type_complexity)]

use amethyst::{
	core::{
		math::{Vector2, Vector4},
		shrev::EventChannel,
	},
	ecs::{prelude::*, ReadExpect, Write},
	error::Error,
	renderer::{
		pipe::{
			pass::{Pass, PassData},
			Effect,
			NewEffect,
		},
		Attribute,
		Attributes,
		Color,
		Encoder,
		Event,
		Mesh,
		Resources,
		TexCoord,
		VertexFormat,
		WindowMessages,
	},
	winit::MouseCursor,
};
use gfx::{
	format::{ChannelType, Format, Srgba8, SurfaceType},
	memory::Typed,
	preset::blend,
	pso::buffer::{ElemStride, Element},
	state::ColorMask,
	traits::Factory,
};
use glsl_layout::Pod;
pub use imgui;
use imgui::{FontGlyphRange, ImFontConfig, ImGui, ImGuiMouseCursor};
use imgui_gfx_renderer::Renderer as ImguiRenderer;

const VERT_SRC: &[u8] = include_bytes!("shaders/vertex.glsl");
const FRAG_SRC: &[u8] = include_bytes!("shaders/frag.glsl");

struct RendererThing {
	renderer: ImguiRenderer<Resources>,
	texture: gfx::handle::Texture<Resources, gfx::format::R8_G8_B8_A8>,
	shader_resource_view: gfx::handle::ShaderResourceView<Resources, [f32; 4]>,
	mesh: Mesh,
}

#[derive(Default)]
pub struct DrawUi {
	imgui: Option<ImGui>,
	renderer: Option<RendererThing>,
}

pub struct ImguiState {
	pub imgui: ImGui,
	pub mouse_state: MouseState,
	pub size: (u16, u16),
}

impl<'a> PassData<'a> for DrawUi {
	type Data = (ReadExpect<'a, amethyst::renderer::ScreenDimensions>, Write<'a, Option<ImguiState>>);
}

#[derive(Default)]
pub struct ImguiIni(Option<String>);
impl ImguiIni {
	pub fn new(path: &str) -> Self { Self(Some(path.to_owned())) }
}

#[allow(dead_code)]
struct PosTexCol {
	pos: Vector2<f32>,
	uv: Vector2<f32>,
	col: Vector4<f32>,
}

unsafe impl Pod for PosTexCol {}

impl VertexFormat for PosTexCol {
	const ATTRIBUTES: Attributes<'static> = &[
		("pos", Element { offset: 0, format: Format(SurfaceType::R32_G32, ChannelType::Float) }),
		("uv", Element { offset: 8, format: TexCoord::FORMAT }),
		("col", Element { offset: 8 + TexCoord::SIZE, format: Color::FORMAT }),
	];
}

/// Fix incorrect colors with sRGB framebuffer
pub fn imgui_gamma_to_linear(col: imgui::ImVec4) -> imgui::ImVec4 {
	let x = col.x.powf(2.2);
	let y = col.y.powf(2.2);
	let z = col.z.powf(2.2);
	let w = 1.0 - (1.0 - col.w).powf(2.2);
	imgui::ImVec4::new(x, y, z, w)
}

impl Pass for DrawUi {
	fn compile(&mut self, mut effect: NewEffect<'_>) -> Result<Effect, Error> {
		let mut imgui = ImGui::init();

		let style = imgui.style_mut();
		for col in 0..style.colors.len() {
			style.colors[col] = imgui_gamma_to_linear(style.colors[col]);
		}
		imgui.set_ini_filename(None);

		let font_size = 13.;
		let _ = imgui.fonts().add_font_with_config(
			include_bytes!("../mplus-1p-regular.ttf"),
			ImFontConfig::new()
				.oversample_h(1)
				.pixel_snap_h(true)
				.size_pixels(font_size)
				.rasterizer_multiply(1.75),
			&FontGlyphRange::japanese(),
		);

		let _ = imgui.fonts().add_default_font_with_config(
			ImFontConfig::new()
				.merge_mode(true)
				.oversample_h(1)
				.pixel_snap_h(true)
				.size_pixels(font_size),
		);

		imgui_winit_support::configure_keys(&mut imgui);

		let data = vec![
			PosTexCol {
				pos: Vector2::new(0., 1.),
				uv: Vector2::new(0., 0.),
				col: Vector4::new(1., 1., 1., 1.),
			},
			PosTexCol {
				pos: Vector2::new(1., 1.),
				uv: Vector2::new(1., 0.),
				col: Vector4::new(1., 1., 1., 1.),
			},
			PosTexCol {
				pos: Vector2::new(1., 0.),
				uv: Vector2::new(1., 1.),
				col: Vector4::new(1., 1., 1., 1.),
			},
			PosTexCol {
				pos: Vector2::new(0., 1.),
				uv: Vector2::new(0., 0.),
				col: Vector4::new(1., 1., 1., 1.),
			},
			PosTexCol {
				pos: Vector2::new(1., 0.),
				uv: Vector2::new(1., 1.),
				col: Vector4::new(1., 1., 1., 1.),
			},
			PosTexCol {
				pos: Vector2::new(0., 0.),
				uv: Vector2::new(0., 1.),
				col: Vector4::new(1., 1., 1., 1.),
			},
		];

		let (texture, shader_resource_view, target) = effect.factory.create_render_target::<Srgba8>(1024, 1024).unwrap();
		let renderer = ImguiRenderer::init(&mut imgui, effect.factory, (VERT_SRC, FRAG_SRC), target).unwrap();
		self.renderer = Some(RendererThing {
			renderer,
			texture,
			shader_resource_view,
			mesh: Mesh::build(data).build(&mut effect.factory)?,
		});
		self.imgui = Some(imgui);

		effect
			.simple(VERT_SRC, FRAG_SRC)
			.with_raw_global("matrix")
			.with_raw_vertex_buffer(PosTexCol::ATTRIBUTES, PosTexCol::size() as ElemStride, 0)
			.with_texture("tex")
			.with_blended_output("Target0", ColorMask::all(), blend::ALPHA, None)
			.build()
	}

	fn apply<'ui, 'apply_pd: 'ui>(
		&'ui mut self,
		encoder: &mut Encoder,
		effect: &mut Effect,
		mut factory: amethyst::renderer::Factory,
		(screen_dimensions, mut imgui_state): <Self as PassData<'apply_pd>>::Data,
	) {
		let imgui_state = imgui_state.get_or_insert_with(|| ImguiState {
			imgui: self.imgui.take().unwrap(),
			mouse_state: MouseState::default(),
			size: (1024, 1024),
		});
		imgui_state.imgui.set_font_global_scale(screen_dimensions.hidpi_factor() as f32);

		let (width, height) = (screen_dimensions.width(), screen_dimensions.height());
		if width <= 0. || height <= 0. {
			return;
		}
		let renderer_thing = self.renderer.as_mut().unwrap();

		let matrix = [
			[2.0, 0.0, 0.0, 0.0],
			[0.0, -2.0, 0.0, 0.0],
			[0.0, 0.0, -1.0, 0.0],
			[-1.0, 1.0, 0.0, 1.0],
		];

		if imgui_state.size.0 != width as u16 || imgui_state.size.1 != height as u16 {
			let (texture, shader_resource_view, target) = factory.create_render_target::<Srgba8>(width as u16, height as u16).unwrap();
			renderer_thing.renderer.update_render_target(target);
			renderer_thing.shader_resource_view = shader_resource_view;
			renderer_thing.texture = texture;
		}

		encoder.clear(
			&factory
				.view_texture_as_render_target::<Srgba8>(&renderer_thing.texture, 0, None)
				.unwrap(),
			[0., 0., 0., 0.],
		);

		unsafe {
			if let Some(ui) = imgui::Ui::current_ui() {
				let ui = ui as *const imgui::Ui;
				renderer_thing.renderer.render(ui.read(), &mut factory, encoder).unwrap();
			}
		}

		{
			use gfx::texture::{FilterMethod, SamplerInfo, WrapMode};
			let sampler = factory.create_sampler(SamplerInfo::new(FilterMethod::Trilinear, WrapMode::Clamp));
			effect.data.samplers.push(sampler);
		}

		effect.update_global("matrix", matrix);
		effect.data.textures.push(renderer_thing.shader_resource_view.raw().clone());
		effect
			.data
			.vertex_bufs
			.push(renderer_thing.mesh.buffer(PosTexCol::ATTRIBUTES).unwrap().clone());

		effect.draw(renderer_thing.mesh.slice(), encoder);

		effect.data.textures.clear();
		effect.data.samplers.clear();
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

#[derive(Default)]
pub struct BeginFrame {
	reader: Option<ReaderId<Event>>,
}
impl<'s> amethyst::ecs::System<'s> for BeginFrame {
	type SystemData = (
		Read<'s, EventChannel<Event>>,
		ReadExpect<'s, amethyst::renderer::ScreenDimensions>,
		ReadExpect<'s, amethyst::core::timing::Time>,
		Write<'s, Option<ImguiState>>,
		Write<'s, WindowMessages>,
		Read<'s, ImguiIni>,
	);

	fn setup(&mut self, res: &mut amethyst::ecs::Resources) {
		Self::SystemData::setup(res);
		self.reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
	}

	fn run(&mut self, (events, dimensions, time, mut imgui_state, mut window_messages, ini_path): Self::SystemData) {
		let dimensions: &amethyst::renderer::ScreenDimensions = &dimensions;
		let time: &amethyst::core::timing::Time = &time;

		if dimensions.width() <= 0. || dimensions.height() <= 0. {
			return;
		}

		let imgui_state = if let Some(x) = &mut imgui_state as &mut Option<ImguiState> { x } else { return; };

		if let Some(path) = &ini_path.0 {
			imgui_state.imgui.set_ini_filename(Some(imgui::ImString::new(path.clone())));
		}

		let dpi = dimensions.hidpi_factor();
		for event in events.read(self.reader.as_mut().unwrap()) {
			imgui_winit_support::handle_event(&mut imgui_state.imgui, &event, dpi as f64, dpi as f64);
		}
		update_mouse_cursor(&imgui_state.imgui, &mut window_messages);

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
