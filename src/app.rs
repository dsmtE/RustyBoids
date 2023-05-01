use anyhow::Result;
use oxyde::{
    egui,
    wgpu,
    wgpu_utils::{
        uniform_buffer::UniformBufferWrapper,
        SingleBufferWrapper,
        binding_builder,
    },
    winit,
    AppState,
};
use wgpu_profiler::{wgpu_profiler, GpuProfiler};

use crate::{simulation::SimulationParametersUniformBufferContent, utils::setup_ui_profiler};

pub type BoidSortingId = u32;
pub type BoidsPosition = nalgebra_glm::Vec2;
pub type BoidsVelocity = nalgebra_glm::Vec2;
pub type BoidsCellId = nalgebra_glm::UVec2;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct InitParametersUniformBufferContent {
    pub seed: u32,
}

const WORKGROUP_SIZE: u32 = 64;

pub struct RustyBoids {
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    init_pipeline: wgpu::ComputePipeline,

    vertices_buffer: wgpu::Buffer,

    boids_sorting_id_buffer_wrapper: SingleBufferWrapper,
    simulation_profiler: GpuProfiler,

    init_parameters_uniform_buffer: UniformBufferWrapper<InitParametersUniformBufferContent>,
    simulation_parameters_uniform_buffer: UniformBufferWrapper<SimulationParametersUniformBufferContent>,

    ping_pong_state: bool,
    ping_pong_bind_group: wgpu::BindGroup,
    pong_ping_bind_group: wgpu::BindGroup,
    ping_bind_group: wgpu::BindGroup,
    pong_bind_group: wgpu::BindGroup,

    need_init: bool,
}

impl oxyde::App for RustyBoids {
    fn create(_app_state: &mut AppState) -> Self {
        let initial_boids_count: u32 = 512;

        let (
            ping_pong_bind_group_layout_builder_descriptor,
            ping_pong_bind_group,
            pong_ping_bind_group,
            read_only_bind_group_layout_builder_descriptor,
            ping_bind_group,
            pong_bind_group,
        ) = RustyBoids::create_boids_buffers_and_bind_groups(
            &_app_state.device,
            initial_boids_count,
            wgpu::ShaderStages::COMPUTE,
            wgpu::ShaderStages::VERTEX
        );

        let sorting_ids: Vec<BoidSortingId> = (0..initial_boids_count).map(|i| i as BoidSortingId).collect();
        let boids_sorting_id_buffer_wrapper = SingleBufferWrapper::new_from_data(
            &_app_state.device,
            &sorting_ids,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            wgpu::ShaderStages::COMPUTE,
            wgpu::BufferBindingType::Storage { read_only: false },
            false,
            Some("Boid Sorting Id Buffer"),
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

        let init_parameters_uniform_buffer = UniformBufferWrapper::new(
            &_app_state.device,
            InitParametersUniformBufferContent::default(),
            wgpu::ShaderStages::COMPUTE,
        );

        let simulation_parameters_uniform_buffer = UniformBufferWrapper::new(
            &_app_state.device,
            SimulationParametersUniformBufferContent{
                boids_count: initial_boids_count,
                ..SimulationParametersUniformBufferContent::default()
            },
            wgpu::ShaderStages::all(),
        );

        let compute_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/computeNaive.wgsl").into()),
        });

