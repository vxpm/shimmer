use crate::{context::Context, vram::Vram};
use encase::{ShaderType, StorageBuffer};
use glam::UVec2;
use shimmer::gpu::interface::{CopyFromVram, CopyInVram, CopyToVram};
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[derive(Debug, Clone, ShaderType)]
struct Config {
    source: UVec2,
    destination: UVec2,
    dimensions: UVec2,
    check_mask: u32,
}

pub struct Transfers {
    ctx: Arc<Context>,

    vram_bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    vram_to_cpu_pipeline: wgpu::ComputePipeline,
    cpu_to_vram_pipeline: wgpu::ComputePipeline,
    vram_to_vram_pipeline: wgpu::ComputePipeline,

    check_mask: bool,
}

impl Transfers {
    pub fn new(ctx: Arc<Context>, vram: &Vram) -> Self {
        let shader = ctx
            .device()
            .create_shader_module(wgpu::include_wgsl!("../shaders/built/transfer.wgsl"));

        let transfers_bind_group_layout =
            ctx.device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("transfers data"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            ctx.device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[vram.bind_group_layout(), &transfers_bind_group_layout],
                    push_constant_ranges: &[],
                });

        let vram_to_cpu_pipeline =
            ctx.device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("transfers"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("transfer_from_vram_to_buffer"),
                    compilation_options: Default::default(),
                    cache: None,
                });

        let cpu_to_vram_pipeline =
            ctx.device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("transfers"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("transfer_from_buffer_to_vram"),
                    compilation_options: Default::default(),
                    cache: None,
                });

        let vram_to_vram_pipeline =
            ctx.device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("transfers"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("transfer_from_vram_to_vram"),
                    compilation_options: Default::default(),
                    cache: None,
                });

        Self {
            ctx,

            vram_bind_group: vram.bind_group().clone(),
            bind_group_layout: transfers_bind_group_layout,
            vram_to_cpu_pipeline,
            cpu_to_vram_pipeline,
            vram_to_vram_pipeline,

            check_mask: false,
        }
    }

    pub fn set_check_mask(&mut self, value: bool) {
        self.check_mask = value;
    }

    pub fn copy_from_vram(&mut self, copy: CopyFromVram) {
        // create config
        let config = Config {
            source: UVec2::new(
                u32::from(copy.coords.x.value()),
                u32::from(copy.coords.y.value()),
            ),
            destination: UVec2::ZERO,
            dimensions: UVec2::new(
                u32::from(copy.dimensions.width.value()),
                u32::from(copy.dimensions.height.value()),
            ),
            check_mask: false as u32,
        };

        let mut data = StorageBuffer::new(Vec::new());
        data.write(&config).unwrap();

        let config = self
            .ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transfer"),
                usage: wgpu::BufferUsages::STORAGE,
                contents: &data.into_inner(),
            });

        // create buffer
        let buffer = self.ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("transfer"),
            size: 8
                * u64::from(copy.dimensions.width.value())
                * u64::from(copy.dimensions.height.value()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // bind group
        let transfer_bind_group = self
            .ctx
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("transfer data"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &config,
                            offset: 0,
                            size: None,
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
            });

        // transfer
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("transfer"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.vram_to_cpu_pipeline);
        pass.set_bind_group(0, &self.vram_bind_group, &[]);
        pass.set_bind_group(1, &transfer_bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);

        std::mem::drop(pass);
        self.ctx.queue().submit([encoder.finish()]);

        // get data back!
        wgpu::util::DownloadBuffer::read_buffer(
            self.ctx.device(),
            self.ctx.queue(),
            &buffer.slice(..),
            |result| {
                let buffer = result.unwrap();
                let bytes = &*buffer;
                let actual_data = bytes.iter().copied().step_by(4).collect::<Vec<_>>();
                copy.response.send(actual_data).unwrap();
            },
        );

        self.ctx.device().poll(wgpu::Maintain::Wait);
    }

    pub fn copy_to_vram(&mut self, copy: &CopyToVram) {
        // create config
        let config = Config {
            source: UVec2::new(
                u32::from(copy.coords.x.value()),
                u32::from(copy.coords.y.value()),
            ),
            destination: UVec2::ZERO,
            dimensions: UVec2::new(
                u32::from(copy.dimensions.width.value()),
                u32::from(copy.dimensions.height.value()),
            ),
            check_mask: self.check_mask as u32,
        };

        let mut data = StorageBuffer::new(Vec::new());
        data.write(&config).unwrap();

        let config = self
            .ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transfer"),
                usage: wgpu::BufferUsages::STORAGE,
                contents: &data.into_inner(),
            });

        let padded = copy
            .data
            .iter()
            .copied()
            .flat_map(|value| u32::from(value).to_le_bytes())
            .collect::<Vec<_>>();

        // create buffer
        let buffer = self
            .ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transfer"),
                contents: &padded,
                usage: wgpu::BufferUsages::STORAGE,
            });

        // bind group
        let transfer_bind_group = self
            .ctx
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("transfer data"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &config,
                            offset: 0,
                            size: None,
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
            });

        // transfer
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("transfer"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.cpu_to_vram_pipeline);
        pass.set_bind_group(0, &self.vram_bind_group, &[]);
        pass.set_bind_group(1, &transfer_bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);

        std::mem::drop(pass);
        self.ctx.queue().submit([encoder.finish()]);
    }

    pub fn copy_in_vram(&mut self, copy: &CopyInVram) {
        // create config
        let config = Config {
            source: UVec2::new(
                u32::from(copy.source.x.value()),
                u32::from(copy.source.y.value()),
            ),
            destination: UVec2::new(
                u32::from(copy.destination.x.value()),
                u32::from(copy.destination.y.value()),
            ),
            dimensions: UVec2::new(
                u32::from(copy.dimensions.width.value()),
                u32::from(copy.dimensions.height.value()),
            ),
            check_mask: self.check_mask as u32,
        };

        let mut data = StorageBuffer::new(Vec::new());
        data.write(&config).unwrap();

        let config = self
            .ctx
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("transfer"),
                usage: wgpu::BufferUsages::STORAGE,
                contents: &data.into_inner(),
            });

        // create buffer
        let buffer = self.ctx.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("transfer"),
            usage: wgpu::BufferUsages::STORAGE,
            size: 1,
            mapped_at_creation: false,
        });

        // bind group
        let transfer_bind_group = self
            .ctx
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("transfer data"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &config,
                            offset: 0,
                            size: None,
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
            });

        // transfer
        let mut encoder = self
            .ctx
            .device()
            .create_command_encoder(&Default::default());

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("transfer"),
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.vram_to_vram_pipeline);
        pass.set_bind_group(0, &self.vram_bind_group, &[]);
        pass.set_bind_group(1, &transfer_bind_group, &[]);
        pass.dispatch_workgroups(1, 1, 1);

        std::mem::drop(pass);
        self.ctx.queue().submit([encoder.finish()]);
    }
}
