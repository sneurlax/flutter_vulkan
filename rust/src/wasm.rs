use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::Performance;

use crate::gpu_context::GpuContext;
use crate::sampler2d::Sampler2D;
use crate::shader_pipeline::ShaderPipeline;
use crate::uniform_queue::{UniformType, UniformValue};

// ---------------------------------------------------------------------------
// Thread-local renderer state
// ---------------------------------------------------------------------------

struct WasmState {
    gpu: GpuContext,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: Option<ShaderPipeline>,
    width: u32,
    height: u32,
    running: bool,
    frame_count: u64,
    fps: f64,
    last_fps_instant: f64,
    start_time: f64,
}

thread_local! {
    static STATE: RefCell<Option<WasmState>> = RefCell::new(None);
    static ANIMATION_CLOSURE: RefCell<Option<Closure<dyn FnMut()>>> = RefCell::new(None);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn performance() -> Performance {
    web_sys::window().unwrap().performance().unwrap()
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .unwrap();
}

fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut WasmState) -> R,
{
    STATE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let state = borrow.as_mut().expect("Renderer not initialised");
        f(state)
    })
}

fn decode_uniform(uniform_type: i32, val: &[u8]) -> Option<UniformValue> {
    match uniform_type {
        0 => {
            if val.is_empty() { return None; }
            Some(UniformValue::Bool(val[0] != 0))
        }
        1 => {
            let bytes: [u8; 4] = val.get(..4)?.try_into().ok()?;
            Some(UniformValue::Int(i32::from_ne_bytes(bytes)))
        }
        2 => {
            let bytes: [u8; 4] = val.get(..4)?.try_into().ok()?;
            Some(UniformValue::Float(f32::from_ne_bytes(bytes)))
        }
        3 => Some(UniformValue::Vec2(read_f32_array::<2>(val)?)),
        4 => Some(UniformValue::Vec3(read_f32_array::<3>(val)?)),
        5 => Some(UniformValue::Vec4(read_f32_array::<4>(val)?)),
        6 => Some(UniformValue::Mat2(read_f32_array::<4>(val)?)),
        7 => Some(UniformValue::Mat3(read_f32_array::<9>(val)?)),
        8 => Some(UniformValue::Mat4(read_f32_array::<16>(val)?)),
        _ => None,
    }
}

fn read_f32_array<const N: usize>(val: &[u8]) -> Option<[f32; N]> {
    let byte_len = N * 4;
    if val.len() < byte_len { return None; }
    let mut arr = [0.0f32; N];
    for i in 0..N {
        let start = i * 4;
        let bytes: [u8; 4] = val[start..start + 4].try_into().ok()?;
        arr[i] = f32::from_ne_bytes(bytes);
    }
    Some(arr)
}

// ---------------------------------------------------------------------------
// Exported functions
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn init_renderer(canvas_id: &str, width: u32, height: u32) -> js_sys::Promise {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);

    let canvas_id = canvas_id.to_owned();
    wasm_bindgen_futures::future_to_promise(async move {
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(&canvas_id)
            .ok_or_else(|| JsValue::from_str(&format!("Canvas '{}' not found", canvas_id)))?
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| JsValue::from_str(&format!("Failed to create surface: {e}")))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or_else(|| JsValue::from_str("Failed to find a suitable GPU adapter"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("flutter_vulkan_wasm_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to open GPU device: {e}")))?;

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let gpu = GpuContext {
            instance: std::sync::Arc::new(instance),
            adapter: std::sync::Arc::new(adapter),
            device: std::sync::Arc::new(device),
            queue: std::sync::Arc::new(queue),
        };

        let now = performance().now();

        STATE.with(|cell| {
            *cell.borrow_mut() = Some(WasmState {
                gpu,
                surface,
                surface_config,
                pipeline: None,
                width,
                height,
                running: false,
                frame_count: 0,
                fps: 0.0,
                last_fps_instant: now,
                start_time: now,
            });
        });

        log::info!("wasm renderer initialised ({}x{})", width, height);
        Ok(JsValue::TRUE)
    })
}

#[wasm_bindgen]
pub fn start_render_loop() {
    with_state(|s| {
        s.running = true;
        s.start_time = performance().now();
    });

    let closure: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let closure_clone = closure.clone();

    *closure.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let should_continue = STATE.with(|cell| {
            let mut borrow = cell.borrow_mut();
            let state = match borrow.as_mut() {
                Some(s) => s,
                None => return false,
            };
            if !state.running { return false; }

            // Update iTime
            let now = performance().now();
            let elapsed = ((now - state.start_time) / 1000.0) as f32;
            if let Some(ref mut pipeline) = state.pipeline {
                pipeline.get_uniforms_mut()
                    .set_uniform_value("iTime", UniformValue::Float(elapsed));

                // Draw to surface
                if let Ok(output) = state.surface.get_current_texture() {
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    // For web, we render directly to the surface texture
                    // by temporarily swapping the output texture view
                    pipeline.draw_frame_to_view(&state.gpu.device, &state.gpu.queue, &view);
                    output.present();
                }
            }

            // FPS bookkeeping
            state.frame_count += 1;
            let now = performance().now();
            let delta = now - state.last_fps_instant;
            if delta >= 1000.0 {
                state.fps = (state.frame_count as f64) / (delta / 1000.0);
                state.frame_count = 0;
                state.last_fps_instant = now;
            }

            true
        });

        if should_continue {
            if let Some(ref cb) = *closure_clone.borrow() {
                request_animation_frame(cb);
            }
        } else {
            ANIMATION_CLOSURE.with(|c| { *c.borrow_mut() = None; });
        }
    }) as Box<dyn FnMut()>));

    if let Some(ref cb) = *closure.borrow() {
        request_animation_frame(cb);
    }

    ANIMATION_CLOSURE.with(|c| {
        *c.borrow_mut() = closure.borrow_mut().take();
    });
}

