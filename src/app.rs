use oxyde::{AppState};
use anyhow::Result;
use oxyde::wgpu;
use oxyde::winit;
use oxyde::egui;

pub struct RustyBoids {
}

impl oxyde::App for RustyBoids {
    fn create(_app_state: &mut AppState) -> Self {
        Self {}
    }

    fn handle_event(&mut self, _app_state: &mut AppState, _event: &winit::event::Event<()>) -> Result<()> { Ok(()) }

    fn render_gui(&mut self, _ctx: &egui::Context) -> Result<()> { Ok(()) }

    fn update(&mut self, _app_state: &mut AppState) -> Result<()> { Ok(()) }

    fn render(
        &mut self,
        _app_state: &mut AppState,
        _encoder: &mut wgpu::CommandEncoder,
        _output_view: &wgpu::TextureView,
    ) -> Result<(), wgpu::SurfaceError> {
        Ok(())
    }
}
