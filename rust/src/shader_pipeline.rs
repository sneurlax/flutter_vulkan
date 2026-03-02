use std::sync::Arc;

use crate::sampler2d::Sampler2D;
use crate::uniform_queue::{PushConstants, UniformQueue};

/// Manages the wgpu render pipeline for shader execution.
///
/// This is the Rust replacement for the C++ `Shader` class.
pub struct ShaderPipeline {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,

    pub width: u32,
    pub height: u32,
    pub is_continuous: bool,
    pub pipeline_valid: bool,
    start_time: f64,

    pub vertex_source: String,
    pub fragment_source: String,

    uniform_queue: UniformQueue,

    // wgpu pipeline resources
    render_pipeline: Option<wgpu::RenderPipeline>,
    pipeline_layout: Option<wgpu::PipelineLayout>,
    bind_group_layout: Option<wgpu::BindGroupLayout>,
    bind_group: Option<wgpu::BindGroup>,

    // Uniform buffer (replaces Vulkan push constants)
    uniform_buffer: Option<wgpu::Buffer>,

    // Target format for the render pipeline's fragment output.
    // Defaults to Rgba8Unorm (native offscreen), but must be set to the
    // surface format for web/surface rendering via `set_surface_format`.
    target_format: wgpu::TextureFormat,

    // Offscreen render target
    output_texture: Option<wgpu::Texture>,
    output_texture_view: Option<wgpu::TextureView>,

    // CPU readback buffer (native only)
    output_buffer: Option<wgpu::Buffer>,

    // Sampler textures used in the bind group (kept alive)
    sampler_textures: Vec<SamplerBinding>,
}

/// Holds the wgpu texture + view + sampler created for one iChannel slot.
struct SamplerBinding {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl ShaderPipeline {
    /// Create a new `ShaderPipeline` with the given dimensions.
    pub fn new(
        device: &Arc<wgpu::Device>,
        queue: &Arc<wgpu::Queue>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            device: Arc::clone(device),
            queue: Arc::clone(queue),
            width,
            height,
            is_continuous: true,
            pipeline_valid: false,
            start_time: 0.0,
            vertex_source: String::new(),
            fragment_source: String::new(),
            uniform_queue: UniformQueue::new(),
            target_format: wgpu::TextureFormat::Rgba8Unorm,
            render_pipeline: None,
            pipeline_layout: None,
            bind_group_layout: None,
            bind_group: None,
            uniform_buffer: None,
            output_texture: None,
            output_texture_view: None,
            output_buffer: None,
            sampler_textures: Vec::new(),
        }
    }

    // ------------------------------------------------------------------ setters

    pub fn set_shaders_text(&mut self, vertex: &str, fragment: &str) {
        self.vertex_source = vertex.to_string();
        self.fragment_source = fragment.to_string();
    }

    pub fn set_shaders_size(&mut self, w: u32, h: u32) {
        self.width = w;
        self.height = h;
    }

    pub fn set_is_continuous(&mut self, b: bool) {
        self.is_continuous = b;
    }

    /// Set the texture format used for the render pipeline's fragment output.
    /// Must be called before `init_shader` / `init_shader_toy` to match the
    /// surface format on web.
    pub fn set_target_format(&mut self, format: wgpu::TextureFormat) {
        self.target_format = format;
    }

    // -------------------------------------------------------------- uniforms

    /// Add the standard ShaderToy uniforms (iMouse, iResolution, iTime,
    /// iChannel0-3 as 4x4 black textures).
    pub fn add_shader_toy_uniforms(&mut self) {
        use crate::uniform_queue::UniformValue;

        self.uniform_queue
            .add_uniform("iMouse", UniformValue::Vec4([0.0, 0.0, 0.0, 0.0]));
        self.uniform_queue.add_uniform(
            "iResolution",
            UniformValue::Vec3([self.width as f32, self.height as f32, 0.0]),
        );
        self.uniform_queue
            .add_uniform("iTime", UniformValue::Float(0.0));

        // 4x4 opaque-black RGBA texture
        let mut black_pixels = vec![0u8; 4 * 4 * 4];
        for i in (3..black_pixels.len()).step_by(4) {
            black_pixels[i] = 255; // alpha = 1
        }

        for name in &["iChannel0", "iChannel1", "iChannel2", "iChannel3"] {
            let mut sampler = Sampler2D::new();
            sampler.add_rgba32(4, 4, &black_pixels);
            self.uniform_queue
                .add_uniform(*name, UniformValue::Sampler2D(sampler));
        }
    }

