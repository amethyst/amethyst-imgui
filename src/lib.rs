#![feature(custom_attribute)]
#![allow(clippy::type_complexity, dead_code)]

mod pass;

pub use imgui;
pub use pass::DrawImguiDesc;

#[derive(Default)]
pub struct ImguiDrawCommandBuffer {
	commands: Vec<fn(&mut imgui::Ui)>,
}

impl ImguiDrawCommandBuffer {
	pub fn draw(&mut self, f: fn(&mut imgui::Ui)) { self.commands.push(f); }

	pub(crate) fn iter(&self) -> impl Iterator<Item = &fn(&mut imgui::Ui)> { self.commands.iter() }

	pub(crate) fn clear(&mut self) { self.commands.clear(); }
}
