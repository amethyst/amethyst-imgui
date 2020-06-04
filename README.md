[![Latest release on crates.io](https://meritbadge.herokuapp.com/amethyst-imgui)](https://crates.io/crates/amethyst-imgui)
[![Documentation on docs.rs](https://docs.rs/amethyst-imgui/badge.svg)](https://docs.rs/amethyst-imgui)

# amethyst-imgui

Amethyst-imgui provides integration for the [imgui-rs crate](https://github.com/Gekkio/imgui-rs) within the [Amethyst](https://amethyst.rs) game engine.

ImGUI is known industry wide for its utility in fast prototyping and debug interfaces. 

## Integration

This crate provides an amethyst `RenderPlugin` (available since amethyst 0.12) which properly renders ImGUI windows which are rendered using the `imgui-rs` crate. This integration is accomplished by calling the `amethyst_imgui::with` function anywhere within an Amethyst (a `System` or `State` is appropriate), which will render within the immediate-mode context of ImGui. All synchronization, frame handling and Amethyst input is handled within this crate.

A minimal example is available at [examples/demo_window.rs](examples/demo_window.rs)

```
# For Windows/Linux:
cargo run --example demo_window --features vulkan
# For MacOS:
cargo run --example demo_window --features metal
```

## Usage

This crate currently requires including the amethyst crate; this may introduce a full recompilation of amethyst due to differing features. If this is the case, you'll need to clone this git repository and and set the appropriate features.

This create uses the amethyst `shader-compiler`, which relies on `shaderc` to compile its shaders at build  time. Finally, this crate exposes the same rendering features as amethyst, and will pass them along to amethyst.

Example Cargo.toml Usage:
```toml
amethyst-imgui = { version = "0.7", features = ["vulkan"] }
```


`RenderPlugin` usage:
```rust
fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());
    let app_root = application_root_dir()?;
    let display_config_path = app_root.join("examples/display.ron");

    let game_data = GameDataBuilder::default()
        .with_barrier()
        .with(DemoSystem::default(), "imgui_use", &[])
        .with_bundle(amethyst::input::InputBundle::<amethyst::input::StringBindings>::default())?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderImgui::<amethyst::input::StringBindings>::default()),
        )?;

    Application::build("/", Example)?.build(game_data)?.run();

    Ok(())
}

```

An example `System` using amethyst-imgui:
```rust
pub struct ImguiDemoSystem;
impl<'s> amethyst::ecs::System<'s> for ImguiDemoSystem {
    type SystemData = ();
    fn run(&mut self, _: Self::SystemData) {
        amethyst_imgui::with(|ui| {
            ui.show_demo_window(&mut true);
        });
    }
}
```
