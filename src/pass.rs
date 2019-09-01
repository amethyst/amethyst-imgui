use amethyst::{
	assets::{AssetStorage, Handle, Loader},
	core::{
		ecs::{Read, ReadExpect, SystemData, World, Write},
		math::{Vector2, Vector4},
	},
	renderer::{
		batch::OrderedOneLevelBatch,
		pipeline::{PipelineDescBuilder, PipelinesBuilder},
		rendy::{
			command::{QueueId, RenderPassEncoder},
			factory::Factory,
			graph::{
				render::{PrepareResult, RenderGroup, RenderGroupDesc},
				GraphContext,
				NodeBuffer,
				NodeImage,
			},
			hal::{
				self,
				device::Device,
				format::Format,
				image::{self, Anisotropic, Filter, PackedColor, SamplerInfo, WrapMode},
				pso,
			},
			mesh::{AsAttribute, AsVertex, Color, TexCoord, VertexFormat},
			shader::{PathBufShaderInfo, Shader, ShaderKind, SourceLanguage, SpirvShader},
			texture::TextureBuilder,
		},
		submodules::{DynamicIndexBuffer, DynamicVertexBuffer, TextureId, TextureSub},
		types::{Backend, TextureData},
		util,
		Texture,
	},
	shrev::{EventChannel, ReaderId},
	window::Window,
	winit::Event,
};
use derivative::Derivative;
use imgui::{internal::RawWrapper, DrawCmd, DrawCmdParams};
use std::{
	borrow::Cow,
	path::PathBuf,
	sync::{Arc, Mutex},
};

use imgui::{FontConfig, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};

pub struct ImguiContextWrapper(imgui::Context);
unsafe impl Send for ImguiContextWrapper {}

