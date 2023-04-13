use oxyde::wgpu_utils::uniform_buffer::UniformBufferWrapper;
use oxyde::wgpu;

use wgpu_profiler::{wgpu_profiler, GpuProfiler};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SortingParams {
    h: u32,
}
pub struct SortingBoids {
    shader: wgpu::ShaderModule,
    local_bms_pipeline: wgpu::ComputePipeline,
    big_flip_pipeline: wgpu::ComputePipeline,
    local_disperse_pipeline: wgpu::ComputePipeline,
    big_disperse_pipeline: wgpu::ComputePipeline,
    sorting_params: UniformBufferWrapper<SortingParams>,
}

impl SortingBoids {
    pub fn new(
        device: &wgpu::Device,
        sorting_shader_source: wgpu::ShaderSource,
        boid_buffers_layout: &wgpu::BindGroupLayout,
    ) -> Self {

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Sorting compute shader"),
            source: sorting_shader_source,
        });

        let sorting_params = UniformBufferWrapper::new(device, SortingParams { h: 1 }, wgpu::ShaderStages::COMPUTE);

        let layout_descriptor = wgpu::PipelineLayoutDescriptor {
            label: Some("Local Bitonic Merge Sorting Pipeline Layout"),
            bind_group_layouts: &[sorting_params.layout(), boid_buffers_layout],
            push_constant_ranges: &[],
        };

        let layout = device.create_pipeline_layout(&layout_descriptor);

        let local_bms_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Local Bitonic Merge Sorting Pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: "cs_local_bms",
        });

        let big_flip_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Big Flip Pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: "cs_big_flip",
        });

        let local_disperse_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Local Disperse Pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: "cs_local_disperse",
        });

        let big_disperse_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Big Disperse Pipeline"),
            layout: Some(&layout),
            module: &shader,
            entry_point: "cs_big_disperse",
        });


        Self {
            shader,
            local_bms_pipeline,
            big_flip_pipeline,
            local_disperse_pipeline,
            big_disperse_pipeline,
            sorting_params,
        }
    }

    fn update_h(&mut self, queue: &wgpu::Queue, h: u32) {
        self.sorting_params.content().h = h;
        self.sorting_params.update_content(queue);
    }

    fn dispatch_pipeline(
        encoder: &mut wgpu::CommandEncoder,
        label : &str,
        pipeline: &wgpu::ComputePipeline,
        boid_bind_group: &wgpu::BindGroup,
        sorting_params_bind_group: &wgpu::BindGroup,
        dispatch_group_count: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some(&format!("{} Pass", label)),
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, sorting_params_bind_group, &[]);
        pass.set_bind_group(1, boid_bind_group, &[]);
        pass.dispatch_workgroups(dispatch_group_count, 1, 1);
    }

    pub fn sort_boids(
        &mut self,
        device: &wgpu::Device,
        queue: &mut wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        boid_buffers: &wgpu::BindGroup,
        workgroup_size: u32,
        boids_count: u32,
        profiler: &mut GpuProfiler) {
        let dispatch_group_count: u32 = std::cmp::max(1, boids_count / (workgroup_size * 2));

        let mut h = workgroup_size * 2;
        
        println!("dispatch_group_count: {}, boids_count: {},  workgroup_size: {}", dispatch_group_count, boids_count, workgroup_size);

        println!("Local Bitonic Merge Sorting h: {}", h);
        
        wgpu_profiler!("Sorting Boids", profiler, encoder, device, {
            self.update_h(queue, h);
            Self::dispatch_pipeline(
                encoder,
                "Local Bitonic Merge Sorting",
                &self.local_bms_pipeline,
                boid_buffers,
                &self.sorting_params.bind_group(),
                dispatch_group_count,
            );

            h *= 2;
            while h <= boids_count {

                println!("Big Flip h: {}", h);
                self.update_h(queue, h);
                Self::dispatch_pipeline(
                    encoder,
                    "Big Flip",
                    &self.big_flip_pipeline,
                    boid_buffers,
                    &self.sorting_params.bind_group(),
                    dispatch_group_count,
                );

                let mut hh = h / 2;
                
                while hh > 1 {

                    println!("Disperse h: {}", hh);
                    self.update_h(queue, hh);
                    Self::dispatch_pipeline(
                        encoder,
                        "Disperse",
                        if hh <= workgroup_size * 2 { &self.local_disperse_pipeline } else { &self.big_disperse_pipeline},
                        boid_buffers,
                        &self.sorting_params.bind_group(),
                        dispatch_group_count,
                    );

                    hh /= 2;
                }
        
                h *= 2;
            }

        });

    }
}
