use wgpu::util::DeviceExt;

pub struct Canvas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,

    // The permanent image
    pub pixel_buffer: Vec<u8>,
    // The temporary layer for the current stroke
    pub stroke_buffer: Vec<u8>,

    pub width: u32,
    pub height: u32,
}

impl Canvas {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> Self {
        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let pixel_count = (width * height) as usize;

        // 1. White Background
        let mut pixel_buffer = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            pixel_buffer.extend_from_slice(&[255, 255, 255, 255]);
        }

        // 2. Empty Stroke Buffer (Transparent)
        let mut stroke_buffer = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            stroke_buffer.extend_from_slice(&[0, 0, 0, 0]);
        }

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Canvas Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Initial Clear
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &pixel_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            pixel_buffer,
            stroke_buffer,
            width,
            height,
        }
    }

    /// Updates the GPU Texture by combining Main Layer + Stroke Layer
    pub fn update_texture(&self, queue: &wgpu::Queue) {
        // We need to composite the two buffers on the CPU before sending to GPU
        // (Optimized: In a real app, do this in a Compute Shader, but this is fine for now)
        let mut composited = self.pixel_buffer.clone();

        for i in (0..self.stroke_buffer.len()).step_by(4) {
            let src_a = self.stroke_buffer[i + 3];
            if src_a > 0 {
                // Alpha Blend Stroke onto Background for Display
                let alpha = src_a as f32 / 255.0;
                let inv_alpha = 1.0 - alpha;

                let dst_r = composited[i] as f32;
                let dst_g = composited[i + 1] as f32;
                let dst_b = composited[i + 2] as f32;

                let src_r = self.stroke_buffer[i] as f32;
                let src_g = self.stroke_buffer[i + 1] as f32;
                let src_b = self.stroke_buffer[i + 2] as f32;

                composited[i] = (src_r * alpha + dst_r * inv_alpha) as u8;
                composited[i + 1] = (src_g * alpha + dst_g * inv_alpha) as u8;
                composited[i + 2] = (src_b * alpha + dst_b * inv_alpha) as u8;
                composited[i + 3] = 255;
            }
        }

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &composited,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Draws to the temporary Stroke Buffer
    /// Logic: MAX ALPHA (Prevents dots from getting darker)
    pub fn draw_to_stroke(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = ((y * self.width + x) * 4) as usize;

        let current_a = self.stroke_buffer[i + 3];

        // MAGIC TRICK: Only update if the new alpha is higher than what's there.
        // This ensures overlapping segments don't add up, they just stay at the max opacity.
        if a > current_a {
            self.stroke_buffer[i] = r;
            self.stroke_buffer[i + 1] = g;
            self.stroke_buffer[i + 2] = b;
            self.stroke_buffer[i + 3] = a;
        }
    }

    /// Permanently bakes the stroke onto the main canvas
    pub fn commit_stroke(&mut self) {
        for i in (0..self.stroke_buffer.len()).step_by(4) {
            let src_a = self.stroke_buffer[i + 3];
            if src_a > 0 {
                // Standard Blend
                let alpha = src_a as f32 / 255.0;
                let inv_alpha = 1.0 - alpha;

                let bg_r = self.pixel_buffer[i] as f32;
                let bg_g = self.pixel_buffer[i + 1] as f32;
                let bg_b = self.pixel_buffer[i + 2] as f32;

                let src_r = self.stroke_buffer[i] as f32;
                let src_g = self.stroke_buffer[i + 1] as f32;
                let src_b = self.stroke_buffer[i + 2] as f32;

                self.pixel_buffer[i] = (src_r * alpha + bg_r * inv_alpha) as u8;
                self.pixel_buffer[i + 1] = (src_g * alpha + bg_g * inv_alpha) as u8;
                self.pixel_buffer[i + 2] = (src_b * alpha + bg_b * inv_alpha) as u8;
                self.pixel_buffer[i + 3] = 255;

                // Clear Stroke Buffer as we go
                self.stroke_buffer[i] = 0;
                self.stroke_buffer[i + 1] = 0;
                self.stroke_buffer[i + 2] = 0;
                self.stroke_buffer[i + 3] = 0;
            }
        }
    }
}
