//! C FFI layer — exports the public API consumed by the Flutter platform
//! plugins. Mirrors the original `ffi.h` / `ffi.cpp` signatures.

use std::ffi::{c_char, c_void, CStr, CString};
use crate::renderer::{Renderer, RENDERER};
use crate::sampler2d::Sampler2D;
use crate::uniform_queue::{UniformType, UniformValue};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// Thread-local storage for CStrings returned to callers. Keeps the pointer
// valid until the next call from the same thread.
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

/// Read a uniform value from a raw `*const c_void` pointer according to
/// `uniform_type` (matches C++ `UniformType` enum ordinals).
///
/// Type mapping:
///   0 = Bool, 1 = Int, 2 = Float, 3 = Vec2, 4 = Vec3, 5 = Vec4,
///   6 = Mat2, 7 = Mat3, 8 = Mat4
///
/// # Safety
///
/// `val` must be non-null and point to memory of the correct size and
/// alignment for the given `uniform_type`, valid for the duration of the call.
unsafe fn read_uniform_value(uniform_type: i32, val: *const c_void) -> Option<UniformValue> {
    if val.is_null() {
        return None;
    }
    match uniform_type {
        0 => Some(UniformValue::Bool(*(val as *const bool))),
        1 => Some(UniformValue::Int(*(val as *const i32))),
        2 => Some(UniformValue::Float(*(val as *const f32))),
        3 => Some(UniformValue::Vec2(*(val as *const [f32; 2]))),
        4 => Some(UniformValue::Vec3(*(val as *const [f32; 3]))),
        5 => Some(UniformValue::Vec4(*(val as *const [f32; 4]))),
        6 => Some(UniformValue::Mat2(*(val as *const [f32; 4]))),
        7 => Some(UniformValue::Mat3(*(val as *const [f32; 9]))),
        8 => Some(UniformValue::Mat4(*(val as *const [f32; 16]))),
        _ => {
            log::warn!("NATIVE FFI: addUniform: unknown uniform type {uniform_type}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Renderer lifecycle
// ---------------------------------------------------------------------------

/// # Safety
///
/// `buffer` must be a valid pointer to at least `width * height * 4` bytes of
/// RGBA8 pixel memory, remaining valid for the lifetime of the renderer.
#[no_mangle]
pub unsafe extern "C" fn createRenderer(buffer: *mut u8, width: i32, height: i32) {
    #[cfg(target_os = "android")]
    {
        let _ = android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("FlutterVulkanRust"),
        );
    }
    #[cfg(not(target_os = "android"))]
    let _ = env_logger::try_init();

    // Tear down any previous renderer first (inline deleteRenderer logic to
    // avoid double-locking the mutex).
    {
        let mut guard = RENDERER.lock().unwrap();
        if let Some(ref r) = *guard {
            if r.is_looping() {
                r.stop();
                while r.is_looping() {
                    std::thread::yield_now();
                }
            }
        }
        *guard = None;
    }

    match Renderer::new(width as u32, height as u32, buffer) {
        Ok(r) => {
            *RENDERER.lock().unwrap() = Some(Box::new(r));
            log::info!("NATIVE FFI: createRenderer OK ({}x{})", width, height);
        }
        Err(e) => {
            log::error!("NATIVE FFI: createRenderer failed: {e}");
        }
    }
}

#[no_mangle]
pub extern "C" fn deleteRenderer() {
    let mut guard = RENDERER.lock().unwrap();
    if let Some(ref r) = *guard {
        if r.is_looping() {
            r.stop();
            while r.is_looping() {
                std::thread::yield_now();
            }
        }
    }
    *guard = None;
}

#[no_mangle]
pub extern "C" fn getRenderer() -> *mut c_void {
    match RENDERER.lock().unwrap().as_deref_mut() {
        Some(r) => r as *mut Renderer as *mut c_void,
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn rendererStatus() -> i32 {
    if RENDERER.lock().unwrap().is_some() { 1 } else { 0 }
}

/// # Safety
///
/// `width` and `height` must be non-null, valid, aligned `*mut i32` pointers
/// that live for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn getTextureSize(width: *mut i32, height: *mut i32) {
    let guard = RENDERER.lock().unwrap();
    match guard.as_deref().and_then(|r| r.get_shader()) {
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

#[no_mangle]
pub extern "C" fn startThread() {
    let mut guard = RENDERER.lock().unwrap();
    if guard.is_none() {
        log::warn!("NATIVE FFI: startThread: Renderer not yet created!");
        return;
    }
    let addr = guard.as_deref_mut().unwrap() as *mut Renderer as usize;
    drop(guard);
    std::thread::spawn(move || {
        // SAFETY: addr points to the Renderer inside the Box<Renderer> owned
        // by RENDERER. We only spawn one loop thread and the FFI contract
        // guarantees deleteRenderer joins before dropping RENDERER.
        unsafe { (*(addr as *mut Renderer)).loop_fn() };
    });
}

#[no_mangle]
pub extern "C" fn stopThread() {
    let guard = RENDERER.lock().unwrap();
    match guard.as_deref() {
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

/// Set a callback that the renderer invokes after writing a new frame to the
/// pixel buffer.  The platform plugin uses this to mark the Flutter texture
/// frame as available.
///
/// # Safety
///
/// `user_data` must remain valid (and not be freed) for the entire lifetime
/// of the renderer, or until a new callback is registered.
#[no_mangle]
pub unsafe extern "C" fn setFrameCallback(
    callback: unsafe extern "C" fn(*mut c_void),
    user_data: *mut c_void,
) {
    if let Some(r) = RENDERER.lock().unwrap().as_deref_mut() {
        r.frame_callback = Some(callback);
        r.frame_callback_data = user_data;
    }
}

// ---------------------------------------------------------------------------
// Shaders
// ---------------------------------------------------------------------------

/// # Safety
///
/// `vertex` and `fragment` must each be either null or a valid NUL-terminated
/// C string, valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn setShader(
    is_continuous: i32,
    vertex: *const c_char,
    fragment: *const c_char,
) -> *const c_char {
    let mut guard = RENDERER.lock().unwrap();
    let r = match guard.as_deref_mut() {
        Some(r) => r,
        None => {
            log::warn!("NATIVE FFI: setShader: Renderer not yet created!");
            return return_c_str("");
        }
    };

    let v = if vertex.is_null() { "" } else { CStr::from_ptr(vertex).to_str().unwrap_or("") };
    let f = if fragment.is_null() { "" } else { CStr::from_ptr(fragment).to_str().unwrap_or("") };

    r.is_shader_toy = false;
    let err = r.set_shader(is_continuous != 0, v, f);
    return_c_str(&err)
}

/// # Safety
///
/// `fragment` must be either null or a valid NUL-terminated C string, valid
/// for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn setShaderToy(fragment: *const c_char) -> *const c_char {
    let mut guard = RENDERER.lock().unwrap();
    let r = match guard.as_deref_mut() {
        Some(r) => r,
        None => {
            log::warn!("NATIVE FFI: setShaderToy: Renderer not yet created!");
            return return_c_str("");
        }
    };

    let f = if fragment.is_null() { "" } else { CStr::from_ptr(fragment).to_str().unwrap_or("") };

    let err = r.set_shader_toy(f);
    return_c_str(&err)
}

#[no_mangle]
pub extern "C" fn getVertexShader() -> *const c_char {
    match RENDERER.lock().unwrap().as_deref().and_then(|r| r.get_shader()) {
        Some(shader) => return_c_str(&shader.vertex_source),
        None => return_c_str(""),
    }
}

#[no_mangle]
pub extern "C" fn getFragmentShader() -> *const c_char {
    match RENDERER.lock().unwrap().as_deref().and_then(|r| r.get_shader()) {
        Some(shader) => return_c_str(&shader.fragment_source),
        None => return_c_str(""),
    }
}

#[no_mangle]
pub extern "C" fn addShaderToyUniforms() {
    if let Some(shader) = RENDERER.lock().unwrap().as_deref_mut().and_then(|r| r.get_shader_mut()) {
        shader.add_shader_toy_uniforms();
    }
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn setMousePosition(
    pos_x: f64,
    pos_y: f64,
    pos_z: f64,
    pos_w: f64,
    texture_widget_width: f64,
    texture_widget_height: f64,
) {
    let mut guard = RENDERER.lock().unwrap();
    let r = match guard.as_deref_mut() {
        Some(r) => r,
        None => return,
    };
    let shader = match r.get_shader_mut() {
        Some(s) => s,
        None => return,
    };

    let tex_w = shader.get_width() as f64;
    let tex_h = shader.get_height() as f64;
    let ar_h = tex_w / texture_widget_width;
    let ar_v = tex_h / texture_widget_height;

    let mouse = [
        (pos_x * ar_h) as f32,
        (tex_h - pos_y * ar_v) as f32,
        (pos_z * ar_h) as f32,
        (-tex_h - pos_w * ar_v) as f32,
    ];
    shader.get_uniforms_mut().set_uniform_value("iMouse", UniformValue::Vec4(mouse));
}

#[no_mangle]
pub extern "C" fn getFPS() -> f64 {
    match RENDERER.lock().unwrap().as_deref() {
        Some(r) if r.is_looping() => r.get_frame_rate(),
        _ => -1.0,
    }
}

// ---------------------------------------------------------------------------
// Uniforms
// ---------------------------------------------------------------------------

/// # Safety
///
/// `name` must be a valid NUL-terminated C string. `val` must be non-null and
/// point to correctly sized, aligned memory for the given `uniform_type`,
/// valid for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn addUniform(
    name: *const c_char,
    uniform_type: i32,
    val: *const c_void,
) -> i32 {
    let mut guard = RENDERER.lock().unwrap();
    let shader = match guard.as_deref_mut().and_then(|r| r.get_shader_mut()) {
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

    if shader.get_uniforms_mut().add_uniform(name_str.to_string(), value) { 1 } else { 0 }
}

/// # Safety
///
/// `name` must be a valid NUL-terminated C string, valid for the duration of
/// the call.
#[no_mangle]
pub unsafe extern "C" fn removeUniform(name: *const c_char) -> i32 {
    let mut guard = RENDERER.lock().unwrap();
    let shader = match guard.as_deref_mut().and_then(|r| r.get_shader_mut()) {
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

/// # Safety
///
/// `name` must be a valid NUL-terminated C string. `val` must point to memory
/// of the correct size for the uniform's existing type, valid for the duration
/// of the call.
#[no_mangle]
pub unsafe extern "C" fn setUniform(name: *const c_char, val: *const c_void) -> i32 {
    let mut guard = RENDERER.lock().unwrap();
    let shader = match guard.as_deref_mut().and_then(|r| r.get_shader_mut()) {
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
            log::warn!("NATIVE FFI: setUniform: use replaceSampler2DUniform for sampler2D");
            return 0;
        }
    };

    let value = match read_uniform_value(type_id, val) {
        Some(v) => v,
        None => return 0,
    };

    if shader.get_uniforms_mut().set_uniform_value(name_str, value) { 1 } else { 0 }
}

/// # Safety
///
/// `name` must be a valid NUL-terminated C string. `val`, if non-null, must
/// point to at least `width * height * 4` bytes of RGBA8 pixel data, valid
/// for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn addSampler2DUniform(
    name: *const c_char,
    width: i32,
    height: i32,
    val: *const c_void,
) -> i32 {
    let mut guard = RENDERER.lock().unwrap();
    let r = match guard.as_deref_mut() {
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

    let added = shader.get_uniforms_mut().add_uniform(name_str.to_string(), UniformValue::Sampler2D(sampler));
    if added && is_looping {
        r.set_new_texture_msg();
    }
    if added { 1 } else { 0 }
}

/// # Safety
///
/// `name` must be a valid NUL-terminated C string. `val`, if non-null, must
/// point to at least `width * height * 4` bytes of RGBA8 pixel data, valid
/// for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn replaceSampler2DUniform(
    name: *const c_char,
    width: i32,
    height: i32,
    val: *const c_void,
) -> i32 {
    let mut guard = RENDERER.lock().unwrap();
    let r = match guard.as_deref_mut() {
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

    let replaced = shader.get_uniforms_mut().replace_sampler2d(name_str, width as u32, height as u32, slice);
    if replaced && is_looping {
        r.set_new_texture_msg();
    }
    if replaced { 1 } else { 0 }
}
