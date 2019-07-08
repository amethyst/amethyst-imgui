#![feature(custom_attribute)]
#![allow(clippy::type_complexity, dead_code)]

mod pass;

pub use imgui;
pub use pass::DrawImguiDesc;

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
