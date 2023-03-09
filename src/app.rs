use anyhow::Result;
use oxyde::{egui, wgpu, winit, AppState, wgpu_utils::{binding_builder::{BindGroupLayoutBuilder, BindGroupBuilder}, uniform_buffer::UniformBuffer}};
use rand::prelude::Distribution;

use crate::boids::BoidData;
use crate::simulation::SimulationParametersUniformBufferContent;

use rand::SeedableRng;

const WORKGROUP_SIZE: usize = 64;

pub struct RustyBoids {
    boids_data: Vec<BoidData>,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    vertices_buffer: wgpu::Buffer,
    boid_buffers: PingPongBuffer,
    compute_bind_groups: PingPongBindGroup,
    frame_count: usize,

    simulation_parameters_uniform_buffer_content: SimulationParametersUniformBufferContent,
    simulation_parameters_uniform_buffer: UniformBuffer<SimulationParametersUniformBufferContent>,
    simulation_parameters_bind_group: wgpu::BindGroup,
}

pub struct PingPongBuffer {
    pub ping: wgpu::Buffer,
    pub pong: wgpu::Buffer,
}

impl PingPongBuffer {
    pub fn new(device: &wgpu::Device, data_slice: &[u8], usages: wgpu::BufferUsages, label: Option<&str>) -> Self {
        Self {
            ping: wgpu::util::DeviceExt::create_buffer_init(
                device,
                &wgpu::util::BufferInitDescriptor {
                label: Some(format!("{}[ping]", label.unwrap_or("unknown")).as_str()),
                contents: data_slice,
                usage: usages,
            }),
            pong: wgpu::util::DeviceExt::create_buffer_init(
                device,
                &wgpu::util::BufferInitDescriptor {
                label: Some(format!("{}[pong]", label.unwrap_or("unknown")).as_str()),
                contents: data_slice,
                usage: usages,
            })
        }
    }
}
    
pub struct PingPongBindGroup {
    pub ping: wgpu::BindGroup,
    pub pong: wgpu::BindGroup,
}

impl oxyde::App for RustyBoids {
    fn create(_app_state: &mut AppState) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<wgpu::Error>();
        _app_state.device.on_uncaptured_error(Box::new(move |e: wgpu::Error| {
            tx.send(e).expect("sending error failed");
        }));

        let initial_boids_count: usize = 2000;

        let mut rng = rand::rngs::StdRng::seed_from_u64(42);
        let unif = rand::distributions::Uniform::new_inclusive(-1.0, 1.0);
        let boids_data: Vec<BoidData> = (0..initial_boids_count)
            .map(|_| {
                BoidData::new(
                    nalgebra_glm::vec2(unif.sample(&mut rng), unif.sample(&mut rng)),
                    nalgebra_glm::vec2(unif.sample(&mut rng), unif.sample(&mut rng)) * 0.5,
                )
            })
            .collect();
        
        let boid_buffers = PingPongBuffer::new(
            &_app_state.device,
            bytemuck::cast_slice(&boids_data),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            Some("Boids Buffer"));

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

        let simulation_parameters_uniform_buffer_content = SimulationParametersUniformBufferContent::default();

        let simulation_parameters_uniform_buffer = UniformBuffer::new_with_data(
            &_app_state.device,
            &simulation_parameters_uniform_buffer_content
        );

