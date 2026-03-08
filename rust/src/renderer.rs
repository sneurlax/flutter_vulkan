use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::gpu_context::GpuContext;
use crate::shader_pipeline::ShaderPipeline;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderThreadMessage {
    None,
    StopRenderer,
    NewShader,
    NewTexture,
}


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
    pub buffer: *mut u8,              // platform-owned RGBA pixel buffer
    /// Called after each frame is written to `buffer`.
    pub frame_callback: Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
    pub frame_callback_data: *mut std::ffi::c_void,
    pub is_shader_toy: bool,
    pub new_shader_vertex_source: Mutex<String>,
    pub new_shader_fragment_source: Mutex<String>,
    pub new_shader_is_continuous: AtomicBool,
}

// SAFETY: buffer only accessed from the render thread inside loop_fn.
unsafe impl Send for Renderer {}
unsafe impl Sync for Renderer {}


pub static RENDERER: std::sync::Mutex<Option<Box<Renderer>>> = std::sync::Mutex::new(None);


impl Renderer {
    /// `buffer` must point to `width * height * 4` bytes of platform-owned memory.
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

    pub fn stop(&self) {
        if let Ok(mut q) = self.msg.lock() {
            q.push(RenderThreadMessage::StopRenderer);
        }
    }

    pub fn is_looping(&self) -> bool {
        self.loop_running.load(Ordering::SeqCst)
    }

    pub fn get_frame_rate(&self) -> f64 {
        self.frame_rate
    }

    pub fn set_new_texture_msg(&self) {
        if let Ok(mut q) = self.msg.lock() {
            q.push(RenderThreadMessage::NewTexture);
        }
    }

    /// Set a new shader. Blocks until the render thread picks it up.
    /// Returns compile error (empty on success).
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

        self.msg_processed.store(false, Ordering::SeqCst);
        if let Ok(mut q) = self.msg.lock() {
            q.push(RenderThreadMessage::NewShader);
        }

        if self.is_looping() {
            while !self.msg_processed.load(Ordering::SeqCst) {
                std::thread::yield_now();
            }
        }

        self.compile_error.lock().unwrap().clone()
    }

    pub fn set_shader_toy(&mut self, fragment: &str) -> String {
        self.is_shader_toy = true;
        self.set_shader(true, "", fragment)
    }

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

    // render loop (runs on a spawned thread)

    pub fn loop_fn(&mut self) {
        log::info!("RENDERER: ENTERING LOOP");

        let mut frames: u32 = 0;
        self.frame_rate = 0.0;
        let mut start_fps = Instant::now();
        let mut start_draw = Instant::now();
        // ~100 FPS cap
        let max_fps_interval: f64 = 1.0 / 100.0;

        self.loop_running.store(true, Ordering::SeqCst);

        while self.loop_running.load(Ordering::SeqCst) {
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
                        .is_some_and(|s| s.is_continuous);

                    if !shader_continuous {
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

                    // FPS (EMA, 1s window)
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
