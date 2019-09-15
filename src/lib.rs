#![allow(clippy::type_complexity, dead_code)]

mod pass;

pub use imgui;
pub use pass::DrawImguiDesc;

use amethyst::{
	core::SystemDesc,
	derive::SystemDesc,
	ecs::{DispatcherBuilder, Read, ReadExpect, System, SystemData, World, Write, WriteExpect},
	error::Error,
	input::{BindingTypes, InputEvent},
	renderer::{
		bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
		rendy::{factory::Factory, graph::render::RenderGroupDesc},
		types::Backend,
	},
	shrev::{EventChannel, ReaderId},
	window::Window,
	winit::Event,
};
use derivative::Derivative;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::sync::{Arc, Mutex};

pub struct ImguiContextWrapper(pub imgui::Context);
unsafe impl Send for ImguiContextWrapper {}

pub struct FilteredInputEvent<T: BindingTypes>(pub InputEvent<T>);

pub struct ImguiInputSystem<T: BindingTypes> {
	input_reader: ReaderId<InputEvent<T>>,
	winit_reader: ReaderId<Event>,
}
impl<'s, T: BindingTypes> System<'s> for ImguiInputSystem<T> {
	type SystemData = (
		ReadExpect<'s, Arc<Mutex<ImguiContextWrapper>>>,
		WriteExpect<'s, WinitPlatform>,
		Read<'s, EventChannel<InputEvent<T>>>,
		Read<'s, EventChannel<Event>>,
		Write<'s, EventChannel<FilteredInputEvent<T>>>,
		ReadExpect<'s, Window>,
	);

	fn run(&mut self, (context, mut platform, input_events, winit_events, mut filtered_events, window): Self::SystemData) {
		let state = &mut context.lock().unwrap().0;

		for event in winit_events.read(&mut self.winit_reader) {
			//platform.handle_event(state.io_mut(), &window, &event);
		}
		for input in input_events.read(&mut self.input_reader) {
			match input {
				InputEvent::MouseMoved { .. } |
				InputEvent::MouseButtonPressed(_) |
				InputEvent::MouseButtonReleased(_) |
				InputEvent::MouseWheelMoved(_) => {
					if !state.io().want_capture_mouse {
						filtered_events.single_write(FilteredInputEvent(input.clone()));
					}
				},
				InputEvent::KeyPressed { .. } | InputEvent::KeyReleased { .. } => {
					if !state.io().want_capture_keyboard {
						filtered_events.single_write(FilteredInputEvent(input.clone()));
					}
				},
				_ => filtered_events.single_write(FilteredInputEvent(input.clone())),
			}
		}
	}
}

pub struct ImguiInputSystemDesc<T: BindingTypes> {
	_marker: std::marker::PhantomData<T>,
	config_flags: imgui::ConfigFlags,
}
impl<T: BindingTypes> ImguiInputSystemDesc<T> {
	pub fn new(config_flags: imgui::ConfigFlags) -> Self {
		Self {
			_marker: Default::default(),
			config_flags,
		}
	}
}

impl<'a, 'b, T: BindingTypes> SystemDesc<'a, 'b, ImguiInputSystem<T>> for ImguiInputSystemDesc<T> {
	fn build(self, world: &mut World) -> ImguiInputSystem<T> {
		<ImguiInputSystem<T> as System<'_>>::SystemData::setup(world);

		let input_reader = Write::<EventChannel<InputEvent<T>>>::fetch(world).register_reader();
		let winit_reader = Write::<EventChannel<Event>>::fetch(world).register_reader();

		// Setup Imgui
		let mut context = imgui::Context::create();

		context.fonts().add_font(&[imgui::FontSource::DefaultFontData {
			config: Some(imgui::FontConfig {
				size_pixels: 13.,
				..imgui::FontConfig::default()
			}),
		}]);

		context.io_mut().config_flags |= self.config_flags;

		let mut platform = WinitPlatform::init(&mut context);
		platform.attach_window(context.io_mut(), &world.fetch::<Window>(), HiDpiMode::Default);

		world.insert(Arc::new(Mutex::new(ImguiContextWrapper(context))));
		world.insert(platform);

		ImguiInputSystem {
			input_reader,
			winit_reader,
		}
	}
}

/// Ui is actually Ui<'a>
/// This implies 'static
static mut CURRENT_UI: Option<imgui::Ui<'static>> = None;

pub fn with(f: impl FnOnce(&imgui::Ui)) {
	unsafe {
		if let Some(ui) = current_ui() {
			(f)(ui);
		}
	}
}

// what lifeimtes go here? how do I use transmute here?
pub unsafe fn current_ui<'a>() -> Option<&'a imgui::Ui<'a>> { CURRENT_UI.as_ref() }

/// A [RenderPlugin] for rendering Imgui elements.
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub struct RenderImgui<T: BindingTypes> {
	target: Target,
	config_flags: imgui::ConfigFlags,
	_marker: std::marker::PhantomData<T>,
}
impl<T: BindingTypes> Default for RenderImgui<T> {
	fn default() -> Self {
		Self {
			target: Default::default(),
			_marker: Default::default(),
			config_flags: imgui::ConfigFlags::ENABLE_DOCKING,
		}
	}
}

impl<T: BindingTypes> RenderImgui<T> {
	pub fn with_imgui_config(mut self, config_flags: imgui::ConfigFlags) -> Self {
		self.config_flags = config_flags;
		self
	}

	/// Select render target on which UI should be rendered.
	pub fn with_target(mut self, target: Target) -> Self {
		self.target = target;
		self
	}
}

impl<B: Backend, T: BindingTypes> RenderPlugin<B> for RenderImgui<T> {
	fn on_build<'a, 'b>(&mut self, world: &mut World, dispatcher: &mut DispatcherBuilder<'a, 'b>) -> Result<(), Error> {
		dispatcher.add(
			ImguiInputSystemDesc::<T>::new(self.config_flags).build(world),
			"imgui_input_system",
			&["input_system", "window"],
		);

		Ok(())
	}

	fn on_plan(&mut self, plan: &mut RenderPlan<B>, _factory: &mut Factory<B>, _: &World) -> Result<(), Error> {
		plan.extend_target(self.target, |ctx| {
			ctx.add(RenderOrder::Overlay, DrawImguiDesc::new().builder())?;
			Ok(())
		});
		Ok(())
	}
}
