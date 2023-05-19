use std::println;

use anyhow::Result;
use oxyde::{
    egui,
    wgpu,
    wgpu_utils::{binding_builder, buffers::StagingBufferWrapper, uniform_buffer::UniformBufferWrapper},
    winit,
    AppState,
};
use wgpu_profiler::{wgpu_profiler, GpuProfiler};

use crate::{simulation::SimulationParametersUniformBufferContent, utils::setup_ui_profiler};

pub type BoidSortingId = u32;
pub type BoidsPosition = nalgebra_glm::Vec2;
pub type BoidsVelocity = nalgebra_glm::Vec2;
pub type BoidsCellId = u32;

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

    simulation_profiler: GpuProfiler,

    vertices_buffer: wgpu::Buffer,

    sorting_id_staging_buffer: StagingBufferWrapper<BoidSortingId, false>,
    sorting_id_buffer: wgpu::Buffer,
    sorting_id_bind_group: wgpu::BindGroup,

    boids_per_cell_count_staging_buffer: StagingBufferWrapper<u32, false>,
    boids_per_cell_count_buffer: wgpu::Buffer,
    boids_per_cell_count_bind_group: wgpu::BindGroup,

    init_parameters_uniform_buffer: UniformBufferWrapper<InitParametersUniformBufferContent>,
    simulation_parameters_uniform_buffer: UniformBufferWrapper<SimulationParametersUniformBufferContent>,

    position_ping_buffer: wgpu::Buffer,
    velocity_ping_buffer: wgpu::Buffer,
    cell_id_ping_buffer: wgpu::Buffer,
    position_pong_buffer: wgpu::Buffer,
    velocity_pong_buffer: wgpu::Buffer,
    cell_id_pong_buffer: wgpu::Buffer,

    cell_id_staging_buffer: StagingBufferWrapper<BoidsCellId, true>,

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
            SimulationParametersUniformBufferContent {
                boids_count: initial_boids_count,
                ..SimulationParametersUniformBufferContent::default()
            },
            wgpu::ShaderStages::all(),
        );

        let cell_id_staging_buffer = StagingBufferWrapper::new(&_app_state.device, initial_boids_count as usize);

        let (
            position_ping_buffer,
            velocity_ping_buffer,
            cell_id_ping_buffer,
            position_pong_buffer,
            velocity_pong_buffer,
            cell_id_pong_buffer,
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
            wgpu::ShaderStages::VERTEX,
        );

        // Boids sorting id buffer
        let sorting_id_staging_buffer =
            StagingBufferWrapper::new_from_data(&_app_state.device, &(0..initial_boids_count as BoidSortingId).collect::<Vec<_>>());

        let sorting_id_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &_app_state.device,
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(sorting_id_staging_buffer.values_as_slice()),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            },
        );

        // bind group for sorting_id
        let sorting_id_bind_group_layout_with_desc = binding_builder::BindGroupLayoutBuilder::new()
            .add_binding_compute(wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(sorting_id_staging_buffer.bytes_size() as u64),
            })
            .create(&_app_state.device, None);

        let sorting_id_bind_group = binding_builder::BindGroupBuilder::new(&sorting_id_bind_group_layout_with_desc)
            .resource(sorting_id_buffer.as_entire_binding())
            .create(&_app_state.device, Some("sorting_id_bind_group"));

        let grid_size = simulation_parameters_uniform_buffer.content().grid_size as usize;

        println!("grid_size: {}", grid_size);
        let boids_per_cell_count_staging_buffer = StagingBufferWrapper::new_from_data(&_app_state.device, &vec![0; grid_size * grid_size + 1]);

        let boids_per_cell_count_buffer = wgpu::util::DeviceExt::create_buffer_init(
            &_app_state.device,
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(boids_per_cell_count_staging_buffer.values_as_slice()),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            },
        );

        // bind group for boid per cell count using binding_builder
        let boids_per_cell_count_bind_group_layout_with_desc = binding_builder::BindGroupLayoutBuilder::new()
            .add_binding_compute(wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(boids_per_cell_count_staging_buffer.bytes_size() as u64),
            })
            .create(&_app_state.device, None);

        let boids_per_cell_count_bind_group = binding_builder::BindGroupBuilder::new(&boids_per_cell_count_bind_group_layout_with_desc)
            .resource(boids_per_cell_count_buffer.as_entire_binding())
            .create(&_app_state.device, Some("boids_per_cell_count_bind_group"));

        let compute_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/computeGrid.wgsl").into()),
        });

        let init_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Init Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/init.wgsl").into()),
        });

        let display_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Display Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/display.wgsl").into()),
        });

        let (init_pipeline, compute_pipeline, render_pipeline) = RustyBoids::create_pipelines(
            &_app_state.device,
            &_app_state.config,
            &display_shader,
            &compute_shader,
            &init_shader,
            &ping_pong_bind_group_layout_builder_descriptor.layout,
            &read_only_bind_group_layout_builder_descriptor.layout,
            &sorting_id_bind_group_layout_with_desc.layout,
            &boids_per_cell_count_bind_group_layout_with_desc.layout,
            init_parameters_uniform_buffer.layout(),
            simulation_parameters_uniform_buffer.layout(),
        );

        let simulation_profiler = GpuProfiler::new(4, _app_state.queue.get_timestamp_period(), _app_state.device.features());

        Self {
            init_pipeline,
            render_pipeline,
            compute_pipeline,

            simulation_profiler,

            vertices_buffer,

            sorting_id_staging_buffer,
            sorting_id_buffer,
            sorting_id_bind_group,

            boids_per_cell_count_staging_buffer,
            boids_per_cell_count_buffer,
            boids_per_cell_count_bind_group,

            init_parameters_uniform_buffer,
            simulation_parameters_uniform_buffer,

            position_ping_buffer,
            velocity_ping_buffer,
            cell_id_ping_buffer,
            position_pong_buffer,
            velocity_pong_buffer,
            cell_id_pong_buffer,

            cell_id_staging_buffer,

            ping_pong_state: true,
            ping_pong_bind_group,
            pong_ping_bind_group,
            ping_bind_group,
            pong_bind_group,

            need_init: true,
        }
    }

    fn handle_event(&mut self, _app_state: &mut AppState, _event: &winit::event::Event<()>) -> Result<()> { Ok(()) }

    fn render_gui(&mut self, _ctx: &egui::Context) -> Result<()> {
        egui::SidePanel::right("right panel").resizable(true).show(_ctx, |ui| {
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

            if let Some(latest_profiler_results) = self.simulation_profiler.process_finished_frame() {
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

    fn render(&mut self, _app_state: &mut AppState, _output_view: &wgpu::TextureView) -> Result<(), wgpu::SurfaceError> {
        let dispatch_group_count = std::cmp::max(1, self.boids_count() / WORKGROUP_SIZE);

        let mut compute_encoder: wgpu::CommandEncoder = _app_state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Compute Boids Encoder") });

        if self.need_init {
            wgpu_profiler!("Init Boids", self.simulation_profiler, &mut compute_encoder, &_app_state.device, {
                let compute_pass = &mut compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });

                compute_pass.set_pipeline(&self.init_pipeline);
                compute_pass.set_bind_group(0, self.init_parameters_uniform_buffer.bind_group(), &[]);
                compute_pass.set_bind_group(1, self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                compute_pass.set_bind_group(
                    2,
                    if self.ping_pong_state {
                        &self.ping_pong_bind_group
                    } else {
                        &self.pong_ping_bind_group
                    },
                    &[],
                );
                compute_pass.dispatch_workgroups(dispatch_group_count, 1, 1);
            });

            self.need_init = false;
        } else {
            // explicit swap ping pong buffers
            self.ping_pong_state = !self.ping_pong_state;

            wgpu_profiler!("Compute Boids", self.simulation_profiler, &mut compute_encoder, &_app_state.device, {
                let mut compute_pass = compute_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                compute_pass.set_bind_group(
                    1,
                    if self.ping_pong_state {
                        &self.ping_pong_bind_group
                    } else {
                        &self.pong_ping_bind_group
                    },
                    &[],
                );
                compute_pass.set_bind_group(2, &self.sorting_id_bind_group, &[]);
                compute_pass.set_bind_group(3, &self.boids_per_cell_count_bind_group, &[]);
                compute_pass.dispatch_workgroups(dispatch_group_count, 1, 1);
            });
        }

        wgpu_profiler!("Read cell id", self.simulation_profiler, &mut compute_encoder, &_app_state.device, {
            self.cell_id_staging_buffer.encode_read(
                &mut compute_encoder,
                if self.ping_pong_state {
                    &self.cell_id_pong_buffer
                } else {
                    &self.cell_id_ping_buffer
                },
            );
        });

        _app_state.queue.submit(Some(compute_encoder.finish()));

        // map buffer wait for CPU read
        self.cell_id_staging_buffer.map_buffer();
        _app_state.device.poll(wgpu::Maintain::Wait);
        self.cell_id_staging_buffer.read_and_unmap_buffer();

        self.sort_from_cell_id();

        let mut copy_encoder: wgpu::CommandEncoder = _app_state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("copy sorting Encoder") });

        // Copy from staging buffer on GPU side
        wgpu_profiler!("Write sorting id", self.simulation_profiler, &mut copy_encoder, &_app_state.device, {
            self.sorting_id_staging_buffer
                .encode_write(&_app_state.queue, &mut copy_encoder, &self.sorting_id_buffer);
        });

        wgpu_profiler!("Write cell count", self.simulation_profiler, &mut copy_encoder, &_app_state.device, {
            self.boids_per_cell_count_staging_buffer
                .encode_write(&_app_state.queue, &mut copy_encoder, &self.boids_per_cell_count_buffer);
        });

        // TODO: why there is random crash (wgpu parent device is lost) during copy with specific simulation parameters?
        _app_state.queue.submit(Some(copy_encoder.finish()));

        let mut display_encoder: wgpu::CommandEncoder = _app_state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Boids Display Encoder") });

        wgpu_profiler!("Render Boids", self.simulation_profiler, &mut display_encoder, &_app_state.device, {
            let mut screen_render_pass = &mut display_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: _output_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                })],
                depth_stencil_attachment: None,
            });
            oxyde::fit_viewport_to_gui_available_rect(screen_render_pass, _app_state);

            screen_render_pass.set_pipeline(&self.render_pipeline);
            screen_render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
            screen_render_pass.set_vertex_buffer(1, self.sorting_id_buffer.slice(..));
            screen_render_pass.set_bind_group(0, self.simulation_parameters_uniform_buffer.bind_group(), &[]);
            screen_render_pass.set_bind_group(
                1,
                if self.ping_pong_state {
                    &self.pong_bind_group
                } else {
                    &self.ping_bind_group
                },
                &[],
            );
            screen_render_pass.draw(0..3, 0..self.boids_count());
        });

        // Why only one resolve_queries on the last encoder works ?
        self.simulation_profiler.resolve_queries(&mut display_encoder);
        _app_state.queue.submit(Some(display_encoder.finish()));

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
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        wgpu::Buffer,
        binding_builder::BindGroupLayoutWithDesc,
        wgpu::BindGroup,
        wgpu::BindGroup,
        binding_builder::BindGroupLayoutWithDesc,
        wgpu::BindGroup,
        wgpu::BindGroup,
    ) {
        let boids_count = boids_count as u64;
        // let usage = wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC;

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
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(position_buffer_descriptor.size),
                },
            )
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(velocity_buffer_descriptor.size),
                },
            )
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(cell_id_buffer_descriptor.size),
                },
            )
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(position_buffer_descriptor.size),
                },
            )
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(velocity_buffer_descriptor.size),
                },
            )
            .add_binding(
                ping_pong_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(cell_id_buffer_descriptor.size),
                },
            )
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
            .add_binding(
                read_only_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset,
                    min_binding_size: wgpu::BufferSize::new(position_buffer_descriptor.size),
                },
            )
            .add_binding(
                read_only_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset,
                    min_binding_size: wgpu::BufferSize::new(velocity_buffer_descriptor.size),
                },
            )
            .add_binding(
                read_only_buffer_visibility,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset,
                    min_binding_size: wgpu::BufferSize::new(cell_id_buffer_descriptor.size),
                },
            )
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
            position_ping_buffer,
            velocity_ping_buffer,
            cell_id_ping_buffer,
            position_pong_buffer,
            velocity_pong_buffer,
            cell_id_pong_buffer,
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
        sorting_id_bind_group_layout: &wgpu::BindGroupLayout,
        boids_per_cell_count_bind_group_layout: &wgpu::BindGroupLayout,

        init_parameters_uniform_buffer_layout: &wgpu::BindGroupLayout,
        simulation_parameters_uniform_buffer_layout: &wgpu::BindGroupLayout,
    ) -> (wgpu::ComputePipeline, wgpu::ComputePipeline, wgpu::RenderPipeline) {
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline"),
                bind_group_layouts: &[
                    simulation_parameters_uniform_buffer_layout,
                    ping_pong_bind_group_layout,
                    sorting_id_bind_group_layout,
                    boids_per_cell_count_bind_group_layout,
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
            module: init_shader,
            entry_point: "cs_main",
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[simulation_parameters_uniform_buffer_layout, read_only_bind_group_layout],
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
                module: display_shader,
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

    fn boids_count(&self) -> u32 { self.simulation_parameters_uniform_buffer.content().boids_count }

    fn sort_from_cell_id(&mut self) {
        let boid_count_usize = self.boids_count() as usize;

        let boids_per_cell_count_slice = self.boids_per_cell_count_staging_buffer.values_as_slice_mut();
        // count boids per cell
        boids_per_cell_count_slice.fill(0);
        self.cell_id_staging_buffer.iter().for_each(|boid_cell_id| {
            boids_per_cell_count_slice[*boid_cell_id as usize] += 1;
        });

        // partial sum of boids per cell
        for i in 1..boids_per_cell_count_slice.len() {
            boids_per_cell_count_slice[i] += boids_per_cell_count_slice[i - 1];
        }

        // sort boids
        self.sorting_id_staging_buffer.clear();
        let sorting_id_values_slice = self.sorting_id_staging_buffer.values_as_slice_mut();
        for i in 0..boid_count_usize {
            let boid_cell_id = self.cell_id_staging_buffer[i];
            let boid_target_index = boids_per_cell_count_slice[boid_cell_id as usize] - 1;
            boids_per_cell_count_slice[boid_cell_id as usize] -= 1;
            sorting_id_values_slice[boid_target_index as usize] = i as BoidSortingId;
        }

        // Debug display
        // println!("boids_sorting_id_value: {:?}\n", self.sorting_id_staging_buffer.values_as_slice());
        // println!("boids_per_cell_count: {:?}\n", self.boids_per_cell_count_staging_buffer.values_as_slice());

        // let boids_cell_id_using_sorting_order = self.sorting_id_staging_buffer.iter().map(|boid_sorting_id| self.cell_id_staging_buffer[*boid_sorting_id as usize].x).collect::<Vec<_>>();
        // println!("boids_cell_id_using_sorting_order: {:?} (sorted : {:?})\n\n",
        //     boids_cell_id_using_sorting_order,
        //     boids_cell_id_using_sorting_order.windows(2).all(|x| x[0] <= x[1])
        // );
    }
}
