extern crate amethyst;
extern crate gfx;
extern crate glsl_layout;
pub extern crate imgui;
extern crate imgui_gfx_renderer;

use amethyst::{
	core::{
		math::{Vector2, Vector3},
		shrev::{EventChannel, EventIterator},
	},
	ecs::{prelude::*, ReadExpect, Write},
	error::Error,
	renderer::{
		pipe::{
			pass::{Pass, PassData},
			Effect,
			NewEffect,
		},
		ElementState,
		Encoder,
		Event,
		Mesh,
		MouseButton,
		PosTex,
		Resources,
		VertexFormat,
		VirtualKeyCode as VK,
		WindowEvent,
		WindowMessages,
	},
	winit::{MouseCursor, MouseScrollDelta, TouchPhase},
};
use gfx::{memory::Typed, preset::blend, pso::buffer::ElemStride, state::ColorMask, traits::Factory};
use glsl_layout::{vec2, vec4, Uniform};
use imgui::{FontGlyphRange, ImFontConfig, ImGui, ImGuiMouseCursor};
use imgui_gfx_renderer::{Renderer as ImguiRenderer, Shaders};

const VERT_SRC: &[u8] = include_bytes!("shaders/vertex.glsl");
const FRAG_SRC: &[u8] = include_bytes!("shaders/frag.glsl");

#[derive(Copy, Clone, Debug, Uniform)]
#[allow(dead_code)] // This is used by the shaders
#[repr(C)]
struct VertexArgs {
	proj_vec: vec4,
	coord: vec2,
	dimension: vec2,
}

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

type FormattedT = (gfx::format::R8_G8_B8_A8, gfx::format::Unorm);

impl<'a> PassData<'a> for DrawUi {
	type Data = (ReadExpect<'a, amethyst::renderer::ScreenDimensions>, Write<'a, Option<ImguiState>>);
}

impl Pass for DrawUi {
	fn compile(&mut self, mut effect: NewEffect<'_>) -> Result<Effect, Error> {
		let mut imgui = ImGui::init();

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
			PosTex {
				position: Vector3::new(0., 1., 0.),
				tex_coord: Vector2::new(0., 0.),
			},
			PosTex {
				position: Vector3::new(1., 1., 0.),
				tex_coord: Vector2::new(1., 0.),
			},
			PosTex {
				position: Vector3::new(1., 0., 0.),
				tex_coord: Vector2::new(1., 1.),
			},
			PosTex {
				position: Vector3::new(0., 1., 0.),
				tex_coord: Vector2::new(0., 0.),
			},
			PosTex {
				position: Vector3::new(1., 0., 0.),
				tex_coord: Vector2::new(1., 1.),
			},
			PosTex {
				position: Vector3::new(0., 0., 0.),
				tex_coord: Vector2::new(0., 1.),
			},
		];

		let (texture, shader_resource_view, target) = effect.factory.create_render_target::<FormattedT>(1024, 1024).unwrap();
		let renderer = ImguiRenderer::init(&mut imgui, effect.factory, Shaders::GlSl130, target).unwrap();
		self.renderer = Some(RendererThing {
			renderer,
			texture,
			shader_resource_view,
			mesh: Mesh::build(data).build(&mut effect.factory)?,
		});
		self.imgui = Some(imgui);

		effect
			.simple(VERT_SRC, FRAG_SRC)
			.with_raw_constant_buffer("VertexArgs", std::mem::size_of::<<VertexArgs as Uniform>::Std140>(), 1)
			.with_raw_vertex_buffer(PosTex::ATTRIBUTES, PosTex::size() as ElemStride, 0)
			.with_texture("albedo")
			.with_blended_output("color", ColorMask::all(), blend::ALPHA, None)
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

		let vertex_args = VertexArgs {
			proj_vec: [2. / width, -2. / height, 0., 1.].into(),
			coord: [0., 0.].into(),
			dimension: [width, height].into(),
		};

		if imgui_state.size.0 != width as u16 || imgui_state.size.1 != height as u16 {
			let (texture, shader_resource_view, target) = factory.create_render_target::<FormattedT>(width as u16, height as u16).unwrap();
			renderer_thing.renderer.update_render_target(target);
			renderer_thing.shader_resource_view = shader_resource_view;
			renderer_thing.texture = texture;
		}

		encoder.clear(
			&factory
				.view_texture_as_render_target::<FormattedT>(&renderer_thing.texture, 0, None)
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

		effect.update_constant_buffer("VertexArgs", &vertex_args.std140(), encoder);
		effect.data.textures.push(renderer_thing.shader_resource_view.raw().clone());
		effect
			.data
			.vertex_bufs
			.push(renderer_thing.mesh.buffer(PosTex::ATTRIBUTES).unwrap().clone());

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
	);

	fn setup(&mut self, res: &mut amethyst::ecs::Resources) {
		Self::SystemData::setup(res);
		self.reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
	}

	fn run(&mut self, (events, dimensions, time, mut imgui_state, mut window_messages): Self::SystemData) {
		let dimensions: &amethyst::renderer::ScreenDimensions = &dimensions;
		let time: &amethyst::core::timing::Time = &time;

		if dimensions.width() <= 0. || dimensions.height() <= 0. {
			return;
		}

		let imgui_state = if let Some(x) = &mut imgui_state as &mut Option<ImguiState> { x } else { return; };

		let dpi = dimensions.hidpi_factor();
		for event in events.read(self.reader.as_mut().unwrap()) {
			imgui_winit_support::handle_event(&mut imgui_state.imgui, &event, dpi as f64, dpi as f64);
		}
		update_mouse_cursor(&mut imgui_state.imgui, &mut window_messages);

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

	fn run(&mut self, _: Self::SystemData) {
		with(|_| {});
	}
}
