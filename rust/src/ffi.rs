//! C FFI layer — exports the public API consumed by the Flutter platform
//! plugins. Mirrors the original `ffi.h` / `ffi.cpp` signatures.

use std::ffi::{c_char, c_void, CStr, CString};
use crate::renderer::{Renderer, RENDERER};
use crate::sampler2d::Sampler2D;
use crate::uniform_queue::{UniformType, UniformValue};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Thread-local storage for CStrings returned to callers. Keeps the pointer
/// valid until the next call from the same thread.
thread_local! {
    static RETURN_STRING: std::cell::RefCell<CString> =
        std::cell::RefCell::new(CString::default());
}

/// Store `s` in thread-local storage and return a `*const c_char` that remains
/// valid until the next call to this function on the same thread.
fn return_c_str(s: &str) -> *const c_char {
    let cs = CString::new(s).unwrap_or_default();
    RETURN_STRING.with(|cell| {
        let mut slot = cell.borrow_mut();
        *slot = cs;
        slot.as_ptr()
    })
}

/// Obtain a `&mut Renderer` from the global, or return `None`.
#[inline]
unsafe fn renderer_mut() -> Option<&'static mut Renderer> {
    unsafe { RENDERER.as_deref_mut() }
}

/// Obtain a `&Renderer` from the global, or return `None`.
#[inline]
unsafe fn renderer_ref() -> Option<&'static Renderer> {
    unsafe { RENDERER.as_deref() }
}

// ---------------------------------------------------------------------------
// Renderer lifecycle
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn createRenderer(buffer: *mut u8, width: i32, height: i32) {
    let _ = env_logger::try_init();

    // Tear down any previous renderer first.
    deleteRenderer();

    match Renderer::new(width as u32, height as u32, buffer) {
        Ok(r) => unsafe {
            RENDERER = Some(Box::new(r));
            log::info!("NATIVE FFI: createRenderer OK ({}x{})", width, height);
        },
        Err(e) => {
            log::error!("NATIVE FFI: createRenderer failed: {e}");
        }
    }
}

#[no_mangle]
pub extern "C" fn deleteRenderer() {
    unsafe {
        if let Some(ref r) = RENDERER {
            if r.is_looping() {
                r.stop();
                // Spin until the loop exits.
                while r.is_looping() {
                    std::thread::yield_now();
                }
            }
        }
        RENDERER = None;
    }
}

