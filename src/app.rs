use egui::{Color32, Id, LayerId, Order, Stroke, TextureHandle, TextureOptions};
use image::io::Reader as ImageReader;
use std::path::Path;
use winit::{event::*, window::Window};

use crate::canvas::Canvas;
use crate::commands::PaintCommand;
use crate::packages::PackageManager;
use crate::scripting::{CursorType, LuaEngine};

pub struct AppState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,

    canvas: Canvas,
    lua: LuaEngine,
    packages: PackageManager,

    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,

    mouse_pos: (f32, f32),
    last_mouse_pos: Option<(f32, f32)>,
    mouse_pressed: bool,
    brush_color: [f32; 3],
    // Removed: brush_size, antialiasing (Lua handles these now)
    active_cursor_texture: Option<TextureHandle>,
    active_tool_name: String,
}

impl AppState {
    pub async fn new(window: &Window) -> Self {
        // ... (WGPU boilerplate stays exactly the same) ...
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(window).unwrap())
        }
        .unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_capabilities(&adapter).formats[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface.get_capabilities(&adapter).alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let canvas = Canvas::new(&device, &queue, 800, 600);
        let mut packages = PackageManager::new();
        packages.load_packages();

        let mut lua = LuaEngine::new();
        let mut active_tool_name = "None".to_string();

        if let Some(first_tool) = packages.tools.first() {
            println!("Auto-loading tool: {}", first_tool.name);
            lua.load_tool(first_tool);
            active_tool_name = first_tool.name.clone();
        }

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: None,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&canvas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&canvas.sampler),
                },
            ],
            label: None,
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            bind_group,
            canvas,
            lua,
            packages,
            egui_ctx,
            egui_state,
            egui_renderer,
            mouse_pos: (0.0, 0.0),
            last_mouse_pos: None,
            mouse_pressed: false,
            brush_color: [0.0, 0.0, 0.0],
            // Removed size/aa defaults
            active_cursor_texture: None,
            active_tool_name,
        }
    }

    fn load_cursor_image(&mut self, path_str: &str) {
        let path = Path::new(path_str);
        if !path.exists() {
            return;
        }
        if let Ok(reader) = ImageReader::open(path) {
            if let Ok(img) = reader.decode() {
                let size = [img.width() as usize, img.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    size,
                    img.to_rgba8().as_flat_samples().as_slice(),
                );
                self.active_cursor_texture = Some(self.egui_ctx.load_texture(
                    "custom_cursor",
                    color_image,
                    TextureOptions::LINEAR,
                ));
            }
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn handle_window_event(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.egui_state.on_window_event(window, event);
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: element_state,
                button: MouseButton::Left,
                ..
            } => {
                self.mouse_pressed = *element_state == ElementState::Pressed;

                if !self.mouse_pressed {
                    // MOUSE RELEASED: Commit the stroke!
                    self.last_mouse_pos = None;
                    self.canvas.commit_stroke();
                    self.canvas.update_texture(&self.queue); // Update one last time to clear the preview
                }
            }
            _ => {}
        }
    }

    pub fn update(&mut self) {
        if self.egui_ctx.is_pointer_over_area() || self.egui_ctx.is_using_pointer() {
            return;
        }

        if self.mouse_pressed {
            let scale_x = self.canvas.width as f32 / self.size.width as f32;
            let scale_y = self.canvas.height as f32 / self.size.height as f32;
            let current_pos = self.mouse_pos;
            let start_pos = self.last_mouse_pos.unwrap_or(current_pos);

            let start_tex_x = (start_pos.0 * scale_x) as u32;
            let start_tex_y = (start_pos.1 * scale_y) as u32;
            let end_tex_x = (current_pos.0 * scale_x) as u32;
            let end_tex_y = (current_pos.1 * scale_y) as u32;

            let commands = self.lua.process_input(
                start_tex_x,
                start_tex_y,
                end_tex_x,
                end_tex_y,
                self.brush_color,
            );

            let mut dirty = false;
            for cmd in commands {
                match cmd {
                    PaintCommand::DrawPixel { x, y, r, g, b, a } => {
                        // CHANGE: Draw to temporary stroke buffer
                        self.canvas.draw_to_stroke(x, y, r, g, b, a);
                        dirty = true;
                    }
                }
            }

            // CHANGE: Update texture (which now composites Stroke + Main)
            if dirty {
                self.canvas.update_texture(&self.queue);
            }

            self.last_mouse_pos = Some(current_pos);
        }
    }

    pub fn render(&mut self, window: &Window) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        let raw_input = self.egui_state.take_egui_input(window);
        let ctx = self.egui_ctx.clone();

        let full_output = ctx.run(raw_input, |ctx| {
            egui::Window::new("Tools").show(ctx, |ui| {
                ui.heading("Pixle");
                ui.label(format!("Active: {}", self.active_tool_name));
                ui.separator();

                // 1. Draw Tool Selector
                let tool_count = self.packages.tools.len();
                for i in 0..tool_count {
                    let tool_name = self.packages.tools[i].name.clone();
                    if ui.button(&tool_name).clicked() {
                        let tool = self.packages.tools[i].clone();
                        self.lua.load_tool(&tool);
                        self.active_tool_name = tool_name;
                        match self.lua.get_current_cursor() {
                            CursorType::SystemCircle => self.active_cursor_texture = None,
                            CursorType::CustomImage(path) => self.load_cursor_image(&path),
                        }
                    }
                }
                ui.separator();

                // 2. Global Color Picker (Managed by Rust, but could be Lua)
                ui.label("Global Color");
                ui.color_edit_button_rgb(&mut self.brush_color);
                ui.separator();

                // 3. Lua Defined UI
                // This replaces the hardcoded sliders
                self.lua.draw_ui(ui);
            });

            if !ctx.is_pointer_over_area() {
                let painter =
                    ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("cursor_overlay")));
                let mouse_pos = egui::Pos2 {
                    x: self.mouse_pos.0,
                    y: self.mouse_pos.1,
                };

                if let Some(texture) = &self.active_cursor_texture {
                    let size = texture.size_vec2();
                    let rect = egui::Rect::from_center_size(mouse_pos, size);
                    painter.image(
                        texture.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    // Update: Ask Lua for the size
                    let lua_size = self.lua.get_tool_size();
                    let zoom_ratio = self.size.width as f32 / self.canvas.width as f32;
                    let visual_radius = (lua_size / 2.0) * zoom_ratio;
                    painter.circle_stroke(
                        mouse_pos,
                        visual_radius,
                        Stroke::new(1.0, Color32::WHITE),
                    );
                    painter.circle_stroke(
                        mouse_pos,
                        visual_radius - 1.0,
                        Stroke::new(1.0, Color32::BLACK),
                    );
                }
            }
        });

        self.egui_state
            .handle_platform_output(window, full_output.platform_output);
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };
        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_desc,
        );
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.egui_renderer
                .render(&mut render_pass, &clipped_primitives, &screen_desc);
        }
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
