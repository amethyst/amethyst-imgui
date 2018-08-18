pub extern crate imgui;
extern crate amethyst;
extern crate gfx;
#[macro_use] extern crate glsl_layout;
extern crate imgui_gfx_renderer;
extern crate shred;

use amethyst::{
	core::{cgmath},
	renderer::{
		error::Result,
		pipe::{
			pass::{Pass, PassData},
			Effect,
			NewEffect,
		},
		Encoder,
		Mesh,
		PosTex,
		Resources,
		VertexFormat,
	},
};
use gfx::{memory::Typed, preset::blend, pso::buffer::ElemStride, state::ColorMask};
use gfx::traits::Factory;
use glsl_layout::{vec2, vec4, Uniform};
use imgui::{FontGlyphRange, FrameSize, ImFontConfig, ImGui, ImVec4};
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

pub struct DrawUi<S>
where
	S: for<'pd> shred::SystemData<'pd>,
{
	imgui: Option<ImGui>,
	renderer: Option<RendererThing>,
	run_ui: fn(&mut imgui::Ui, &S),
}

impl<S> DrawUi<S>
where
	S: for<'pd> shred::SystemData<'pd>,
{
	pub fn new(run_ui: fn(&mut imgui::Ui, &S)) -> Self {
		Self {
			imgui: None,
			renderer: None,
			run_ui,
		}
	}
}

pub struct ImguiState {
	imgui: ImGui,
	mouse_state: MouseState,
	size: (u16, u16),
}

type FormattedT = (gfx::format::R8_G8_B8_A8, gfx::format::Unorm);

impl<'pd4, S> PassData<'pd4> for DrawUi<S>
where
	S: for <'a> shred::SystemData<'a> + Send,
{
	type Data = (
		shred::ReadExpect<'pd4, amethyst::renderer::ScreenDimensions>,
		shred::Read<'pd4, amethyst::core::timing::Time>,
		shred::Write<'pd4, Option<ImguiState>>,
		S,
	);
}

