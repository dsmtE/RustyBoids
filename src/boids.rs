use oxyde::wgpu;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoidData {
    position: nalgebra_glm::Vec2,
    velocity: nalgebra_glm::Vec2,
}

impl BoidData {
    pub fn new(position: nalgebra_glm::Vec2, velocity: nalgebra_glm::Vec2) -> Self { Self { position, velocity } }
}

impl BoidData {
    const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    pub fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<BoidData>() as _,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::VERTEX_ATTRIBUTES,
        }
    }
}