lazy_static::lazy_static! {
	static ref VERTEX_SRC: SpirvShader = PathBufShaderInfo::new(
		PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/imgui.vert")),
		ShaderKind::Vertex,
		SourceLanguage::GLSL,
		"main",
	).precompile().unwrap();

	static ref VERTEX: SpirvShader = SpirvShader::new(
		(*VERTEX_SRC).spirv().unwrap().to_vec(),
		(*VERTEX_SRC).stage(),
		"main",
	);

	static ref FRAGMENT_SRC: SpirvShader = PathBufShaderInfo::new(
		PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/imgui.frag")),
		ShaderKind::Fragment,
		SourceLanguage::GLSL,
		"main",
	).precompile().unwrap();

	static ref FRAGMENT: SpirvShader = SpirvShader::new(
		(*FRAGMENT_SRC).spirv().unwrap().to_vec(),
		(*FRAGMENT_SRC).stage(),
		"main",
	);

	static ref TIME: std::sync::Mutex<std::time::Instant> = std::sync::Mutex::new(std::time::Instant::now());

//static ref SHADERS: ShaderSetBuilder = ShaderSetBuilder::default()
//		.with_vertex(&*VERTEX).unwrap()
//		.with_fragment(&*FRAGMENT).unwrap();
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ImguiColor(pub u32);
impl<T> From<T> for ImguiColor
where
	T: Into<u32>,
{
	fn from(from: T) -> Self { ImguiColor(from.into()) }
}
impl AsAttribute for ImguiColor {
	const FORMAT: hal::format::Format = hal::format::Format::Rgba32Uint;
	const NAME: &'static str = "color";
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ImguiPushConstant {
	inner: Vector4<f32>,
}
impl ImguiPushConstant {
	pub fn new(scale_x: f32, scale_y: f32, trans_x: f32, trans_y: f32) -> Self {
		Self {
			inner: Vector4::new(scale_x, scale_y, trans_x, trans_y),
		}
	}

	pub fn raw(&self) -> &[f32] { &self.inner.data }

	pub fn scale(&self) -> Vector2<f32> { Vector2::new(self.inner.x, self.inner.y) }

	pub fn translation(&self) -> Vector2<f32> { Vector2::new(self.inner.z, self.inner.w) }

	pub fn set_scale(&mut self, scale: Vector2<f32>) {
		self.inner.x = scale.x;
		self.inner.y = scale.y;
	}

	pub fn set_translation(&mut self, translation: Vector2<f32>) {
		self.inner.z = translation.x;
		self.inner.w = translation.y;
	}
}
impl Default for ImguiPushConstant {
	fn default() -> Self {
		Self {
			inner: Vector4::new(1.0, 1.0, 0.0, 0.0),
		}
	}
}

/// Vertex format with position and UV texture coordinate attributes.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct ImguiArgs {
	/// Position of the vertex in 2D space.
	pub position: TexCoord,
	/// UV texture coordinates used by the vertex.
	pub tex_coord: TexCoord,
	pub color: Color,
}

impl AsVertex for ImguiArgs {
	fn vertex() -> VertexFormat { VertexFormat::new((TexCoord::vertex(), TexCoord::vertex(), Color::vertex())) }
}

impl From<imgui::DrawVert> for ImguiArgs {
	fn from(other: imgui::DrawVert) -> Self {
		Self {
			position: other.pos.into(),
			tex_coord: other.uv.into(),
			color: normalize(other.col).into(),
		}
	}
}

#[inline(always)]
pub fn normalize(src: [u8; 4]) -> [f32; 4] {
	[
		f32::from(src[0]) / 255.0,
		f32::from(src[1]) / 255.0,
		f32::from(src[2]) / 255.0,
		f32::from(src[3]) / 255.0,
	]
}

/// Draw opaque sprites without lighting.
#[derive(Clone, Debug, PartialEq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct DrawImguiDesc;

impl DrawImguiDesc {
	/// Create instance of `DrawImgui` render group
	pub fn new() -> Self { Default::default() }

	fn generate_upload_font_textures(&self, world: &World, mut fonts: imgui::FontAtlasRefMut) -> Vec<Handle<Texture>> {
		let tex = fonts.build_rgba32_texture();

		let loader = world.fetch_mut::<Loader>();
		let texture_storage = world.fetch_mut::<AssetStorage<Texture>>();
		let data = tex.data.to_vec();

		let texture_builder = TextureBuilder::new()
			.with_data_width(tex.width)
			.with_data_height(tex.height)
			.with_kind(image::Kind::D2(tex.width, tex.height, 1, 1))
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
			.with_raw_data(Cow::Owned(data), Format::Rgba8Unorm);

		vec![loader.load_from_data(TextureData(texture_builder), (), &texture_storage)]
	}
}

impl<B: Backend> RenderGroupDesc<B, World> for DrawImguiDesc {
	fn build(
		self,
		_ctx: &GraphContext<B>,
		factory: &mut Factory<B>,
		_queue: QueueId,
		world: &World,
		framebuffer_width: u32,
		framebuffer_height: u32,
		subpass: hal::pass::Subpass<'_, B>,
		_buffers: Vec<NodeBuffer>,
		_images: Vec<NodeImage>,
	) -> Result<Box<dyn RenderGroup<B, World>>, failure::Error> {
		let (window, mut events) = <(ReadExpect<'_, Window>, Write<'_, EventChannel<Event>>)>::fetch(world);

		let textures = TextureSub::new(factory)?;
		let vertex = DynamicVertexBuffer::new();
		let index = DynamicIndexBuffer::new();

		let (pipeline, pipeline_layout) =
			build_imgui_pipeline(factory, subpass, framebuffer_width, framebuffer_height, vec![textures.raw_layout()])?;

		// Initialize everything
		let mut context = imgui::Context::create();
		let mut platform = WinitPlatform::init(&mut context);
		platform.attach_window(context.io_mut(), &window, HiDpiMode::Default);

		//imgui.set_ini_filename(config.ini.as_ref().map(|i| imgui::ImString::new(i)));
		context.fonts().add_font(&[FontSource::DefaultFontData {
			config: Some(FontConfig {
				size_pixels: 13.,
				..FontConfig::default()
			}),
		}]);

		let imgui_textures = self.generate_upload_font_textures(&world, context.fonts());

		unsafe { crate::CURRENT_UI = Some(std::mem::transmute(context.frame())) }

		Ok(Box::new(DrawImgui::<B> {
			pipeline,
			pipeline_layout,
			vertex,
			index,
			textures,
			constant: ImguiPushConstant::default(),
			commands: Vec::new(),
			batches: Default::default(),
			event_reader_id: events.register_reader(),
			context: Arc::new(Mutex::new(ImguiContextWrapper(context))),
			platform,
			imgui_textures,
		}))
	}
}

#[derive(Debug)]
struct DrawCmdOps {
	vertex_range: std::ops::Range<u32>,
	index_range: std::ops::Range<u32>,
	scissor: hal::pso::Rect,
	texture_id: TextureId,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DrawImgui<B: Backend> {
	pipeline: B::GraphicsPipeline,
	pipeline_layout: B::PipelineLayout,
	vertex: DynamicVertexBuffer<B, ImguiArgs>,
	index: DynamicIndexBuffer<B, u16>,
	batches: OrderedOneLevelBatch<TextureId, ImguiArgs>,
	textures: TextureSub<B>,
	commands: Vec<DrawCmdOps>,
	constant: ImguiPushConstant,

	event_reader_id: ReaderId<Event>,
	#[derivative(Debug = "ignore")]
	context: Arc<Mutex<ImguiContextWrapper>>,
	platform: WinitPlatform,
	imgui_textures: Vec<Handle<Texture>>,
}

impl<B: Backend> DrawImgui<B> {}

impl<B: Backend> RenderGroup<B, World> for DrawImgui<B> {
	#[allow(clippy::identity_conversion)]
	fn prepare(
		&mut self,
		factory: &Factory<B>,
		_queue: QueueId,
		index: usize,
		_subpass: hal::pass::Subpass<'_, B>,
		world: &World,
	) -> PrepareResult {
		let (window, events) = <(ReadExpect<'_, Window>, Read<'_, EventChannel<Event>>)>::fetch(world);

		let state = &mut self.context.lock().unwrap().0;

		for texture in &self.imgui_textures {
			self.textures
				.insert(factory, world, &texture, hal::image::Layout::ShaderReadOnlyOptimal);
		}
		self.constant
			.set_scale(Vector2::new(2.0 / state.io().display_size[0], 2.0 / state.io().display_size[1]));
		self.constant.set_translation(Vector2::new(-1.0, -1.0));

		let last_frame = &mut TIME.lock().unwrap() as &mut std::time::Instant;
		std::mem::replace(last_frame, state.io_mut().update_delta_time(*last_frame));
		//		state.io_mut().font_global_scale = (1.0 / dimensions.hidpi_factor()) as f32;
		//		state.io_mut().display_size = [dimensions.width(), dimensions.height()];
		unsafe {
			if let Some(ui) = crate::current_ui() {
				self.platform.prepare_render(ui, &window);
			}
		}

		{
			let draw_data = unsafe {
				crate::CURRENT_UI = None;
				imgui::sys::igRender();
				&*(imgui::sys::igGetDrawData() as *mut imgui::DrawData)
			};

			let mut vertices: Vec<ImguiArgs> = Vec::with_capacity(draw_data.total_vtx_count as usize);
			let mut indices: Vec<u16> = Vec::with_capacity(draw_data.total_idx_count as usize);

			self.commands.reserve(draw_data.draw_lists().count());

			for draw_list in draw_data.draw_lists() {
				for draw_cmd in draw_list.commands() {
					match draw_cmd {
						DrawCmd::Elements {
							cmd_params: DrawCmdParams { clip_rect, texture_id, .. },
							..
						} => {
							self.commands.push(DrawCmdOps {
								vertex_range: std::ops::Range {
									start: vertices.len() as u32,
									end: (vertices.len() + draw_list.vtx_buffer().len()) as u32,
								},
								index_range: std::ops::Range {
									start: indices.len() as u32,
									end: (indices.len() + draw_list.idx_buffer().len()) as u32,
								},
								scissor: hal::pso::Rect {
									x: ((clip_rect[0] - draw_data.display_pos[0]) * draw_data.framebuffer_scale[0]) as i16,
									y: ((clip_rect[1] - draw_data.display_pos[1]) * draw_data.framebuffer_scale[1]) as i16,
									w: ((clip_rect[2] - clip_rect[0] - draw_data.display_pos[0]) * draw_data.framebuffer_scale[0]) as i16,
									h: ((clip_rect[3] - clip_rect[1] - draw_data.display_pos[1]) * draw_data.framebuffer_scale[1]) as i16,
								},
								texture_id: unsafe { std::mem::transmute::<u32, TextureId>(texture_id.id() as u32) },
							});
						},
						DrawCmd::ResetRenderState => (), // TODO
						DrawCmd::RawCallback { callback, raw_cmd } => unsafe { callback(draw_list.raw(), raw_cmd) },
					}
				}
				vertices.extend(draw_list.vtx_buffer().iter().map(|v| (*v).into()).collect::<Vec<ImguiArgs>>());
				indices.extend(draw_list.idx_buffer().iter().map(|v| (*v).into()).collect::<Vec<u16>>());
			}

			self.vertex.write(factory, index, vertices.len() as u64, &[vertices.iter()]);
			self.index.write(factory, index, indices.len() as u64, &[indices.iter()]);

			self.textures.maintain(factory, world);
		}

		for event in events.read(&mut self.event_reader_id) {
			self.platform.handle_event(state.io_mut(), &window, &event);
		}

		unsafe {
			crate::CURRENT_UI = Some(std::mem::transmute(state.frame()));
		}

		PrepareResult::DrawRecord
	}

	fn draw_inline(&mut self, mut encoder: RenderPassEncoder<'_, B>, index: usize, _: hal::pass::Subpass<'_, B>, _: &World) {
		let layout = &self.pipeline_layout;

		for draw in &self.commands {
			encoder.bind_graphics_pipeline(&self.pipeline);

			self.vertex.bind(index, 0, 0, &mut encoder);
			self.index.bind(index, 0, &mut encoder);

			if self.textures.loaded(draw.texture_id) {
				self.textures.bind(layout, 0, draw.texture_id, &mut encoder);
			}

			unsafe {
				encoder.set_scissors(0, &[draw.scissor]);

				encoder.push_constants(
					layout,
					pso::ShaderStageFlags::VERTEX,
					0,
					hal::memory::cast_slice::<f32, u32>(self.constant.raw()),
				);

				encoder.draw_indexed(
					draw.index_range.clone(),
					draw.vertex_range.start as i32,
					std::ops::Range { start: 0, end: 1 },
				);
			}
		}

		self.commands.clear();
	}

	fn dispose(self: Box<Self>, factory: &mut Factory<B>, _aux: &World) {
		unsafe {
			factory.device().destroy_graphics_pipeline(self.pipeline);
			factory.device().destroy_pipeline_layout(self.pipeline_layout);
		}
	}
}

fn build_imgui_pipeline<B: Backend>(
	factory: &Factory<B>,
	subpass: hal::pass::Subpass<'_, B>,
	framebuffer_width: u32,
	framebuffer_height: u32,
	layouts: Vec<&B::DescriptorSetLayout>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), failure::Error> {
	let pipeline_layout = unsafe {
		factory
			.device()
			.create_pipeline_layout(layouts, &[(pso::ShaderStageFlags::VERTEX, 0..16)])
	}?;

	let shader_vertex = unsafe { VERTEX.module(factory).unwrap() };
	let shader_fragment = unsafe { FRAGMENT.module(factory).unwrap() };

	let pipes = PipelinesBuilder::new()
		.with_pipeline(
			PipelineDescBuilder::new()
				.with_vertex_desc(&[(ImguiArgs::vertex(), pso::VertexInputRate::Vertex)])
				.with_input_assembler(pso::InputAssemblerDesc::new(hal::Primitive::TriangleList))
				.with_rasterizer(hal::pso::Rasterizer {
					polygon_mode: hal::pso::PolygonMode::Fill,
					cull_face: hal::pso::Face::NONE,
					front_face: hal::pso::FrontFace::Clockwise,
					depth_clamping: false,
					depth_bias: None,
					conservative: false,
				})
				.with_shaders(util::simple_shader_set(&shader_vertex, Some(&shader_fragment)))
				.with_layout(&pipeline_layout)
				.with_subpass(subpass)
				.with_baked_states(hal::pso::BakedStates {
					viewport: Some(hal::pso::Viewport {
						rect: hal::pso::Rect {
							x: 0,
							y: 0,
							w: framebuffer_width as i16,
							h: framebuffer_height as i16,
						},
						depth: 0.0..1.0,
					}),
					scissor: None,
					..Default::default()
				})
				.with_blend_targets(vec![pso::ColorBlendDesc(pso::ColorMask::ALL, pso::BlendState::ALPHA)])
				.with_depth_test(pso::DepthTest::Off),
		)
		.build(factory, None);

	unsafe {
		factory.destroy_shader_module(shader_vertex);
		factory.destroy_shader_module(shader_fragment);
	}

	match pipes {
		Err(e) => {
			unsafe {
				factory.device().destroy_pipeline_layout(pipeline_layout);
			}
			Err(e)
		},
		Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
	}
}