        // Compute Pipeline
        let compute_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/compute.wgsl").into()),
        });

        let simulation_parameters_bind_group_layout_with_desc = BindGroupLayoutBuilder::new()
        .add_binding_compute(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<SimulationParametersUniformBufferContent>() as _),
        })
        .create(&_app_state.device, Some("compute_bind_group_layout"));
    
        let compute_bind_group_layout_with_desc = BindGroupLayoutBuilder::new()
        .add_binding_compute(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new((boids_data.len() * std::mem::size_of::<BoidData>()) as _),
        })
        .add_binding_compute(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new((boids_data.len() * std::mem::size_of::<BoidData>()) as _),
        })
        .create(&_app_state.device, Some("compute_bind_group_layout"));

        let simulation_parameters_bind_group = BindGroupBuilder::new(&simulation_parameters_bind_group_layout_with_desc)
        .resource(simulation_parameters_uniform_buffer.binding_resource())
        .create(&_app_state.device, Some("simulation_parameters_bind_group"));

        let compute_bind_groups = PingPongBindGroup {
            ping: BindGroupBuilder::new(&compute_bind_group_layout_with_desc)
                .resource(boid_buffers.pong.as_entire_binding())
                .resource(boid_buffers.ping.as_entire_binding())
                .create(&_app_state.device, Some("ping_compute_bind_group_layout")),

            pong: BindGroupBuilder::new(&compute_bind_group_layout_with_desc)
                .resource(boid_buffers.ping.as_entire_binding())
                .resource(boid_buffers.pong.as_entire_binding())
                .create(&_app_state.device, Some("pong_compute_bind_group_layout"))
        };

        let compute_pipeline = _app_state.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&_app_state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline"),
                bind_group_layouts: &[
                    &simulation_parameters_bind_group_layout_with_desc.layout,
                    &compute_bind_group_layout_with_desc.layout
                ],
                push_constant_ranges: &[],
            })),
            module: &compute_shader,
            entry_point: "cs_main",
        });
        
        // Render Pipeline
        let display_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Display Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/display.wgsl").into()),
        });

        let render_pipeline = _app_state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            render_pipeline,
            compute_pipeline,
            boid_buffers,
            compute_bind_groups,
            simulation_parameters_bind_group,
            vertices_buffer,
            simulation_parameters_uniform_buffer_content,
            simulation_parameters_uniform_buffer,
            frame_count: 0,
        }
    }


    fn handle_event(&mut self, _app_state: &mut AppState, _event: &winit::event::Event<()>) -> Result<()> { Ok(()) }

    fn render_gui(&mut self, _ctx: &egui::Context) -> Result<()> {
        
        egui::SidePanel::right("right panel").resizable(true).show(&_ctx, |ui| {

            egui::CollapsingHeader::new("Simulation settings").default_open(true).show(ui, |ui| {


                ui.add(egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.simulation_parameters_uniform_buffer_content.delta_t = v as f32;
                    }
                    self.simulation_parameters_uniform_buffer_content.delta_t as f64
                }).prefix("Delta t"));

                ui.add(egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.simulation_parameters_uniform_buffer_content.cohesion_distance = v as f32;
                    }
                    self.simulation_parameters_uniform_buffer_content.cohesion_distance as f64
                }).prefix("Cohesion distance"));

                ui.add(egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.simulation_parameters_uniform_buffer_content.aligment_distance = v as f32;
                    }
                    self.simulation_parameters_uniform_buffer_content.aligment_distance as f64
                }).prefix("Aligment distance"));

                ui.add(egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.simulation_parameters_uniform_buffer_content.separation_distance = v as f32;
                    }
                    self.simulation_parameters_uniform_buffer_content.separation_distance as f64
                }).prefix("Separation distance"));

                ui.add(egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.simulation_parameters_uniform_buffer_content.cohesion_scale = v as f32;
                    }
                    self.simulation_parameters_uniform_buffer_content.cohesion_scale as f64
                }).prefix("Cohesion scale"));

                ui.add(egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.simulation_parameters_uniform_buffer_content.aligment_scale = v as f32;
                    }
                    self.simulation_parameters_uniform_buffer_content.aligment_scale as f64
                }).prefix("Aligment scale"));

                ui.add(egui::Slider::from_get_set(0.0..=0.1, |optional_value: Option<f64>| {
                    if let Some(v) = optional_value {
                        self.simulation_parameters_uniform_buffer_content.separation_scale = v as f32;
                    }
                    self.simulation_parameters_uniform_buffer_content.separation_scale as f64
                }).prefix("Separation scale"));

            });
        });

        Ok(())
    
    }

    fn update(&mut self, _app_state: &mut AppState) -> Result<()> {

        self.simulation_parameters_uniform_buffer.update_content(&_app_state.queue, self.simulation_parameters_uniform_buffer_content);

        Ok(())
    }

    fn render(
        &mut self,
        _app_state: &mut AppState,
        _encoder: &mut wgpu::CommandEncoder,
        _output_view: &wgpu::TextureView,
    ) -> Result<(), wgpu::SurfaceError> {
        _encoder.push_debug_group("Compute Boids");
        {
            let mut compute_pass = _encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            let compute_bind_group = if self.frame_count % 2 == 0 { &self.compute_bind_groups.ping } else { &self.compute_bind_groups.pong };
            compute_pass.set_bind_group(0, &self.simulation_parameters_bind_group, &[]);
            compute_pass.set_bind_group(1, &compute_bind_group, &[]);
            compute_pass.dispatch_workgroups((self.boids_data.len() / WORKGROUP_SIZE + 1) as _, 1, 1);
        }
        _encoder.pop_debug_group();

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

            screen_render_pass.set_pipeline(&self.render_pipeline);
            screen_render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));

            let boid_buffer = if self.frame_count % 2 == 0 { &self.boid_buffers.ping } else { &self.boid_buffers.pong };

            screen_render_pass.set_vertex_buffer(1, boid_buffer.slice(..));
            screen_render_pass.draw(0..3, 0..self.boids_data.len() as _);
        }
        _encoder.pop_debug_group();
        
        self.frame_count += 1;
        Ok(())
    }
}