    pub fn get_uniforms(&self) -> &UniformQueue {
        &self.uniform_queue
    }

    pub fn get_uniforms_mut(&mut self) -> &mut UniformQueue {
        &mut self.uniform_queue
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn is_continuous(&self) -> bool {
        self.is_continuous
    }

    // -------------------------------------------------------- GLSL -> WGSL

    /// Compile a GLSL source string to WGSL via naga.
    fn compile_glsl_to_wgsl(
        source: &str,
        stage: naga::ShaderStage,
    ) -> Result<String, String> {
        use naga::back::wgsl;
        use naga::front::glsl;
        use naga::valid::{Capabilities, ValidationFlags, Validator};

        let mut parser = glsl::Frontend::default();
        let options = glsl::Options {
            stage,
            defines: Default::default(),
        };

        let module = parser
            .parse(&options, source)
            .map_err(|errs| {
                let stage_name = match stage {
                    naga::ShaderStage::Vertex => "VERTEX",
                    naga::ShaderStage::Fragment => "FRAGMENT",
                    naga::ShaderStage::Compute => "COMPUTE",
                };
                format!(
                    "{stage_name} GLSL parse error:\n{}",
                    errs.errors.iter()
                        .map(|e| format!("{e}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })?;

        let info = Validator::new(ValidationFlags::all(), Capabilities::all())
            .validate(&module)
            .map_err(|e| format!("naga validation error: {e}"))?;

        let wgsl_source = wgsl::write_string(&module, &info, wgsl::WriterFlags::empty())
            .map_err(|e| format!("WGSL write error: {e}"))?;

        Ok(wgsl_source)
    }

    // ----------------------------------------------------- init_shader

    /// Compile the stored vertex/fragment GLSL sources, create the wgpu
    /// render pipeline and all associated resources.
    ///
    /// Returns an empty string on success or an error description.
    pub fn init_shader(&mut self) -> String {
        self.cleanup();

        // -- compile vertex shader ------------------------------------------------
        let vert_wgsl = match Self::compile_glsl_to_wgsl(
            &self.vertex_source,
            naga::ShaderStage::Vertex,
        ) {
            Ok(s) => s,
            Err(e) => return e,
        };

        // -- compile fragment shader ----------------------------------------------
        let frag_wgsl = match Self::compile_glsl_to_wgsl(
            &self.fragment_source,
            naga::ShaderStage::Fragment,
        ) {
            Ok(s) => s,
            Err(e) => return e,
        };

        // On WebGPU (WASM), Dawn/Tint enforces derivative uniformity analysis
        // which rejects textureSample / dpdx / dpdy in non-uniform control flow.
        // ShaderToy shaders commonly violate this.  The WGSL diagnostic directive
        // tells the validator to accept these calls everywhere.
        #[cfg(target_arch = "wasm32")]
        let frag_wgsl = format!("diagnostic(off, derivative_uniformity);\n{frag_wgsl}");

        log::info!("init_shader: vertex WGSL ({} bytes), fragment WGSL ({} bytes)", vert_wgsl.len(), frag_wgsl.len());
        log::debug!("VERTEX WGSL:\n{vert_wgsl}");
        log::debug!("FRAGMENT WGSL:\n{frag_wgsl}");

        let vert_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shader_pipeline_vertex"),
                source: wgpu::ShaderSource::Wgsl(vert_wgsl.into()),
            });

        let frag_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("shader_pipeline_fragment"),
                source: wgpu::ShaderSource::Wgsl(frag_wgsl.into()),
            });

        // -- bind group layout ----------------------------------------------------
        // Naga translates combined GLSL `sampler2D` into a (texture, sampler) pair.
        // With GLSL bindings 0-3 for the four combined image samplers the
        // generated WGSL will use bindings 0-7 (texture at even, sampler at odd).
        // Binding 4 in GLSL (the uniform buffer) becomes binding 8 in WGSL after
        // naga's splitting of the four combined samplers.
        //
        // Layout:
        //   0  texture_2d<f32>  (iChannel0)
        //   1  sampler          (iChannel0)
        //   2  texture_2d<f32>  (iChannel1)
        //   3  sampler          (iChannel1)
        //   4  texture_2d<f32>  (iChannel2)
        //   5  sampler          (iChannel2)
        //   6  texture_2d<f32>  (iChannel3)
        //   7  sampler          (iChannel3)
        //   8  uniform buffer   (PushConstants)

        let bind_group_layout =
            self.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("shader_pipeline_bgl"),
                    entries: &Self::bind_group_layout_entries(),
                });

        // -- pipeline layout ------------------------------------------------------
        let pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("shader_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        // -- render pipeline ------------------------------------------------------
        let render_pipeline =
            self.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("shader_pipeline"),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &vert_module,
                        entry_point: Some("main"),
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &frag_module,
                        entry_point: Some("main"),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: self.target_format,
                            blend: Some(wgpu::BlendState::REPLACE),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        unclipped_depth: false,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        // -- output texture (offscreen render target) -----------------------------
        let output_texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shader_pipeline_output"),
            size: wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let output_texture_view =
            output_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // -- readback buffer (native only) ----------------------------------------
        // On WASM we render directly to the surface, so no readback is needed.
        #[cfg(not(target_arch = "wasm32"))]
        let output_buffer = {
            let bytes_per_row = self.width * 4;
            let padded_bytes_per_row = (bytes_per_row + 255) & !255;
            let buffer_size = (padded_bytes_per_row as u64) * (self.height as u64);
            self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("shader_pipeline_readback"),
                size: buffer_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        // -- uniform buffer -------------------------------------------------------
        let uniform_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("shader_pipeline_uniforms"),
            size: std::mem::size_of::<PushConstants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // -- sampler textures + bind group ----------------------------------------
        self.uniform_queue
            .set_all_sampler2d(&self.device, &self.queue);

        let sampler_textures = self.create_sampler_bindings();

        let bind_group = self.create_bind_group(
            &bind_group_layout,
            &sampler_textures,
            &uniform_buffer,
        );

        // -- store everything -----------------------------------------------------
        self.render_pipeline = Some(render_pipeline);
        self.pipeline_layout = Some(pipeline_layout);
        self.bind_group_layout = Some(bind_group_layout);
        self.bind_group = Some(bind_group);
        self.uniform_buffer = Some(uniform_buffer);
        self.output_texture = Some(output_texture);
        self.output_texture_view = Some(output_texture_view);
        #[cfg(not(target_arch = "wasm32"))]
        { self.output_buffer = Some(output_buffer); }
        self.sampler_textures = sampler_textures;

        self.start_time = instant_now();
        self.pipeline_valid = true;

        log::info!(
            "wgpu pipeline created successfully ({}x{}, format={:?})",
            self.width,
            self.height,
            self.target_format,
        );

        String::new()
    }

    // --------------------------------------------------- init_shader_toy

    /// Generate the ShaderToy vertex + fragment wrapper, add uniforms, and
    /// call `init_shader`.
    pub fn init_shader_toy(&mut self) -> String {
        // Full-screen triangle vertex shader in WGSL (bypasses naga GLSL
        // conversion for the vertex stage since this is a fixed utility
        // shader).
        self.vertex_source = concat!(
            "#version 450\n",
            "void main() {\n",
            "    vec2 uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);\n",
            "    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);\n",
            "}\n",
        )
        .to_string();

        // Wrap the user's ShaderToy GLSL with header/footer.
        // naga does not support combined `uniform sampler2D` in GLSL, so we
        // use separated texture2D + sampler declarations and reconstruct
        // combined samplers via #define macros.
        let header = concat!(
            "#version 450\n",
            "layout(set=0, binding=8) uniform PushConstants {\n",
            "    vec4 iMouse;\n",
            "    vec3 iResolution;\n",
            "    float iTime;\n",
            "} pc;\n",
            "#define iMouse pc.iMouse\n",
            "#define iResolution pc.iResolution\n",
            "#define iTime pc.iTime\n",
            "layout(set=0, binding=0) uniform texture2D _t_iChannel0;\n",
            "layout(set=0, binding=1) uniform sampler _s_iChannel0;\n",
            "layout(set=0, binding=2) uniform texture2D _t_iChannel1;\n",
            "layout(set=0, binding=3) uniform sampler _s_iChannel1;\n",
            "layout(set=0, binding=4) uniform texture2D _t_iChannel2;\n",
            "layout(set=0, binding=5) uniform sampler _s_iChannel2;\n",
            "layout(set=0, binding=6) uniform texture2D _t_iChannel3;\n",
            "layout(set=0, binding=7) uniform sampler _s_iChannel3;\n",
            "#define iChannel0 sampler2D(_t_iChannel0, _s_iChannel0)\n",
            "#define iChannel1 sampler2D(_t_iChannel1, _s_iChannel1)\n",
            "#define iChannel2 sampler2D(_t_iChannel2, _s_iChannel2)\n",
            "#define iChannel3 sampler2D(_t_iChannel3, _s_iChannel3)\n",
            "layout(location=0) out vec4 fragColor;\n",
        );

        let footer = concat!(
            "\nvoid main() {\n",
            "    mainImage(fragColor, vec2(gl_FragCoord.x, iResolution.y - gl_FragCoord.y));\n",
            "    fragColor.a = 1.0;\n",
            "}\n",
        );

        self.fragment_source = format!("{header}{}{footer}", self.fragment_source);

        self.add_shader_toy_uniforms();
        self.init_shader()
    }

    // -------------------------------------------------------- draw_frame

    /// Render a single frame to the offscreen texture and copy the result
    /// to the readback buffer.
    pub fn draw_frame(&mut self) {
        if !self.pipeline_valid {
            return;
        }

        let pipeline = match self.render_pipeline.as_ref() {
            Some(p) => p,
            None => return,
        };
        let bind_group = match self.bind_group.as_ref() {
            Some(bg) => bg,
            None => return,
        };
        let output_view = match self.output_texture_view.as_ref() {
            Some(v) => v,
            None => return,
        };
        let output_tex = match self.output_texture.as_ref() {
            Some(t) => t,
            None => return,
        };
        let output_buf = match self.output_buffer.as_ref() {
            Some(b) => b,
            None => return,
        };
        let uniform_buf = match self.uniform_buffer.as_ref() {
            Some(b) => b,
            None => return,
        };

        // Update iTime
        let elapsed = instant_now() - self.start_time;
        self.uniform_queue.set_uniform_value(
            "iTime",
            crate::uniform_queue::UniformValue::Float(elapsed as f32),
        );

        // Build push constants from uniform queue
        let pc = self.build_push_constants();
        self.queue.write_buffer(uniform_buf, 0, pc.as_bytes());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("shader_pipeline_encoder"),
            });

        // Render pass
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shader_pipeline_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output_view,
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

            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, Some(bind_group), &[]);
            rpass.draw(0..3, 0..1);
        }

        // Copy texture to readback buffer
        let bytes_per_row = self.width * 4;
        // wgpu requires rows aligned to 256 bytes
        let padded_bytes_per_row = (bytes_per_row + 255) & !255;

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: output_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: output_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));
    }

    // ------------------------------------------------ draw_frame_to_view

    /// Render a single frame directly to the provided texture view (e.g. a
    /// surface texture).  Unlike `draw_frame`, this does **not** copy to the
    /// readback buffer — it is intended for the web path where the surface
    /// texture is presented directly.
    pub fn draw_frame_to_view(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
    ) {
        // Even if the pipeline is invalid, clear to magenta so we can
        // distinguish "pipeline broken" (magenta) from "no rendering" (black).
        let pipeline = match (self.pipeline_valid, self.render_pipeline.as_ref()) {
            (true, Some(p)) => Some(p),
            _ => None,
        };
        let bind_group = self.bind_group.as_ref();
        let uniform_buf = self.uniform_buffer.as_ref();

        // Upload push constants if we have a valid pipeline
        if let (Some(_), Some(ub)) = (pipeline, uniform_buf) {
            let pc = self.build_push_constants();
            queue.write_buffer(ub, 0, pc.as_bytes());
        }

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("shader_pipeline_encoder_surface"),
            });

        {
            // Use magenta clear so a broken pipeline is visually distinct from
            // "nothing rendered at all" (which would remain whatever the
            // browser's default is).
            let clear_color = if pipeline.is_some() {
                wgpu::Color::BLACK
            } else {
                wgpu::Color { r: 1.0, g: 0.0, b: 1.0, a: 1.0 }
            };

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shader_pipeline_pass_surface"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let (Some(p), Some(bg)) = (pipeline, bind_group) {
                rpass.set_pipeline(p);
                rpass.set_bind_group(0, Some(bg), &[]);
                rpass.draw(0..3, 0..1);
            }
        }

        queue.submit(std::iter::once(encoder.finish()));
    }

    // ------------------------------------------------------- read_pixels

    /// Map the readback buffer and return RGBA pixel data.
    ///
    /// This blocks until the GPU work has completed.  Returns `None` if the
    /// pipeline is not valid or if mapping fails.
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        let output_buf = self.output_buffer.as_ref()?;

        let bytes_per_row = self.width * 4;
        let padded_bytes_per_row = (bytes_per_row + 255) & !255;

        let buffer_slice = output_buf.slice(..);

        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        self.device.poll(wgpu::Maintain::Wait);

        match rx.recv() {
            Ok(Ok(())) => {}
            _ => return None,
        }

        let mapped = buffer_slice.get_mapped_range();

        // Remove row padding if present
        let pixels = if padded_bytes_per_row != bytes_per_row {
            let mut out = Vec::with_capacity((bytes_per_row * self.height) as usize);
            for row in 0..self.height {
                let start = (row * padded_bytes_per_row) as usize;
                let end = start + bytes_per_row as usize;
                out.extend_from_slice(&mapped[start..end]);
            }
            out
        } else {
            mapped.to_vec()
        };

        drop(mapped);
        output_buf.unmap();

        Some(pixels)
    }

    // ------------------------------------------------- refresh_textures

    /// Re-create sampler textures from the uniform queue and rebuild the
    /// bind group.
    pub fn refresh_textures(&mut self) {
        self.uniform_queue
            .set_all_sampler2d(&self.device, &self.queue);

        let bgl = match self.bind_group_layout.as_ref() {
            Some(l) => l,
            None => return,
        };
        let ub = match self.uniform_buffer.as_ref() {
            Some(b) => b,
            None => return,
        };

        let sampler_textures = self.create_sampler_bindings();
        let bind_group = self.create_bind_group(bgl, &sampler_textures, ub);

        self.sampler_textures = sampler_textures;
        self.bind_group = Some(bind_group);
    }

    // ---------------------------------------------------------- cleanup

    /// Release all GPU resources held by this pipeline.
    pub fn cleanup(&mut self) {
        self.pipeline_valid = false;

        self.bind_group = None;
        self.render_pipeline = None;
        self.pipeline_layout = None;
        self.bind_group_layout = None;
        self.uniform_buffer = None;

        if let Some(tex) = self.output_texture.take() {
            tex.destroy();
        }
        self.output_texture_view = None;

        if let Some(buf) = self.output_buffer.take() {
            buf.destroy();
        }

        self.sampler_textures.clear();
    }

    // ============================================================ private

    /// Build the bind group layout entries (bindings 0-8).
    fn bind_group_layout_entries() -> [wgpu::BindGroupLayoutEntry; 9] {
        let tex_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };

        let sampler_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };

        [
            tex_entry(0),
            sampler_entry(1),
            tex_entry(2),
            sampler_entry(3),
            tex_entry(4),
            sampler_entry(5),
            tex_entry(6),
            sampler_entry(7),
            // Binding 8: uniform buffer (PushConstants)
            wgpu::BindGroupLayoutEntry {
                binding: 8,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ]
    }

    /// Create placeholder `SamplerBinding`s for iChannel0-3 from the
    /// uniform queue's `Sampler2D` entries.  Falls back to a 1x1 black
    /// texture if the sampler has no wgpu resources yet.
    fn create_sampler_bindings(&self) -> Vec<SamplerBinding> {
        let channel_names = ["iChannel0", "iChannel1", "iChannel2", "iChannel3"];
        let mut bindings = Vec::with_capacity(4);

        for name in &channel_names {
            let maybe_sampler = self.uniform_queue.get_sampler2d(name);

            if let Some(s) = maybe_sampler {
                if let (Some(tv), Some(samp)) = (s.texture_view.as_ref(), s.sampler.as_ref()) {
                    // Re-use the Sampler2D's existing wgpu view/sampler by
                    // creating trivial bindings that reference the same GPU
                    // resources.  However wgpu bind groups take owned
                    // references so we need to create new view/sampler handles
                    // that point to the same underlying texture.
                    //
                    // Since Sampler2D already holds the texture alive we can
                    // simply create a new view from the existing texture.
                    let texture = s.texture.as_ref().unwrap();
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    // Re-use the same sampler descriptor.
                    let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                        label: Some("ichannel_sampler"),
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Linear,
                        ..Default::default()
                    });

                    // We need to keep a reference-counted texture alive.
                    // Create a dummy 1x1 texture just to satisfy SamplerBinding's
                    // field -- but we already have the real view from the
                    // Sampler2D's texture.
                    //
                    // Actually, re-create from the Sampler2D pixel data would
                    // be redundant.  We can store a dummy since the view is
                    // what matters for the bind group.  But we already created
                    // `view` from `texture` which is owned by the Sampler2D.
                    // To keep things simple and self-contained, create our own
                    // small copy.  The Sampler2D already cleared its CPU data
                    // after upload, so we just make a 1x1 fallback for the
                    // _texture field and use the real view/sampler.
                    //
                    // Better approach: just store the already-created view.
                    let _ = tv; // suppress unused warning
                    let _ = samp;

                    bindings.push(SamplerBinding {
                        _texture: self.create_fallback_texture(),
                        view,
                        sampler,
                    });
                    continue;
                }
            }

            // Fallback: 1x1 black texture
            let tex = self.create_fallback_texture();
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("fallback_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            bindings.push(SamplerBinding {
                _texture: tex,
                view,
                sampler,
            });
        }

        bindings
    }

    /// Create a 1x1 black RGBA texture.
    fn create_fallback_texture(&self) -> wgpu::Texture {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback_texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[0u8, 0, 0, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        tex
    }

    /// Create the bind group from sampler bindings and uniform buffer.
    fn create_bind_group(
        &self,
        layout: &wgpu::BindGroupLayout,
        samplers: &[SamplerBinding],
        uniform_buf: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        // Build entries: 0-7 are texture/sampler pairs, 8 is uniform buffer
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::with_capacity(9);

        for (i, sb) in samplers.iter().enumerate() {
            let tex_binding = (i as u32) * 2;
            let samp_binding = tex_binding + 1;
            entries.push(wgpu::BindGroupEntry {
                binding: tex_binding,
                resource: wgpu::BindingResource::TextureView(&sb.view),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: samp_binding,
                resource: wgpu::BindingResource::Sampler(&sb.sampler),
            });
        }

        entries.push(wgpu::BindGroupEntry {
            binding: 8,
            resource: uniform_buf.as_entire_binding(),
        });

        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shader_pipeline_bg"),
            layout,
            entries: &entries,
        })
    }

    /// Collect push-constant values from the uniform queue.
    fn build_push_constants(&self) -> PushConstants {
        self.uniform_queue.get_push_constants()
    }
}

// ------------------------------------------------------------------ helpers

/// Return the current time in seconds (monotonic where possible).
fn instant_now() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::SystemTime;
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() / 1000.0
    }
}
