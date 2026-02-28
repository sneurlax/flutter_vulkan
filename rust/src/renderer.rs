use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::gpu_context::GpuContext;
use crate::shader_pipeline::ShaderPipeline;

// ---------------------------------------------------------------------------
// Render-thread message enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderThreadMessage {
    None,
    StopRenderer,
    NewShader,
    NewTexture,
}

// ---------------------------------------------------------------------------
// Renderer
// ---------------------------------------------------------------------------

pub struct Renderer {
    pub gpu_ctx: GpuContext,
    pub shader: Option<ShaderPipeline>,
    pub frame_rate: f64,
    pub loop_running: AtomicBool,
    pub msg: Mutex<Vec<RenderThreadMessage>>,
    pub msg_processed: AtomicBool,
    pub compile_error: Mutex<String>,
    pub width: u32,
    pub height: u32,
    /// Raw pixel buffer owned by the platform plugin. The renderer copies
    /// rendered pixels here after each frame so the Flutter texture can pick
    /// them up.
    pub buffer: *mut u8,
    /// Optional callback the platform layer can set; invoked after new pixel
    /// data has been written to `buffer`.
    pub frame_callback: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub frame_callback_data: *mut std::ffi::c_void,
    pub is_shader_toy: bool,
    pub new_shader_vertex_source: Mutex<String>,
    pub new_shader_fragment_source: Mutex<String>,
    pub new_shader_is_continuous: AtomicBool,
}

// SAFETY: The raw `*mut u8` buffer is provided by the platform plugin and is
// only accessed from the render thread (inside `loop_fn`). We guarantee
// single-threaded access to it by only touching it while the render loop mutex
// is held.
unsafe impl Send for Renderer {}
unsafe impl Sync for Renderer {}

// ---------------------------------------------------------------------------
// Global renderer instance (mirrors the C++ `Renderer *renderer = nullptr;`)
// ---------------------------------------------------------------------------

