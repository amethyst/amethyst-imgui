#![feature(custom_attribute)]
#![allow(clippy::type_complexity, dead_code)]

mod pass;

pub use imgui;
pub use pass::DrawImguiDesc;

use amethyst::{
	ecs::{DispatcherBuilder, Resources},
	error::Error,
	renderer::{
		bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
		rendy::{factory::Factory, graph::render::RenderGroupDesc},
		types::Backend,
	},
};

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
#[derive(Debug, Default)]
pub struct RenderImgui {
	target: Target,
}

impl RenderImgui {
	/// Select render target on which UI should be rendered.
	pub fn with_target(mut self, target: Target) -> Self {
		self.target = target;
		self
	}
}

impl<B: Backend> RenderPlugin<B> for RenderImgui {
	fn on_build<'a, 'b>(&mut self, _: &mut DispatcherBuilder<'a, 'b>) -> Result<(), Error> { Ok(()) }

	fn on_plan(&mut self, plan: &mut RenderPlan<B>, _factory: &mut Factory<B>, _res: &Resources) -> Result<(), Error> {
		plan.extend_target(self.target, |ctx| {
			ctx.add(RenderOrder::Overlay, DrawImguiDesc::new().builder())?;
			Ok(())
		});
		Ok(())
	}
}