        let init_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Init Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/init.wgsl").into()),
        });

        let display_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Display Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/display.wgsl").into()),
        });

        let (
            init_pipeline,
            compute_pipeline,
            render_pipeline,
        ) = RustyBoids::create_pipelines(
            &_app_state.device,
            &_app_state.config,
            &display_shader,
            &compute_shader,
            &init_shader,
            &ping_pong_bind_group_layout_builder_descriptor.layout,
            &read_only_bind_group_layout_builder_descriptor.layout,
            &init_parameters_uniform_buffer.layout(),
            &simulation_parameters_uniform_buffer.layout(),
        );

        let simulation_profiler = GpuProfiler::new(4, _app_state.queue.get_timestamp_period(), _app_state.device.features());

        Self {
            need_init: true,

            init_pipeline,
            render_pipeline,
            compute_pipeline,

            simulation_profiler,
            vertices_buffer,
            init_parameters_uniform_buffer,
            simulation_parameters_uniform_buffer,
            boids_sorting_id_buffer_wrapper,

            ping_pong_bind_group,
            pong_ping_bind_group,
            ping_bind_group,
            pong_bind_group,
            ping_pong_state: true,
        }
    }

    fn handle_event(&mut self, _app_state: &mut AppState, _event: &winit::event::Event<()>) -> Result<()> { Ok(()) }

    fn render_gui(&mut self, _ctx: &egui::Context) -> Result<()> {
        egui::SidePanel::right("right panel").resizable(true).show(_ctx, |ui| {
            
            self.simulation_parameters_uniform_buffer.content_mut().display_ui(ui);

            egui::CollapsingHeader::new("Init settings").default_open(true).show(ui, |ui| {
                ui.add(egui::DragValue::new(&mut self.init_parameters_uniform_buffer.content_mut().seed).speed(1).prefix("Seed: "));
                if ui.button("Init boids").clicked() {
                    self.need_init = true;
                }
            });
            

            if let Some(latest_profiler_results) = self.simulation_profiler.process_finished_frame() {
                setup_ui_profiler(ui, &latest_profiler_results, 1);
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

    fn render(
        &mut self,
        _app_state: &mut AppState,
        _output_view: &wgpu::TextureView,
    ) -> Result<(), wgpu::SurfaceError> {

        let mut encoder: wgpu::CommandEncoder = _app_state.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Boids Encoder") });

        wgpu_profiler!("Wgpu Profiler", self.simulation_profiler, &mut encoder, &_app_state.device, {

            let dispatch_group_count = std::cmp::max(1, self.boids_count() / WORKGROUP_SIZE);
            
            if self.need_init {
                wgpu_profiler!("Init Boids", self.simulation_profiler, &mut encoder, &_app_state.device, {
                    let compute_pass = &mut encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });

                    compute_pass.set_pipeline(&self.init_pipeline);
                    compute_pass.set_bind_group(0, &self.init_parameters_uniform_buffer.bind_group(), &[]);
                    compute_pass.set_bind_group(1, &self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                    compute_pass.set_bind_group(2, if self.ping_pong_state { &self.ping_pong_bind_group } else { &self.pong_ping_bind_group }, &[]);
                    compute_pass.dispatch_workgroups(dispatch_group_count, 1, 1);
                });

                self.need_init = false;
            }

            // explicit swap ping pong buffers
            self.ping_pong_state = !self.ping_pong_state;

            wgpu_profiler!("Compute Boids", self.simulation_profiler, &mut encoder, &_app_state.device, {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });
                
                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                compute_pass.set_bind_group(1, if self.ping_pong_state { &self.ping_pong_bind_group } else { &self.pong_ping_bind_group }, &[]);
                compute_pass.dispatch_workgroups(dispatch_group_count, 1, 1);
            });

            wgpu_profiler!("Render Boids", self.simulation_profiler, &mut encoder, &_app_state.device, {
                let mut screen_render_pass = &mut encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: _output_view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                    })],
                    depth_stencil_attachment: None,
                });
                oxyde::fit_viewport_to_gui_available_rect(&mut screen_render_pass, _app_state);

                screen_render_pass.set_pipeline(&self.render_pipeline);
                screen_render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
                screen_render_pass.set_vertex_buffer(1, self.boids_sorting_id_buffer_wrapper.buffer().slice(..));
                screen_render_pass.set_bind_group(0, &self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                screen_render_pass.set_bind_group(1, if self.ping_pong_state { &self.pong_bind_group } else { &self.ping_bind_group }, &[]);
                screen_render_pass.draw(0..3, 0..self.boids_count());
            });
        });

        self.simulation_profiler.resolve_queries(&mut encoder);

        _app_state.queue.submit(Some(encoder.finish()));

        Ok(())
    }

    fn post_render(&mut self, _app_state: &mut AppState) -> Result<()> {
        self.simulation_profiler.end_frame().unwrap();

        Ok(())
    }
}

