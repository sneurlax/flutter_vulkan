use wgpu;

/// Rust replacement for the C++ Sampler2D class.
///
/// Holds raw RGBA8 pixel data and optional wgpu resources
/// (texture, texture view, sampler) created from that data.
pub struct Sampler2D {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub n_texture: i32,

    // Optional wgpu resources – created by `create_wgpu_texture`.
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub sampler: Option<wgpu::Sampler>,
}

impl Default for Sampler2D {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            width: 0,
            height: 0,
            n_texture: -1,
            texture: None,
            texture_view: None,
            sampler: None,
        }
    }
}

impl Sampler2D {
    pub fn new() -> Self {
        Self::default()
    }

    /// Store raw RGBA8 pixel data (4 bytes per pixel).
    pub fn add_rgba32(&mut self, w: u32, h: u32, raw_data: &[u8]) {
        self.width = w;
        self.height = h;
        let size = (w * h * 4) as usize;
        self.data.resize(size, 0);
        self.data[..size].copy_from_slice(&raw_data[..size]);
    }

    /// Replace the stored pixel data (clears old data first).
    /// No-op if `n_texture` has not been assigned yet (== -1).
    pub fn replace_texture(&mut self, w: u32, h: u32, raw_data: &[u8]) {
        if self.n_texture == -1 {
            return;
        }
        self.data.clear();
        self.add_rgba32(w, h, raw_data);
    }

    /// Create a wgpu texture (+ view + sampler) from the stored pixel data.
    ///
    /// Uses a single mip level with linear filtering for simplicity
    /// (full mipmap generation in wgpu would require a compute/render pass).
    pub fn create_wgpu_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if self.data.is_empty() {
            return;
        }

        // Drop any previously created resources.
        self.destroy();

        let size = wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sampler2d_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Write pixel data to mip level 0.
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width),
                rows_per_image: Some(self.height),
            },
            size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sampler2d_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        self.texture = Some(texture);
        self.texture_view = Some(texture_view);
        self.sampler = Some(sampler);

        // Clear CPU-side data after upload (matches C++ behaviour).
        self.data.clear();
    }

    /// Drop wgpu resources (texture, view, sampler).
    pub fn destroy(&mut self) {
        // Dropping wgpu handles releases the underlying GPU resources.
        if let Some(tex) = self.texture.take() {
            tex.destroy();
        }
        self.texture_view = None;
        self.sampler = None;
    }
}
