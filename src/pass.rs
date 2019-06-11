use amethyst::{
    core::{
        ecs::{Read, ReadExpect, WriteExpect, ReadStorage, Resources, SystemData},
        transform::Transform,
        Hidden, HiddenPropagate,
        math::{Vector2, Vector4},
    },
    assets::AssetStorage,
    renderer::{
        batch::{GroupIterator, OneLevelBatch, OrderedOneLevelBatch},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        submodules::{DynamicIndexBuffer, DynamicVertexBuffer, FlatEnvironmentSub, TextureId, TextureSub},
        transparent::Transparent,
        types::{Texture, Backend},
        util,
        rendy::{
            mesh::{AsAttribute, AsVertex, TexCoord, VertexFormat, Color},
            shader::{SpirvShader,},
            command::{QueueId, RenderPassEncoder},
            factory::Factory,
            graph::{
                render::{PrepareResult, RenderGroup, RenderGroupDesc, SetLayout},
                GraphContext, NodeBuffer, NodeImage,
            },
            resource::{DescriptorSetLayout, DescriptorSet},
            hal::{self, device::Device, pso},
        },
        mtl::TexAlbedo,
        skinning::JointCombined,
    }
};
use rendy::shader::{ShaderKind, SourceLanguage, StaticShaderInfo, ShaderSetBuilder, SpirvReflection};
use derivative::Derivative;

use imgui::ImDrawVert;

use rendy::shader::Shader;

lazy_static::lazy_static! {
	static ref VERTEX_SRC: rendy::shader::SpirvShader = StaticShaderInfo::new(
		concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/imgui.vert"),
		ShaderKind::Vertex,
		SourceLanguage::GLSL,
		"main",
	).precompile().unwrap();

    static ref VERTEX: SpirvShader = SpirvShader::new(
        (*VERTEX_SRC).spirv().unwrap().to_vec(),
        (*VERTEX_SRC).stage(),
        "main",
    );

	static ref FRAGMENT_SRC: rendy::shader::SpirvShader = StaticShaderInfo::new(
		concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/imgui.frag"),
		ShaderKind::Fragment,
		SourceLanguage::GLSL,
		"main",
	).precompile().unwrap();

	static ref FRAGMENT: SpirvShader = SpirvShader::new(
        (*FRAGMENT_SRC).spirv().unwrap().to_vec(),
        (*FRAGMENT_SRC).stage(),
        "main",
    );

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
    fn from(from: T) -> Self {
        ImguiColor(from.into())
    }
}
impl AsAttribute for ImguiColor {
    const NAME: &'static str = "color";
    const FORMAT: hal::format::Format = hal::format::Format::Rgba32Uint;
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

    pub fn raw(&self) -> &[f32] {
        &self.inner.data
    }

    pub fn scale(&self) -> Vector2<f32> {
        Vector2::new(self.inner.x, self.inner.y)
    }

    pub fn translation(&self) -> Vector2<f32> {
        Vector2::new(self.inner.z, self.inner.w)
    }

    pub fn set_scale(&mut self, scale: Vector2<f32>,) {
        self.inner.x = scale.x;
        self.inner.y = scale.y;
    }
    pub fn set_translation(&mut self, translation: Vector2<f32>,) {
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
    fn vertex() -> VertexFormat {
        VertexFormat::new((TexCoord::vertex(), TexCoord::vertex(), Color::vertex()))
    }
}

impl From<ImDrawVert> for ImguiArgs {
    fn from(v: ImDrawVert) -> Self {
        ImguiArgs {
            position: [v.pos.x, v.pos.y].into(),
            tex_coord: [v.uv.x, v.uv.y].into(),
            color: normalize(v.col).into(),
        }
    }
}

#[inline(always)]
pub fn normalize(src: u32) -> [f32; 4] {
    [
        ((src >> 0) & 0xff) as f32 / 255.0,
        ((src >> 8) & 0xff) as f32 / 255.0,
        ((src >> 16) & 0xff) as f32 / 255.0,
        ((src >> 24) & 0xff) as f32 / 255.0,

    ]
}

/// Draw opaque sprites without lighting.
#[derive(Clone, Debug, PartialEq, Derivative)]
#[derivative(Default(bound = ""))]
pub struct DrawImguiDesc;

impl DrawImguiDesc {
    /// Create instance of `DrawImgui` render group
    pub fn new() -> Self {
        Default::default()
    }
}

impl<B: Backend> RenderGroupDesc<B, Resources> for DrawImguiDesc {
    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        _queue: QueueId,
        _aux: &Resources,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, Resources>>, failure::Error> {
        let env = FlatEnvironmentSub::new(factory)?;
        let textures = TextureSub::new(factory)?;
        let vertex = DynamicVertexBuffer::new();
        let index = DynamicIndexBuffer::new();

        let (pipeline, pipeline_layout) = build_imgui_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![textures.raw_layout()],
        )?;

        Ok(Box::new(DrawImgui::<B> {
            pipeline: pipeline,
            pipeline_layout,
            vertex,
            index,
            textures,
            constant: ImguiPushConstant::default(),
            commands: Vec::new(),
            batches: Default::default(),
        }))
    }
}

