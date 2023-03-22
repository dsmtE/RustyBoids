use anyhow::Result;
use oxyde::{
    egui,
    wgpu,
    wgpu_utils::{
        uniform_buffer::UniformBufferWrapper,
        PingPongBuffer,
        SingleBufferWrapper,
    },
    winit,
    AppState,
};
use wgpu_profiler::{wgpu_profiler, GpuProfiler};

use crate::{boids::{BoidData, BoidSortingId}, simulation::SimulationParametersUniformBufferContent, utils::setup_ui_profiler};


#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct InitParametersUniformBufferContent {
    pub seed: u32,
}

const WORKGROUP_SIZE: usize = 64;

pub struct RustyBoids {
    boids_count: usize,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    init_pipeline: wgpu::ComputePipeline,
    vertices_buffer: wgpu::Buffer,
    boid_buffers: PingPongBuffer,

    boids_sorting_id_buffer_wrapper: SingleBufferWrapper,
    simulation_profiler: GpuProfiler,

    init_parameters_uniform_buffer: UniformBufferWrapper<InitParametersUniformBufferContent>,
    simulation_parameters_uniform_buffer: UniformBufferWrapper<SimulationParametersUniformBufferContent>,

    need_init: bool,
}

impl oxyde::App for RustyBoids {
    fn create(_app_state: &mut AppState) -> Self {
        let initial_boids_count: usize = 4096;

        let boid_buffers = PingPongBuffer::from_buffer_descriptor(
            &_app_state.device,
            &wgpu::BufferDescriptor {
                label: Some("Boid Buffers"),
                size: (initial_boids_count * std::mem::size_of::<BoidData>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            },
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
            SimulationParametersUniformBufferContent::default(),
            wgpu::ShaderStages::all(),
        );

        // Compute Pipeline
        let compute_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/computeNaive.wgsl").into()),
        });

        let compute_pipeline = _app_state.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&_app_state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline"),
                bind_group_layouts: &[&simulation_parameters_uniform_buffer.layout(), boid_buffers.layout()],
                push_constant_ranges: &[],
            })),
            module: &compute_shader,
            entry_point: "cs_main",
        });

        // Init Pipeline
        let init_shader = _app_state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Init Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/init.wgsl").into()),
        });

        let init_pipeline = _app_state.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Init pipeline"),
            layout: Some(&_app_state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Init Pipeline"),
                bind_group_layouts: &[&init_parameters_uniform_buffer.layout(), &simulation_parameters_uniform_buffer.layout(), boid_buffers.layout()],
                push_constant_ranges: &[],
            })),
            module: &init_shader,
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
                bind_group_layouts: &[&simulation_parameters_uniform_buffer.layout()],
                push_constant_ranges: &[],
            })),

            vertex: wgpu::VertexState {
                module: &display_shader,
                entry_point: "vs_main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: 2 * 4,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![3 => Float32x2],
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

        let simulation_profiler = GpuProfiler::new(4, _app_state.queue.get_timestamp_period(), _app_state.device.features());

        Self {
            boids_count: initial_boids_count,
            need_init: true,
            init_pipeline,
            render_pipeline,
            compute_pipeline,
            boid_buffers,
            simulation_profiler,
            vertices_buffer,
            init_parameters_uniform_buffer,
            simulation_parameters_uniform_buffer,
            boids_sorting_id_buffer_wrapper,
        }
    }

    fn handle_event(&mut self, _app_state: &mut AppState, _event: &winit::event::Event<()>) -> Result<()> { Ok(()) }

    fn render_gui(&mut self, _ctx: &egui::Context) -> Result<()> {
        egui::SidePanel::right("right panel").resizable(true).show(_ctx, |ui| {
            
            self.simulation_parameters_uniform_buffer.content().display_ui(ui);

            egui::CollapsingHeader::new("Init settings").default_open(true).show(ui, |ui| {
                ui.add(egui::DragValue::new(&mut self.init_parameters_uniform_buffer.content().seed).speed(1).prefix("Seed: "));
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
        _encoder: &mut wgpu::CommandEncoder,
        _output_view: &wgpu::TextureView,
    ) -> Result<(), wgpu::SurfaceError> {
        wgpu_profiler!("Wgpu Profiler", self.simulation_profiler, _encoder, &_app_state.device, {

            let dispatch_group_count: u32 = (self.boids_count / WORKGROUP_SIZE) as u32 + 1;
            
            if self.need_init {
                wgpu_profiler!("Init Boids", self.simulation_profiler, _encoder, &_app_state.device, {
                    let mut compute_pass = _encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });

                    compute_pass.set_pipeline(&self.init_pipeline);
                    compute_pass.set_bind_group(0, &self.init_parameters_uniform_buffer.bind_group(), &[]);
                    compute_pass.set_bind_group(1, &self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                    compute_pass.set_bind_group(2, self.boid_buffers.get_current_source_bind_group(), &[]);
                    compute_pass.dispatch_workgroups(dispatch_group_count, 1, 1);
                });

                self.need_init = false;
            }
            wgpu_profiler!("Compute Boids", self.simulation_profiler, _encoder, &_app_state.device, {
                let mut compute_pass = _encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Compute Pass") });

                compute_pass.set_pipeline(&self.compute_pipeline);
                compute_pass.set_bind_group(0, &self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                compute_pass.set_bind_group(1, self.boid_buffers.get_next_target_bind_group(), &[]);
                compute_pass.dispatch_workgroups(dispatch_group_count, 1, 1);
            });

            wgpu_profiler!("Render Boids", self.simulation_profiler, _encoder, &_app_state.device, {
                let mut screen_render_pass = _encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                screen_render_pass.set_bind_group(0, &self.simulation_parameters_uniform_buffer.bind_group(), &[]);
                screen_render_pass.set_vertex_buffer(0, self.vertices_buffer.slice(..));
                screen_render_pass.set_vertex_buffer(1, self.boid_buffers.get_target_buffer().slice(..));
                screen_render_pass.draw(0..3, 0..self.boids_count as _);
            });
        });

        self.simulation_profiler.resolve_queries(_encoder);

        Ok(())
    }

    fn post_render(&mut self, _app_state: &mut AppState) -> Result<()> {
        self.simulation_profiler.end_frame().unwrap();

        Ok(())
    }
}
