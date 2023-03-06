use anyhow::Result;
use oxyde::{egui, wgpu, winit, AppState};
use rand::prelude::Distribution;

use crate::boids::BoidData;

use rand::SeedableRng;

pub struct RustyBoids {
    boids_data: Vec<BoidData>,
    pipeline: wgpu::RenderPipeline,
    boid_buffer: wgpu::Buffer,
    vertices_buffer: wgpu::Buffer,
}

impl oxyde::App for RustyBoids {
    fn create(_app_state: &mut AppState) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<wgpu::Error>();
        _app_state.device.on_uncaptured_error(Box::new(move |e: wgpu::Error| {
            tx.send(e).expect("sending error failed");
        }));

        let initial_boids_count: usize = 100;

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let unif = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
        let boids_data: Vec<BoidData> = (0..initial_boids_count)
            .map(|_| {
                BoidData::new(
                    nalgebra_glm::vec2(unif.sample(&mut rng), unif.sample(&mut rng)),
                    nalgebra_glm::vec2(unif.sample(&mut rng), unif.sample(&mut rng)) * 0.1,
                )
            })
            .collect();

        // Create helper for ping pong buffer swapping in compute shader
        let boid_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &_app_state.device,
            &wgpu::util::BufferInitDescriptor {
                label: Some("Boids Buffer"),
                contents: bytemuck::cast_slice(&boids_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            },
        );

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

        let display_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Display Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/display.wgsl").into()),
        });

        let pipeline = _app_state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&_app_state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            })),

            vertex: wgpu::VertexState {
                module: &display_shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 2 * 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![2 => Float32x2],
                    },
                    BoidData::vertex_buffer_layout(),
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: _app_state.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        _app_state.device.on_uncaptured_error(Box::new(|err| panic!("{}", err)));

        if let Ok(err) = rx.try_recv() {
            panic!("{}", err);
        }

        Self {
            boids_data,
            pipeline,
            boid_buffer,
            vertices_buffer,
        }
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
        _encoder.push_debug_group("Render Boids");
        {
            let mut screen_render_pass = _encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: _output_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                })],
                depth_stencil_attachment: None,
            });

            // Update viewport accordingly to the Ui to display in the available rect
            // It must be multiplied by window scale factor as render pass use physical pixels screen size

            let window_scale_factor = _app_state.window.scale_factor() as f32;
            let available_rect = _app_state.gui.available_rect;
            let available_rect_size = available_rect.size();

            screen_render_pass.set_viewport(
                available_rect.min.x * window_scale_factor,
                available_rect.min.y * window_scale_factor,
                available_rect_size.x * window_scale_factor,
                available_rect_size.y * window_scale_factor,
                0.0,
                1.0,
            );

            screen_render_pass.set_pipeline(&self.pipeline);
            screen_render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            screen_render_pass.set_vertex_buffer(1, self.boid_buffer.slice(..));
            screen_render_pass.draw(0..3, 0..self.boids_data.len() as _);
        }
        _encoder.pop_debug_group();

        Ok(())
    }
}