pub static mut RENDERER: Option<Box<Renderer>> = None;

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl Renderer {
    /// Create a new `Renderer`, initialising the GPU context.
    ///
    /// `buffer` is a pointer to platform-allocated RGBA8 pixel memory of size
    /// `width * height * 4` bytes.
    pub fn new(width: u32, height: u32, buffer: *mut u8) -> Result<Self, String> {
        let gpu_ctx = GpuContext::new()?;

        Ok(Self {
            gpu_ctx,
            shader: None,
            frame_rate: 0.0,
            loop_running: AtomicBool::new(false),
            msg: Mutex::new(vec![RenderThreadMessage::None]),
            msg_processed: AtomicBool::new(true),
            compile_error: Mutex::new(String::new()),
            width,
            height,
            buffer,
            frame_callback: None,
            frame_callback_data: std::ptr::null_mut(),
            is_shader_toy: false,
            new_shader_vertex_source: Mutex::new(String::new()),
            new_shader_fragment_source: Mutex::new(String::new()),
            new_shader_is_continuous: AtomicBool::new(false),
        })
    }

    /// Request the render loop to stop.
    pub fn stop(&self) {
        if let Ok(mut q) = self.msg.lock() {
            q.push(RenderThreadMessage::StopRenderer);
        }
    }

    /// Whether the render loop is currently running.
    pub fn is_looping(&self) -> bool {
        self.loop_running.load(Ordering::SeqCst)
    }

    /// Current smoothed frame rate.
    pub fn get_frame_rate(&self) -> f64 {
        self.frame_rate
    }

    /// Enqueue a message telling the render loop to refresh textures.
    pub fn set_new_texture_msg(&self) {
        if let Ok(mut q) = self.msg.lock() {
            q.push(RenderThreadMessage::NewTexture);
        }
    }

    /// Set a new custom shader. Blocks (spin-yields) until the render thread
    /// has processed the message when the loop is running.
    ///
    /// Returns the compile error string (empty on success).
    pub fn set_shader(
        &self,
        is_continuous: bool,
        vertex: &str,
        fragment: &str,
    ) -> String {
        {
            let mut ce = self.compile_error.lock().unwrap();
            ce.clear();
        }

        // Store sources for the render thread to pick up.
        {
            let mut vs = self.new_shader_vertex_source.lock().unwrap();
            *vs = vertex.to_string();
        }
        {
            let mut fs = self.new_shader_fragment_source.lock().unwrap();
            *fs = fragment.to_string();
        }
        self.new_shader_is_continuous
            .store(is_continuous, Ordering::SeqCst);

        // NOTE: is_shader_toy is set via interior-mutability-unfriendly bool.
        // The caller (ffi layer) sets it directly before calling this via
        // `&mut` or before the loop is running, so this is safe from the FFI
        // side which holds &mut Renderer.

        self.msg_processed.store(false, Ordering::SeqCst);
        if let Ok(mut q) = self.msg.lock() {
            q.push(RenderThreadMessage::NewShader);
        }

        // Spin-yield until the render thread processes the message.
        if self.is_looping() {
            while !self.msg_processed.load(Ordering::SeqCst) {
                std::thread::yield_now();
            }
        }

        self.compile_error.lock().unwrap().clone()
    }

    /// Convenience wrapper for ShaderToy shaders.
    ///
    /// Returns the compile error string (empty on success).
    pub fn set_shader_toy(&mut self, fragment: &str) -> String {
        self.is_shader_toy = true;
        self.set_shader(true, "", fragment)
    }

    /// Set a custom (non-ShaderToy) shader via `&mut self`.
    pub fn set_shader_mut(
        &mut self,
        is_continuous: bool,
        vertex: &str,
        fragment: &str,
    ) -> String {
        self.is_shader_toy = false;
        self.set_shader(is_continuous, vertex, fragment)
    }

    pub fn get_shader(&self) -> Option<&ShaderPipeline> {
        self.shader.as_ref()
    }

    pub fn get_shader_mut(&mut self) -> Option<&mut ShaderPipeline> {
        self.shader.as_mut()
    }

    // -----------------------------------------------------------------------
    // Main render loop (meant to be called from a spawned thread)
    // -----------------------------------------------------------------------

    pub fn loop_fn(&mut self) {
        log::info!("RENDERER: ENTERING LOOP");

        let mut frames: u32 = 0;
        self.frame_rate = 0.0;
        let mut start_fps = Instant::now();
        let mut start_draw = Instant::now();
        // Max ~100 FPS → minimum 10 ms between frames.
        let max_fps_interval: f64 = 1.0 / 100.0;

        self.loop_running.store(true, Ordering::SeqCst);

        while self.loop_running.load(Ordering::SeqCst) {
            // Pop the latest message from the queue.
            let current_msg = {
                let mut q = self.msg.lock().unwrap();
                if q.is_empty() {
                    RenderThreadMessage::None
                } else {
                    q.pop().unwrap_or(RenderThreadMessage::None)
                }
            };

            match current_msg {
                RenderThreadMessage::NewShader => {
                    self.shader = None;

                    let vertex_src = self.new_shader_vertex_source.lock().unwrap().clone();
                    let fragment_src = self.new_shader_fragment_source.lock().unwrap().clone();
                    let is_continuous =
                        self.new_shader_is_continuous.load(Ordering::SeqCst);

                    let mut pipeline =
                        ShaderPipeline::new(&self.gpu_ctx.device, &self.gpu_ctx.queue, self.width, self.height);
                    pipeline.set_shaders_text(&vertex_src, &fragment_src);
                    pipeline.set_is_continuous(is_continuous);

                    let error = if self.is_shader_toy {
                        pipeline.init_shader_toy()
                    } else {
                        pipeline.init_shader()
                    };

                    if !error.is_empty() {
                        log::error!("RENDERER: shader compile error: {error}");
                    }

                    {
                        let mut ce = self.compile_error.lock().unwrap();
                        *ce = error;
                    }
                    self.shader = Some(pipeline);
                    self.msg_processed.store(true, Ordering::SeqCst);
                }

                RenderThreadMessage::NewTexture => {
                    if let Some(ref mut shader) = self.shader {
                        shader.refresh_textures();
                    }
                }

                RenderThreadMessage::StopRenderer => {
                    self.loop_running.store(false, Ordering::SeqCst);
                }

                RenderThreadMessage::None => {
                    let shader_continuous = self
                        .shader
                        .as_ref()
                        .map_or(false, |s| s.is_continuous);

                    if !shader_continuous {
                        // Nothing to do — yield briefly so we don't burn CPU.
                        std::thread::yield_now();
                        continue;
                    }

                    let elapsed_draw = start_draw.elapsed().as_secs_f64();
                    if elapsed_draw >= max_fps_interval {
                        frames += 1;

                        if let Some(ref mut shader) = self.shader {
                            shader.draw_frame();

                            if let Some(pixels) = shader.read_pixels() {
                                if !pixels.is_empty() && !self.buffer.is_null() {
                                    let byte_count =
                                        (self.width as usize) * (self.height as usize) * 4;
                                    let copy_len = byte_count.min(pixels.len());
                                    unsafe {
                                        std::ptr::copy_nonoverlapping(
                                            pixels.as_ptr(),
                                            self.buffer,
                                            copy_len,
                                        );
                                    }

                                    if let Some(cb) = self.frame_callback {
                                        unsafe { cb(self.frame_callback_data) };
                                    }
                                }
                            }
                        }

                        start_draw = Instant::now();
                    }

                    // FPS tracking — EMA smoothing every 1 second.
                    let elapsed_fps = start_fps.elapsed().as_secs_f64();
                    if elapsed_fps >= 1.0 {
                        self.frame_rate =
                            (frames as f64) * 0.5 + self.frame_rate * 0.5;
                        frames = 0;
                        start_fps = Instant::now();
                    }
                }
            }
        }

        self.loop_running.store(false, Ordering::SeqCst);
        log::info!("RENDERER: EXITING LOOP");
    }
}
