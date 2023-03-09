use oxyde::{wgpu_utils::uniform_buffer::UniformBuffer, wgpu};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimulationParametersUniformBufferContent {
    pub delta_t: f32,
    pub cohesion_distance: f32,
    pub aligment_distance: f32,
    pub separation_distance: f32,
    pub cohesion_scale: f32,
    pub aligment_scale: f32,
    pub separation_scale: f32,
}

impl Default for SimulationParametersUniformBufferContent {
    fn default() -> Self {
        Self {
            delta_t: 0.04,
            cohesion_distance: 0.1,
            aligment_distance: 0.025,
            separation_distance: 0.025,
            cohesion_scale: 0.02,
            aligment_scale: 0.005,
            separation_scale: 0.05,
        }
    }
}