impl RustyBoids {
    // fn that create buffers and bind groups for boids data
    fn create_boids_buffers_and_bind_groups(
        device: &wgpu::Device,
        boids_count: u32,
        ping_pong_buffer_visibility: wgpu::ShaderStages,
        read_only_buffer_visibility: wgpu::ShaderStages,
    ) -> (
        binding_builder::BindGroupLayoutWithDesc,
        wgpu::BindGroup,
        wgpu::BindGroup,
        binding_builder::BindGroupLayoutWithDesc,
        wgpu::BindGroup,
        wgpu::BindGroup,
    ) {

        let boids_count = boids_count as u64;
        // let usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let usage = wgpu::BufferUsages::STORAGE;

        let position_buffer_descriptor = wgpu::BufferDescriptor {
            label: Some("Position"),
            size: boids_count * std::mem::size_of::<BoidsPosition>() as u64,
            usage,
            mapped_at_creation: false,
        };

        let velocity_buffer_descriptor = wgpu::BufferDescriptor {
            label: Some("Velocity"),
            size: boids_count * std::mem::size_of::<BoidsVelocity>() as u64,
            usage,
            mapped_at_creation: false,
        };

        let cell_id_buffer_descriptor = wgpu::BufferDescriptor {
            label: Some("Cell Id"),
            size: boids_count * std::mem::size_of::<BoidsCellId>() as u64,
            usage,
            mapped_at_creation: false,
        };

        let position_ping_buffer = device.create_buffer(&position_buffer_descriptor);
        let velocity_ping_buffer = device.create_buffer(&velocity_buffer_descriptor);
        let cell_id_ping_buffer = device.create_buffer(&cell_id_buffer_descriptor);

        let position_pong_buffer = device.create_buffer(&position_buffer_descriptor);
        let velocity_pong_buffer = device.create_buffer(&velocity_buffer_descriptor);
        let cell_id_pong_buffer = device.create_buffer(&cell_id_buffer_descriptor);

        let ping_pong_bind_group_layout_builder_descriptor = binding_builder::BindGroupLayoutBuilder::new()
            .add_binding(ping_pong_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(position_buffer_descriptor.size),
            })
            .add_binding(ping_pong_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(velocity_buffer_descriptor.size),
            })
            .add_binding(ping_pong_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(cell_id_buffer_descriptor.size),
            })

            .add_binding(ping_pong_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(position_buffer_descriptor.size),
            })
            .add_binding(ping_pong_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(velocity_buffer_descriptor.size),
            })
            .add_binding(ping_pong_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(cell_id_buffer_descriptor.size),
            })
            .create(device, Some("Boids data (ping <-> pong)"));

        let ping_pong_bind_group = binding_builder::BindGroupBuilder::new(&ping_pong_bind_group_layout_builder_descriptor)
            .resource(position_ping_buffer.as_entire_binding())
            .resource(velocity_ping_buffer.as_entire_binding())
            .resource(cell_id_ping_buffer.as_entire_binding())

            .resource(position_pong_buffer.as_entire_binding())
            .resource(velocity_pong_buffer.as_entire_binding())
            .resource(cell_id_pong_buffer.as_entire_binding())
            .create(device, Some("Boids data (ping -> pong)"));

        let pong_ping_bind_group = binding_builder::BindGroupBuilder::new(&ping_pong_bind_group_layout_builder_descriptor)
            .resource(position_pong_buffer.as_entire_binding())
            .resource(velocity_pong_buffer.as_entire_binding())
            .resource(cell_id_pong_buffer.as_entire_binding())

            .resource(position_ping_buffer.as_entire_binding())
            .resource(velocity_ping_buffer.as_entire_binding())
            .resource(cell_id_ping_buffer.as_entire_binding())
            .create(device, Some("Boids data (pong -> ping)"));
        
        // read only bind group for final display
        let has_dynamic_offset = false;
        let read_only_bind_group_layout_builder_descriptor = binding_builder::BindGroupLayoutBuilder::new()
            .add_binding(read_only_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset,
                min_binding_size: wgpu::BufferSize::new(position_buffer_descriptor.size),
            })
            .add_binding(read_only_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset,
                min_binding_size: wgpu::BufferSize::new(velocity_buffer_descriptor.size),
            })
            .add_binding(read_only_buffer_visibility, wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset,
                min_binding_size: wgpu::BufferSize::new(cell_id_buffer_descriptor.size),
            })
            .create(device, Some("Boids data (read only)"));

        let ping_bind_group = binding_builder::BindGroupBuilder::new(&read_only_bind_group_layout_builder_descriptor)
            .resource(position_ping_buffer.as_entire_binding())
            .resource(velocity_ping_buffer.as_entire_binding())
            .resource(cell_id_ping_buffer.as_entire_binding())
            .create(device, Some("Boids data (ping read only)"));

        let pong_bind_group = binding_builder::BindGroupBuilder::new(&read_only_bind_group_layout_builder_descriptor)
            .resource(position_pong_buffer.as_entire_binding())
            .resource(velocity_pong_buffer.as_entire_binding())
            .resource(cell_id_pong_buffer.as_entire_binding())
            .create(device, Some("Boids data (pong read only)"));

        (
            ping_pong_bind_group_layout_builder_descriptor,
            ping_pong_bind_group,
            pong_ping_bind_group,
            read_only_bind_group_layout_builder_descriptor,
            ping_bind_group,
            pong_bind_group,
        )
    }

    // fn to create pipelines
    fn create_pipelines(
        device: &wgpu::Device,
        surface_configuration: &wgpu::SurfaceConfiguration,
        display_shader: &wgpu::ShaderModule,
        compute_shader: &wgpu::ShaderModule,
        init_shader: &wgpu::ShaderModule,

        ping_pong_bind_group_layout: &wgpu::BindGroupLayout,
        read_only_bind_group_layout: &wgpu::BindGroupLayout,

        init_parameters_uniform_buffer_layout: &wgpu::BindGroupLayout,
        simulation_parameters_uniform_buffer_layout: &wgpu::BindGroupLayout,
    ) -> (
        wgpu::ComputePipeline,
        wgpu::ComputePipeline,
        wgpu::RenderPipeline
    ) {
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline"),
                bind_group_layouts: &[
                    simulation_parameters_uniform_buffer_layout,
                    ping_pong_bind_group_layout,
                    ],
                push_constant_ranges: &[],
            })),
            module: compute_shader,
            entry_point: "cs_main",
        });

        let init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Init pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Init Pipeline"),
                bind_group_layouts: &[
                    init_parameters_uniform_buffer_layout,
                    simulation_parameters_uniform_buffer_layout,
                    ping_pong_bind_group_layout,
                    ],
                push_constant_ranges: &[],
            })),
            module: &init_shader,
            entry_point: "cs_main",
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    simulation_parameters_uniform_buffer_layout,
                    read_only_bind_group_layout,
                    ],
                push_constant_ranges: &[],
            })),

            vertex: wgpu::VertexState {
                module: display_shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<nalgebra_glm::Vec2>() as _,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<BoidSortingId>() as _,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![1 => Uint32],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &display_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_configuration.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        (init_pipeline, compute_pipeline, render_pipeline)
    }

    fn boids_count(&self) -> u32 {
        self.simulation_parameters_uniform_buffer.content().boids_count
    }
}