#[derive(Debug)]
struct DrawCmd {
    vertex_range: std::ops::Range<u32>,
    index_range: std::ops::Range<u32>,
    scissor: hal::pso::Rect,
    texture_id: TextureId,
}

#[derive(Debug)]
pub struct DrawImgui<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    vertex: DynamicVertexBuffer<B, ImguiArgs>,
    index: DynamicIndexBuffer<B, u16>,
    batches: OrderedOneLevelBatch<TextureId, ImguiArgs>,
    textures: TextureSub<B>,
    commands: Vec<DrawCmd>,
    constant: ImguiPushConstant,
}

impl<B: Backend> DrawImgui<B> {

}

impl<B: Backend> RenderGroup<B, Resources> for DrawImgui<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        resources: &Resources,
    ) -> PrepareResult {
        let (
            state,
        ) = <(
            ReadExpect<'_, crate::ImguiState>,
        )>::fetch(resources);

        for texture in &state.textures {
            self.textures.insert(factory, resources, &texture, hal::image::Layout::ShaderReadOnlyOptimal,);
        }

        if let Some(ui) = unsafe { imgui::Ui::current_ui() } {
            let ui = ui as *const imgui::Ui;
            let ui = unsafe { ui.read() };

            self.constant.set_scale(Vector2::new(2.0 / ui.imgui().display_size().0, 2.0 / ui.imgui().display_size().1));
            self.constant.set_translation(Vector2::new(-1.0, -1.0));

            let vertices = ui.render(|ui, mut draw_data| {
                draw_data.scale_clip_rects(ui.imgui().display_framebuffer_scale());

                let mut vertices: Vec<ImguiArgs> = Vec::with_capacity(draw_data.total_vtx_count());
                let mut indices: Vec<u16> = Vec::with_capacity(draw_data.total_idx_count());

                self.commands.reserve(draw_data.draw_list_count());

                for draw_list in &draw_data {
                    for draw_cmd in draw_list.cmd_buffer.iter() {
                        self.commands.push(DrawCmd {
                            vertex_range: std::ops::Range { start: vertices.len() as u32, end: (vertices.len() + draw_list.vtx_buffer.len()) as u32 },
                            index_range: std::ops::Range { start: indices.len() as u32, end: (indices.len() + draw_list.idx_buffer.len()) as u32 },
                            scissor: hal::pso::Rect {
                                x: draw_cmd.clip_rect.x as i16,
                                y: draw_cmd.clip_rect.y as i16,
                                w: (draw_cmd.clip_rect.z - draw_cmd.clip_rect.x) as i16,
                                h: (draw_cmd.clip_rect.w - draw_cmd.clip_rect.y) as i16,
                            },
                            texture_id: unsafe { std::mem::transmute::<u32, TextureId>(draw_cmd.texture_id as u32) },
                        });
                    }
                    vertices.extend(draw_list.vtx_buffer.iter().map(|v| (*v).into()).collect::<Vec<ImguiArgs>>());
                    indices.extend(draw_list.idx_buffer.iter().map(|v| (*v).into()).collect::<Vec<u16>>());
                }

                self.vertex.write(factory, index, vertices.len() as u64, &[vertices.iter()]);
                self.index.write(factory, index, indices.len() as u64, &[indices.iter()]);

                self.textures.maintain(factory, resources);

                if false == true {
                    //  This is a fucking stupid type inference issue
                    return Err(failure::format_err!("WTF"));
                }
                Ok(())
            });
        }

        PrepareResult::DrawRecord
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        resources: &Resources,
    ) {
        let (
            state,
            tex_storage,
        ) = <(
            ReadExpect<'_, crate::ImguiState>,
            Read<'_, AssetStorage<Texture>>,
        )>::fetch(resources);

        let layout = &self.pipeline_layout;


        for draw in &self.commands {
            encoder.bind_graphics_pipeline(&self.pipeline);

            encoder.set_scissors(0, &[draw.scissor]);

            encoder.push_constants(layout, pso::ShaderStageFlags::VERTEX, 0, hal::memory::cast_slice::<f32, u32>(self.constant.raw()));

            self.vertex.bind(index, 0, 0, &mut encoder);
            self.index.bind(index, 0, &mut encoder);

            if self.textures.loaded(draw.texture_id) {
                self.textures.bind(layout, 0, draw.texture_id, &mut encoder);
            }
            encoder.draw_indexed(draw.index_range.clone(), draw.vertex_range.start as i32, std::ops::Range{ start: 0, end: 1  });
        }


        self.commands.clear();
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _aux: &Resources) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
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

    use amethyst::renderer::rendy::shader::Shader;

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
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
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
                .with_blend_targets(vec![pso::ColorBlendDesc(
                    pso::ColorMask::ALL,
                    pso::BlendState::ALPHA,
                )])
                .with_depth_test(pso::DepthTest::On {
                    fun: pso::Comparison::Less,
                    write: false,
                }),
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
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}