impl<S> Pass for DrawUi<S>
where
	S: for<'pd5> shred::SystemData<'pd5> + Send,
{
	fn compile(&mut self, mut effect: NewEffect<'_>) -> Result<Effect> {
		let mut imgui = ImGui::init();
		{
			// Fix incorrect colors with sRGB framebuffer
			fn imgui_gamma_to_linear(col: ImVec4) -> ImVec4 {
				let x = col.x.powf(2.2);
				let y = col.y.powf(2.2);
				let z = col.z.powf(2.2);
				let w = 1.0 - (1.0 - col.w).powf(2.2);
				ImVec4::new(x, y, z, w)
			}

			let style = imgui.style_mut();
			for col in 0..style.colors.len() {
				style.colors[col] = imgui_gamma_to_linear(style.colors[col]);
			}
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

		{
			macro_rules! set_keys {
				($($key:ident => $id:expr),+$(,)*) => {
					$(imgui.set_imgui_key(imgui::ImGuiKey::$key, $id);)+
				};
			}

			set_keys![
				Tab => 0,
				LeftArrow => 1,
				RightArrow => 2,
				UpArrow => 3,
				DownArrow => 4,
				PageUp => 5,
				PageDown => 6,
				Home => 7,
				End => 8,
				Delete => 9,
				Backspace => 10,
				Enter => 11,
				Escape => 12,
				A => 13,
				C => 14,
				V => 15,
				X => 16,
				Y => 17,
				Z => 18,
			];
		}

		let data = vec![
			PosTex {
				position: [0., 1., 0.],
				tex_coord: [0., 0.],
			},
			PosTex {
				position: [1., 1., 0.],
				tex_coord: [1., 0.],
			},
			PosTex {
				position: [1., 0., 0.],
				tex_coord: [1., 1.],
			},
			PosTex {
				position: [0., 1., 0.],
				tex_coord: [0., 0.],
			},
			PosTex {
				position: [1., 0., 0.],
				tex_coord: [1., 1.],
			},
			PosTex {
				position: [0., 0., 0.],
				tex_coord: [0., 1.],
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
		(screen_dimensions, time, mut imgui_state, ui_data): <Self as PassData<'apply_pd>>::Data,
	) {
		let imgui_state = imgui_state.get_or_insert_with(|| ImguiState {
			imgui: self.imgui.take().unwrap(),
			mouse_state: MouseState::default(),
			size: (1024, 1024),
		});
		let imgui = &mut imgui_state.imgui;

		let (width, height) = (screen_dimensions.width(), screen_dimensions.height());
		let renderer_thing = self.renderer.as_mut().unwrap();

		let vertex_args = VertexArgs {
			proj_vec: cgmath::vec4(2. / width, -2. / height, 0., 1.).into(),
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
		{
			let mut ui = imgui.frame(FrameSize::new(f64::from(width), f64::from(height), 1.), time.delta_seconds());
			(self.run_ui)(&mut ui, &ui_data);
			renderer_thing.renderer.render(ui, &mut factory, encoder).unwrap();
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
struct MouseState {
	pos: (i32, i32),
	pressed: (bool, bool, bool),
	wheel: f32,
}

pub fn handle_imgui_events(imgui_state: &mut ImguiState, event: &amethyst::renderer::Event) {
	use amethyst::{
		renderer::{
			ElementState,
			Event,
			MouseButton,
			VirtualKeyCode as VK,
			WindowEvent::{self, ReceivedCharacter},
		},
		winit::{MouseScrollDelta, TouchPhase},
	};

	let imgui = &mut imgui_state.imgui;
	let mouse_state = &mut imgui_state.mouse_state;

	if let Event::WindowEvent { event, .. } = event {
		match event {
			WindowEvent::KeyboardInput { input, .. } => {
				let pressed = input.state == ElementState::Pressed;
				match input.virtual_keycode {
					Some(VK::Tab) => imgui.set_key(0, pressed),
					Some(VK::Left) => imgui.set_key(1, pressed),
					Some(VK::Right) => imgui.set_key(2, pressed),
					Some(VK::Up) => imgui.set_key(3, pressed),
					Some(VK::Down) => imgui.set_key(4, pressed),
					Some(VK::PageUp) => imgui.set_key(5, pressed),
					Some(VK::PageDown) => imgui.set_key(6, pressed),
					Some(VK::Home) => imgui.set_key(7, pressed),
					Some(VK::End) => imgui.set_key(8, pressed),
					Some(VK::Delete) => imgui.set_key(9, pressed),
					Some(VK::Back) => imgui.set_key(10, pressed),
					Some(VK::Return) => imgui.set_key(11, pressed),
					Some(VK::Escape) => imgui.set_key(12, pressed),
					Some(VK::A) => imgui.set_key(13, pressed),
					Some(VK::C) => imgui.set_key(14, pressed),
					Some(VK::V) => imgui.set_key(15, pressed),
					Some(VK::X) => imgui.set_key(16, pressed),
					Some(VK::Y) => imgui.set_key(17, pressed),
					Some(VK::Z) => imgui.set_key(18, pressed),
					Some(VK::LControl) | Some(VK::RControl) => imgui.set_key_ctrl(pressed),
					Some(VK::LShift) | Some(VK::RShift) => imgui.set_key_shift(pressed),
					Some(VK::LAlt) | Some(VK::RAlt) => imgui.set_key_alt(pressed),
					Some(VK::LWin) | Some(VK::RWin) => imgui.set_key_super(pressed),
					_ => {},
				}
			},
			WindowEvent::CursorMoved { position: pos, .. } => {
				mouse_state.pos = (pos.0 as i32, pos.1 as i32);
			},
			WindowEvent::MouseInput { state, button, .. } => match button {
				MouseButton::Left => mouse_state.pressed.0 = *state == ElementState::Pressed,
				MouseButton::Right => mouse_state.pressed.1 = *state == ElementState::Pressed,
				MouseButton::Middle => mouse_state.pressed.2 = *state == ElementState::Pressed,
				_ => {},
			},
			WindowEvent::MouseWheel {
				delta: MouseScrollDelta::LineDelta(_, y),
				phase: TouchPhase::Moved,
				..
			} | WindowEvent::MouseWheel {
				delta: MouseScrollDelta::PixelDelta(_, y),
				phase: TouchPhase::Moved,
				..
			} => mouse_state.wheel = *y,
			ReceivedCharacter(c) => imgui.add_input_character(*c),
			_ => (),
		}
	}

	imgui.set_mouse_pos(mouse_state.pos.0 as f32, mouse_state.pos.1 as f32);
	imgui.set_mouse_down([mouse_state.pressed.0, mouse_state.pressed.1, mouse_state.pressed.2, false, false]);
	imgui.set_mouse_wheel(mouse_state.wheel);
	mouse_state.wheel = 0.0;
}
