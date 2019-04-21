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
	},
	winit::{MouseScrollDelta, TouchPhase},
};
use gfx::{memory::Typed, preset::blend, pso::buffer::ElemStride, state::ColorMask, traits::Factory};
use glsl_layout::{vec2, vec4, Uniform};
use imgui::{FontGlyphRange, ImFontConfig, ImGui, ImVec4};
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

macro_rules! keys {
	($m:ident) => {
		$m![
			Tab => Tab,
			LeftArrow => Left,
			RightArrow => Right,
			UpArrow => Up,
			DownArrow => Down,
			PageUp => PageUp,
			PageDown => PageDown,
			Home => Home,
			End => End,
			Insert => Insert,
			Delete => Delete,
			Backspace => Back,
			Space => Space,
			Enter => Return,
			Escape => Escape,
			A => A,
			C => C,
			V => V,
			X => X,
			Y => Y,
			Z => Z,
		];
	};
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

		{
			macro_rules! set_keys {
				($($key:ident => $id:ident),+$(,)*) => {
					$(imgui.set_imgui_key(imgui::ImGuiKey::$key, VK::$id as _);)+
				};
			}

			keys!(set_keys);
		}

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

fn handle_imgui_events(imgui_state: &mut ImguiState, events: EventIterator<Event>, dpi: f32) {
	let imgui = &mut imgui_state.imgui;
	let mouse_state = &mut imgui_state.mouse_state;

	for event in events {
		if let Event::WindowEvent { event: e, .. } = event {
			match e {
				WindowEvent::KeyboardInput { input, .. } => {
					let pressed = input.state == ElementState::Pressed;

					macro_rules! match_keys {
						($($key:ident => $id:ident),+$(,)*) => {
							match input.virtual_keycode {
								$(Some(VK::$id) => imgui.set_key(VK::$id as _, pressed),)+
								_ => {},
							}
						};
					}

					keys!(match_keys);
				},
				WindowEvent::CursorMoved { position: pos, .. } => {
					mouse_state.pos = (pos.x as i32, pos.y as i32);
				},
				WindowEvent::MouseInput { state, button, .. } => match button {
					MouseButton::Left => mouse_state.pressed.0 = *state == ElementState::Pressed,
					MouseButton::Right => mouse_state.pressed.1 = *state == ElementState::Pressed,
					MouseButton::Middle => mouse_state.pressed.2 = *state == ElementState::Pressed,
					_ => {},
				},
				WindowEvent::MouseWheel {
					delta,
					phase: TouchPhase::Moved,
					..
				} => match delta {
					MouseScrollDelta::LineDelta(_, y) => mouse_state.wheel = *y,
					MouseScrollDelta::PixelDelta(lp) => mouse_state.wheel = lp.y as f32,
				},
				WindowEvent::ReceivedCharacter(c) => imgui.add_input_character(*c),
				_ => (),
			}
		}
	}

	imgui.set_mouse_pos(mouse_state.pos.0 as f32 * dpi, mouse_state.pos.1 as f32 * dpi);
	imgui.set_mouse_down([mouse_state.pressed.0, mouse_state.pressed.1, mouse_state.pressed.2, false, false]);
	imgui.set_mouse_wheel(mouse_state.wheel);
	mouse_state.wheel = 0.0;
}

pub fn with(f: impl Fn(&imgui::Ui)) {
	unsafe {
		if let Some(ui) = imgui::Ui::current_ui() {
			(ui as *const imgui::Ui<'_>).read_volatile();
			f(ui);
		}
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
	);

	fn setup(&mut self, res: &mut amethyst::ecs::Resources) {
		Self::SystemData::setup(res);
		self.reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
	}

	fn run(&mut self, (events, dimensions, time, mut imgui_state): Self::SystemData) {
		let dimensions: &amethyst::renderer::ScreenDimensions = &dimensions;
		let time: &amethyst::core::timing::Time = &time;

		if dimensions.width() <= 0. || dimensions.height() <= 0. {
			return;
		}

		let imgui_state = if let Some(x) = &mut imgui_state as &mut Option<ImguiState> {
			x
		} else {
			return;
		};
		handle_imgui_events(imgui_state, events.read(self.reader.as_mut().unwrap()), 1.);

		let frame = imgui_state.imgui.frame(
			imgui::FrameSize::new(
				f64::from(dimensions.width()),
				f64::from(dimensions.height()),
				dimensions.hidpi_factor(),
			),
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
