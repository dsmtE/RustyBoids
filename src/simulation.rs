pub mod parameters;
pub mod types;
pub mod gpu_spatial_partitioning_strategy;

use oxyde::{wgpu, wgpu_utils::uniform_buffer::UniformBufferWrapper, AppState};
pub use parameters::SimulationParametersUniformBufferContent;

use self::parameters::InitParametersUniformBufferContent;

pub trait SimulationStrategy {
    fn render(
        &mut self,
        _app_state: &mut AppState,
        _output_view: &wgpu::TextureView,
        init_parameters_uniform_buffer: &UniformBufferWrapper<InitParametersUniformBufferContent>,
        simulation_parameters_uniform_buffer: &UniformBufferWrapper<SimulationParametersUniformBufferContent>,
        vertices_buffer: &wgpu::Buffer,
        simulation_profiler: &mut wgpu_profiler::GpuProfiler,
        need_init: &mut bool,
    ) -> Result<(), wgpu::SurfaceError>;
}