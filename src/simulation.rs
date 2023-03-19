#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimulationParametersUniformBufferContent {
    pub delta_t: f32,
    pub view_radius: f32,
    pub cohesion_scale: f32,
    pub aligment_scale: f32,
    pub separation_scale: f32,
    pub grid_count: u32,
}

impl Default for SimulationParametersUniformBufferContent {
    fn default() -> Self {
        Self {
            delta_t: 0.04,
            view_radius: 0.05,
            cohesion_scale: 0.02,
            aligment_scale: 0.005,
            separation_scale: 0.05,
            grid_count: 4,
        }
    }
}