#[wasm_bindgen]
pub fn stop_render_loop() {
    with_state(|s| s.running = false);
}

#[wasm_bindgen]
pub fn set_shader_toy(fragment_src: &str) -> String {
    with_state(|s| {
        let mut pipeline = ShaderPipeline::new(&s.gpu.device, &s.gpu.queue, s.width, s.height);
        pipeline.set_shaders_text("", fragment_src);
        pipeline.set_is_continuous(true);
        let err = pipeline.init_shader_toy();
        s.pipeline = Some(pipeline);
        err
    })
}

#[wasm_bindgen]
pub fn set_shader(is_continuous: bool, vertex_src: &str, fragment_src: &str) -> String {
    with_state(|s| {
        let mut pipeline = ShaderPipeline::new(&s.gpu.device, &s.gpu.queue, s.width, s.height);
        pipeline.set_shaders_text(vertex_src, fragment_src);
        pipeline.set_is_continuous(is_continuous);
        let err = pipeline.init_shader();
        s.pipeline = Some(pipeline);
        err
    })
}

#[wasm_bindgen]
pub fn get_fps() -> f64 {
    STATE.with(|cell| {
        cell.borrow().as_ref().map(|s| s.fps).unwrap_or(0.0)
    })
}

#[wasm_bindgen]
pub fn set_mouse_position(
    pos_x: f64, pos_y: f64, pos_z: f64, pos_w: f64,
    _tw_width: f64, _tw_height: f64,
) {
    with_state(|s| {
        if let Some(ref mut pipeline) = s.pipeline {
            pipeline.get_uniforms_mut().set_uniform_value(
                "iMouse",
                UniformValue::Vec4([pos_x as f32, pos_y as f32, pos_z as f32, pos_w as f32]),
            );
        }
    });
}

#[wasm_bindgen]
pub fn add_shader_toy_uniforms() {
    with_state(|s| {
        if let Some(ref mut pipeline) = s.pipeline {
            pipeline.add_shader_toy_uniforms();
        }
    });
}

#[wasm_bindgen]
pub fn add_uniform(name: &str, uniform_type: i32, val: &[u8]) -> bool {
    let value = match decode_uniform(uniform_type, val) {
        Some(v) => v,
        None => return false,
    };
    with_state(|s| {
        match s.pipeline.as_mut() {
            Some(p) => p.get_uniforms_mut().add_uniform(name.to_owned(), value),
            None => false,
        }
    })
}

#[wasm_bindgen]
pub fn remove_uniform(name: &str) -> bool {
    with_state(|s| {
        match s.pipeline.as_mut() {
            Some(p) => p.get_uniforms_mut().remove_uniform(name),
            None => false,
        }
    })
}

#[wasm_bindgen]
pub fn set_uniform(name: &str, val: &[u8]) -> bool {
    with_state(|s| {
        let pipeline = match s.pipeline.as_mut() {
            Some(p) => p,
            None => return false,
        };
        let utype = match pipeline.get_uniforms().get_value(name) {
            Some(v) => v.uniform_type(),
            None => return false,
        };
        let type_id = match utype {
            UniformType::Bool => 0, UniformType::Int => 1, UniformType::Float => 2,
            UniformType::Vec2 => 3, UniformType::Vec3 => 4, UniformType::Vec4 => 5,
            UniformType::Mat2 => 6, UniformType::Mat3 => 7, UniformType::Mat4 => 8,
            UniformType::Sampler2D => return false,
        };
        match decode_uniform(type_id, val) {
            Some(v) => pipeline.get_uniforms_mut().set_uniform_value(name, v),
            None => false,
        }
    })
}

#[wasm_bindgen]
pub fn add_sampler2d_uniform(name: &str, width: i32, height: i32, val: &[u8]) -> bool {
    with_state(|s| {
        let pipeline = match s.pipeline.as_mut() {
            Some(p) => p,
            None => return false,
        };
        let mut sampler = Sampler2D::new();
        sampler.add_rgba32(width as u32, height as u32, val);
        pipeline.get_uniforms_mut()
            .add_uniform(name.to_owned(), UniformValue::Sampler2D(sampler))
    })
}

#[wasm_bindgen]
pub fn replace_sampler2d_uniform(name: &str, width: i32, height: i32, val: &[u8]) -> bool {
    with_state(|s| {
        match s.pipeline.as_mut() {
            Some(p) => p.get_uniforms_mut()
                .replace_sampler2d(name, width as u32, height as u32, val),
            None => false,
        }
    })
}