#[no_mangle]
pub extern "C" fn getRenderer() -> *mut c_void {
    unsafe {
        match RENDERER {
            Some(ref mut r) => r.as_mut() as *mut Renderer as *mut c_void,
            None => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
pub extern "C" fn rendererStatus() -> i32 {
    unsafe { if RENDERER.is_some() { 1 } else { 0 } }
}

#[no_mangle]
pub extern "C" fn getTextureSize(width: *mut i32, height: *mut i32) {
    unsafe {
        match renderer_ref().and_then(|r| r.get_shader()) {
            Some(shader) => {
                *width = shader.get_width() as i32;
                *height = shader.get_height() as i32;
            }
            None => {
                *width = -1;
                *height = -1;
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn startThread() {
    unsafe {
        if RENDERER.is_none() {
            log::warn!("NATIVE FFI: startThread: Renderer not yet created!");
            return;
        }

        let addr = renderer_mut().unwrap() as *mut Renderer as usize;
        std::thread::spawn(move || {
            let ptr = addr as *mut Renderer;
            (*ptr).loop_fn();
        });
    }
}

#[no_mangle]
pub extern "C" fn stopThread() {
    unsafe {
        match renderer_ref() {
            Some(r) => {
                r.stop();
                while r.is_looping() {
                    std::thread::yield_now();
                }
            }
            None => {
                log::warn!("NATIVE FFI: stopThread: Renderer not yet created!");
            }
        }
    }
}

/// Set a callback that the renderer invokes after writing a new frame to the
/// pixel buffer.  The platform plugin uses this to mark the Flutter texture
/// frame as available.
#[no_mangle]
pub extern "C" fn setFrameCallback(
    callback: unsafe extern "C" fn(*mut c_void),
    user_data: *mut c_void,
) {
    unsafe {
        if let Some(r) = renderer_mut() {
            r.frame_callback = Some(callback);
            r.frame_callback_data = user_data;
        }
    }
}

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn setShader(
    is_continuous: i32,
    vertex: *const c_char,
    fragment: *const c_char,
) -> *const c_char {
    unsafe {
        let r = match renderer_mut() {
            Some(r) => r,
            None => {
                log::warn!("NATIVE FFI: setShader: Renderer not yet created!");
                return return_c_str("");
            }
        };

        let v = if vertex.is_null() {
            ""
        } else {
            CStr::from_ptr(vertex).to_str().unwrap_or("")
        };
        let f = if fragment.is_null() {
            ""
        } else {
            CStr::from_ptr(fragment).to_str().unwrap_or("")
        };

        r.is_shader_toy = false;
        let err = r.set_shader(is_continuous != 0, v, f);
        return_c_str(&err)
    }
}

#[no_mangle]
pub extern "C" fn setShaderToy(fragment: *const c_char) -> *const c_char {
    unsafe {
        let r = match renderer_mut() {
            Some(r) => r,
            None => {
                log::warn!("NATIVE FFI: setShaderToy: Renderer not yet created!");
                return return_c_str("");
            }
        };

        let f = if fragment.is_null() {
            ""
        } else {
            CStr::from_ptr(fragment).to_str().unwrap_or("")
        };

        let err = r.set_shader_toy(f);
        return_c_str(&err)
    }
}

#[no_mangle]
pub extern "C" fn getVertexShader() -> *const c_char {
    unsafe {
        match renderer_ref().and_then(|r| r.get_shader()) {
            Some(shader) => return_c_str(&shader.vertex_source),
            None => return_c_str(""),
        }
    }
}

#[no_mangle]
pub extern "C" fn getFragmentShader() -> *const c_char {
    unsafe {
        match renderer_ref().and_then(|r| r.get_shader()) {
            Some(shader) => return_c_str(&shader.fragment_source),
            None => return_c_str(""),
        }
    }
}

#[no_mangle]
pub extern "C" fn addShaderToyUniforms() {
    unsafe {
        if let Some(shader) = renderer_mut().and_then(|r| r.get_shader_mut()) {
            shader.add_shader_toy_uniforms();
        }
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn setMousePosition(
    posX: f64,
    posY: f64,
    posZ: f64,
    posW: f64,
    textureWidgetWidth: f64,
    textureWidgetHeight: f64,
) {
    unsafe {
        let r = match renderer_mut() {
            Some(r) => r,
            None => return,
        };
        let shader = match r.get_shader_mut() {
            Some(s) => s,
            None => return,
        };

        let tex_w = shader.get_width() as f64;
        let tex_h = shader.get_height() as f64;
        let ar_h = tex_w / textureWidgetWidth;
        let ar_v = tex_h / textureWidgetHeight;

        let mx = (posX * ar_h) as f32;
        let my = (tex_h - posY * ar_v) as f32;
        let mz = (posZ * ar_h) as f32;
        let mw = (-tex_h - posW * ar_v) as f32;

        let mouse = [mx, my, mz, mw];
        shader
            .get_uniforms_mut()
            .set_uniform_value("iMouse", UniformValue::Vec4(mouse));
    }
}

#[no_mangle]
pub extern "C" fn getFPS() -> f64 {
    unsafe {
        match renderer_ref() {
            Some(r) if r.is_looping() => r.get_frame_rate(),
            _ => -1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Uniforms
// ---------------------------------------------------------------------------

/// Read a uniform value from a raw `*const c_void` pointer according to
/// `uniform_type` (matches C++ `UniformType` enum ordinals).
///
/// Type mapping:
///   0 = Bool, 1 = Int, 2 = Float, 3 = Vec2, 4 = Vec3, 5 = Vec4,
///   6 = Mat2, 7 = Mat3, 8 = Mat4
unsafe fn read_uniform_value(uniform_type: i32, val: *const c_void) -> Option<UniformValue> {
    if val.is_null() {
        return None;
    }
    match uniform_type {
        0 => {
            let v = *(val as *const bool);
            Some(UniformValue::Bool(v))
        }
        1 => {
            let v = *(val as *const i32);
            Some(UniformValue::Int(v))
        }
        2 => {
            let v = *(val as *const f32);
            Some(UniformValue::Float(v))
        }
        3 => {
            let v = *(val as *const [f32; 2]);
            Some(UniformValue::Vec2(v))
        }
        4 => {
            let v = *(val as *const [f32; 3]);
            Some(UniformValue::Vec3(v))
        }
        5 => {
            let v = *(val as *const [f32; 4]);
            Some(UniformValue::Vec4(v))
        }
        6 => {
            let v = *(val as *const [f32; 4]);
            Some(UniformValue::Mat2(v))
        }
        7 => {
            let v = *(val as *const [f32; 9]);
            Some(UniformValue::Mat3(v))
        }
        8 => {
            let v = *(val as *const [f32; 16]);
            Some(UniformValue::Mat4(v))
        }
        _ => {
            log::warn!("NATIVE FFI: addUniform: unknown uniform type {uniform_type}");
            None
        }
    }
}

#[no_mangle]
pub extern "C" fn addUniform(
    name: *const c_char,
    uniform_type: i32,
    val: *const c_void,
) -> i32 {
    unsafe {
        let shader = match renderer_mut().and_then(|r| r.get_shader_mut()) {
            Some(s) => s,
            None => return 0,
        };

        let name_str = if name.is_null() {
            return 0;
        } else {
            match CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => return 0,
            }
        };

        let value = match read_uniform_value(uniform_type, val) {
            Some(v) => v,
            None => return 0,
        };

        if shader
            .get_uniforms_mut()
            .add_uniform(name_str.to_string(), value) { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn removeUniform(name: *const c_char) -> i32 {
    unsafe {
        let shader = match renderer_mut().and_then(|r| r.get_shader_mut()) {
            Some(s) => s,
            None => return 0,
        };

        let name_str = if name.is_null() {
            return 0;
        } else {
            match CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => return 0,
            }
        };

        if shader.get_uniforms_mut().remove_uniform(name_str) { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn setUniform(name: *const c_char, val: *const c_void) -> i32 {
    unsafe {
        let shader = match renderer_mut().and_then(|r| r.get_shader_mut()) {
            Some(s) => s,
            None => return 0,
        };

        let name_str = if name.is_null() {
            return 0;
        } else {
            match CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => return 0,
            }
        };

        // We need to figure out the existing uniform type to know how to read
        // the value. Look it up in the queue.
        let existing_type = match shader.get_uniforms().uniforms.get(name_str) {
            Some(existing) => existing.uniform_type(),
            None => {
                log::warn!("NATIVE FFI: setUniform: uniform \"{name_str}\" not found");
                return 0;
            }
        };

        let type_id = match existing_type {
            UniformType::Bool => 0,
            UniformType::Int => 1,
            UniformType::Float => 2,
            UniformType::Vec2 => 3,
            UniformType::Vec3 => 4,
            UniformType::Vec4 => 5,
            UniformType::Mat2 => 6,
            UniformType::Mat3 => 7,
            UniformType::Mat4 => 8,
            UniformType::Sampler2D => {
                log::warn!(
                    "NATIVE FFI: setUniform: use replaceSampler2DUniform for sampler2D"
                );
                return 0;
            }
        };

        let value = match read_uniform_value(type_id, val) {
            Some(v) => v,
            None => return 0,
        };

        if shader
            .get_uniforms_mut()
            .set_uniform_value(name_str, value) { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn addSampler2DUniform(
    name: *const c_char,
    width: i32,
    height: i32,
    val: *const c_void,
) -> i32 {
    unsafe {
        let r = match renderer_mut() {
            Some(r) => r,
            None => return 0,
        };
        let is_looping = r.is_looping();

        let shader = match r.get_shader_mut() {
            Some(s) => s,
            None => return 0,
        };

        let name_str = if name.is_null() {
            return 0;
        } else {
            match CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => return 0,
            }
        };

        let mut sampler = Sampler2D::new();
        if !val.is_null() && width > 0 && height > 0 {
            let byte_len = (width as usize) * (height as usize) * 4;
            let slice = std::slice::from_raw_parts(val as *const u8, byte_len);
            sampler.add_rgba32(width as u32, height as u32, slice);
        }

        let added = shader
            .get_uniforms_mut()
            .add_uniform(name_str.to_string(), UniformValue::Sampler2D(sampler));

        if added && is_looping {
            if let Some(rr) = renderer_ref() {
                rr.set_new_texture_msg();
            }
        }
        if added { 1 } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn replaceSampler2DUniform(
    name: *const c_char,
    width: i32,
    height: i32,
    val: *const c_void,
) -> i32 {
    unsafe {
        let r = match renderer_mut() {
            Some(r) => r,
            None => return 0,
        };
        let is_looping = r.is_looping();

        let shader = match r.get_shader_mut() {
            Some(s) => s,
            None => return 0,
        };

        let name_str = if name.is_null() {
            return 0;
        } else {
            match CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => return 0,
            }
        };

        let byte_len = (width as usize) * (height as usize) * 4;
        let slice = if val.is_null() || byte_len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(val as *const u8, byte_len)
        };

        let replaced = shader
            .get_uniforms_mut()
            .replace_sampler2d(name_str, width as u32, height as u32, slice);

        if replaced && is_looping {
            if let Some(rr) = renderer_ref() {
                rr.set_new_texture_msg();
            }
        }
        if replaced { 1 } else { 0 }
    }
}
