use anyhow::Result;
use oxyde::{
    egui,
    wgpu,
    wgpu_utils::uniform_buffer::UniformBufferWrapper,
    AppState,
    winit::event::Event,
};
use wgpu_profiler::{GpuProfiler, GpuProfilerSettings};

use crate::{
    simulation::{
        gpu_spatial_partitioning_strategy::create_gpu_spatial_partitioning_strategy, parameters::InitParametersUniformBufferContent, SimulationParametersUniformBufferContent, SimulationStrategy
}   ,
    utils::setup_ui_profiler,
};

pub struct RustyBoids {
    pub simulation_profiler: GpuProfiler,

    pub vertices_buffer: wgpu::Buffer,

    simulation_strategy: Box<dyn SimulationStrategy>,

    pub init_parameters_uniform_buffer: UniformBufferWrapper<InitParametersUniformBufferContent>,
    pub simulation_parameters_uniform_buffer: UniformBufferWrapper<SimulationParametersUniformBufferContent>,

    pub need_init: bool,
}

impl oxyde::App for RustyBoids {
    fn create(_app_state: &mut AppState) -> Self {
        // buffer for the three 2d triangle vertices of each boid
        let vertex_buffer_data = [-0.01f32, -0.02, 0.01, -0.02, 0.00, 0.02];
        let vertices_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &_app_state.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::bytes_of(&vertex_buffer_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            },
        );


        let init_parameters_uniform_buffer =
            UniformBufferWrapper::new(&_app_state.device, InitParametersUniformBufferContent::default(), wgpu::ShaderStages::COMPUTE);

        let simulation_parameters_uniform_buffer = UniformBufferWrapper::new(
            &_app_state.device,
            SimulationParametersUniformBufferContent::default(),
            wgpu::ShaderStages::all(),
        );

        let simulation_strategy = create_gpu_spatial_partitioning_strategy(
            &_app_state.device,
            &_app_state.config,
            &init_parameters_uniform_buffer,
            &simulation_parameters_uniform_buffer,
            false,
        );

        let simulation_profiler = GpuProfiler::new(GpuProfilerSettings::default()).unwrap();

        Self {
            vertices_buffer,
            simulation_strategy,
            simulation_profiler,
            init_parameters_uniform_buffer,
            simulation_parameters_uniform_buffer,
            need_init: true,
        }
    }

    fn handle_event<T: 'static>(&mut self, _app_state: &mut AppState, _event: &Event<T>) -> Result<()> { Ok(()) }

    fn render_gui(&mut self, _app_state: &mut AppState) -> Result<()> {
        egui::SidePanel::right("right panel").resizable(true).show(_app_state.egui_renderer.context(), |ui| {
            self.simulation_parameters_uniform_buffer.content_mut().display_ui(ui);

            egui::CollapsingHeader::new("Init settings").default_open(true).show(ui, |ui| {
                ui.add(
                    egui::DragValue::new(&mut self.init_parameters_uniform_buffer.content_mut().seed)
                        .speed(1)
                        .prefix("Seed: "),
                );
                if ui.button("Init boids").clicked() {
                    self.need_init = true;
                }
            });

            if let Some(latest_profiler_results) = self.simulation_profiler.process_finished_frame(_app_state.queue.get_timestamp_period()) {
                egui::CollapsingHeader::new("Wgpu Profiler")
                    .default_open(true)
                    .show(ui, |ui| setup_ui_profiler(ui, &latest_profiler_results, 1));
            } else {
                ui.label("No profiler results yet");
            }
        });

        Ok(())
    }

    fn update(&mut self, _app_state: &mut AppState) -> Result<()> {
        self.simulation_parameters_uniform_buffer.update_content(&_app_state.queue);
        self.init_parameters_uniform_buffer.update_content(&_app_state.queue);

        Ok(())
    }

    fn render(&mut self, _app_state: &mut AppState, _output_view: &wgpu::TextureView) -> Result<()> {
        self.simulation_strategy.render(
            _app_state,
            _output_view,
            &self.init_parameters_uniform_buffer,
            &self.simulation_parameters_uniform_buffer,
            &self.vertices_buffer,
            &mut self.simulation_profiler,
            &mut self.need_init,
        )?;
        Ok(())
    }

    fn post_render(&mut self, _app_state: &mut AppState) -> Result<()> {
        self.simulation_profiler.end_frame().unwrap();

        Ok(())
    }
}
