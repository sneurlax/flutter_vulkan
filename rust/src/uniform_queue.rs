use std::collections::HashMap;

use bytemuck::{Pod, Zeroable};

use crate::sampler2d::Sampler2D;

// ---------------------------------------------------------------------------
// Uniform type discriminant (mirrors the C++ UniformType enum)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UniformType {
    Bool,
    Int,
    Float,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
    Sampler2D,
}

// ---------------------------------------------------------------------------
// Simple vec / mat value types
// ---------------------------------------------------------------------------

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];
pub type Mat2 = [f32; 4];
pub type Mat3 = [f32; 9];
pub type Mat4 = [f32; 16];

// ---------------------------------------------------------------------------
// UniformValue – a tagged union of all possible uniform payloads
// ---------------------------------------------------------------------------

pub enum UniformValue {
    Bool(bool),
    Int(i32),
    Float(f32),
    Vec2(Vec2),
    Vec3(Vec3),
    Vec4(Vec4),
    Mat2(Mat2),
    Mat3(Mat3),
    Mat4(Mat4),
    Sampler2D(Sampler2D),
}

impl UniformValue {
    /// Return the type discriminant for this value.
    pub fn uniform_type(&self) -> UniformType {
        match self {
            UniformValue::Bool(_) => UniformType::Bool,
            UniformValue::Int(_) => UniformType::Int,
            UniformValue::Float(_) => UniformType::Float,
            UniformValue::Vec2(_) => UniformType::Vec2,
            UniformValue::Vec3(_) => UniformType::Vec3,
            UniformValue::Vec4(_) => UniformType::Vec4,
            UniformValue::Mat2(_) => UniformType::Mat2,
            UniformValue::Mat3(_) => UniformType::Mat3,
            UniformValue::Mat4(_) => UniformType::Mat4,
            UniformValue::Sampler2D(_) => UniformType::Sampler2D,
        }
    }
}

// ---------------------------------------------------------------------------
// PushConstants – ShaderToy built-in uniforms, passed via push constants.
// Must be exactly 32 bytes and satisfy Pod requirements.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PushConstants {
    /// iMouse (vec4) – 16 bytes
    pub i_mouse: [f32; 4],
    /// iResolution (vec3) – 12 bytes
    pub i_resolution: [f32; 3],
    /// iTime (float) – 4 bytes
    pub i_time: f32,
}

// SAFETY: PushConstants is #[repr(C)], contains only f32 (which is Pod),
// has no padding (4*4 + 3*4 + 1*4 = 32 bytes), and every bit pattern is valid.
unsafe impl Zeroable for PushConstants {}
unsafe impl Pod for PushConstants {}

impl PushConstants {
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

impl Default for PushConstants {
    fn default() -> Self {
        Self {
            i_mouse: [0.0; 4],
            i_resolution: [0.0; 3],
            i_time: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// UniformQueue
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct UniformQueue {
    pub uniforms: HashMap<String, UniformValue>,
}


impl UniformQueue {
    pub fn new() -> Self {
        Self::default()
    }

    // -- basic CRUD --------------------------------------------------------

    /// Look up a uniform value by name.
    pub fn get_value(&self, name: &str) -> Option<&UniformValue> {
        self.uniforms.get(name)
    }

    /// Look up a Sampler2D uniform by name.
    pub fn get_sampler2d(&self, name: &str) -> Option<&Sampler2D> {
        match self.uniforms.get(name) {
            Some(UniformValue::Sampler2D(ref s)) => Some(s),
            _ => None,
        }
    }

    /// Add a uniform. Returns `false` if a uniform with `name` already exists.
    pub fn add_uniform(&mut self, name: impl Into<String>, value: UniformValue) -> bool {
        let name = name.into();
        if self.uniforms.contains_key(&name) {
            log::warn!("Uniform \"{}\" already exists!", name);
            return false;
        }
        self.uniforms.insert(name, value);
        true
    }

    /// Remove a uniform. Returns `false` if the name was not found.
    /// Automatically destroys wgpu resources for Sampler2D uniforms.
    pub fn remove_uniform(&mut self, name: &str) -> bool {
        match self.uniforms.remove(name) {
            Some(UniformValue::Sampler2D(mut s)) => {
                s.destroy();
                true
            }
            Some(_) => true,
            None => {
                log::warn!("Uniform \"{}\" doesn't exist!", name);
                false
            }
        }
    }

    /// Overwrite the value of an existing uniform.
    /// For Sampler2D uniforms the provided `value` must also be `Sampler2D`;
    /// alternatively, use `replace_sampler2d` for raw-data replacement.
    /// Returns `false` if the uniform was not found.
    pub fn set_uniform_value(&mut self, name: &str, value: UniformValue) -> bool {
        match self.uniforms.get_mut(name) {
            Some(slot) => {
                *slot = value;
                true
            }
            None => {
                log::warn!("Uniform \"{}\" not found!", name);
                false
            }
        }
    }

    // -- push constants ----------------------------------------------------

    /// Build a `PushConstants` struct from the stored iMouse, iResolution and
    /// iTime uniforms (falling back to zero when not present).
    pub fn get_push_constants(&self) -> PushConstants {
        let mut pc = PushConstants::default();

        if let Some(UniformValue::Vec4(v)) = self.uniforms.get("iMouse") {
            pc.i_mouse = *v;
        }
        if let Some(UniformValue::Vec3(v)) = self.uniforms.get("iResolution") {
            pc.i_resolution = *v;
        }
        if let Some(UniformValue::Float(v)) = self.uniforms.get("iTime") {
            pc.i_time = *v;
        }

        pc
    }

    // -- sampler2D helpers -------------------------------------------------

    /// Create wgpu textures for every Sampler2D uniform that has pixel data.
    /// Assigns sequential `n_texture` indices to samplers that don't have one
    /// yet (n_texture == -1).
    pub fn set_all_sampler2d(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let mut n: i32 = 0;
        for (_name, value) in self.uniforms.iter_mut() {
            if let UniformValue::Sampler2D(ref mut sampler) = value {
                if !sampler.data.is_empty() {
                    if sampler.n_texture == -1 {
                        sampler.n_texture = n;
                    }
                    sampler.create_wgpu_texture(device, queue);
                    n += 1;
                } else if sampler.n_texture >= 0 {
                    n = sampler.n_texture + 1;
                }
            }
        }
    }

    /// Replace the pixel data of an existing Sampler2D uniform.
    /// Returns `false` if the name is not found or the uniform is not a
    /// Sampler2D.
    pub fn replace_sampler2d(&mut self, name: &str, w: u32, h: u32, raw_data: &[u8]) -> bool {
        match self.uniforms.get_mut(name) {
            Some(UniformValue::Sampler2D(ref mut sampler)) => {
                sampler.replace_texture(w, h, raw_data);
                true
            }
            _ => false,
        }
    }

    /// Return references to every Sampler2D uniform that has a valid texture
    /// index and a created texture view, paired with its texture index.
    pub fn get_all_sampler2d_textures(&self) -> Vec<(i32, &Sampler2D)> {
        let mut result = Vec::new();
        for value in self.uniforms.values() {
            if let UniformValue::Sampler2D(ref sampler) = value {
                if sampler.n_texture >= 0 && sampler.texture_view.is_some() {
                    result.push((sampler.n_texture, sampler));
                }
            }
        }
        result
    }
}

impl Drop for UniformQueue {
    fn drop(&mut self) {
        for (_name, value) in self.uniforms.iter_mut() {
            if let UniformValue::Sampler2D(ref mut sampler) = value {
                sampler.destroy();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_get() {
        let mut q = UniformQueue::new();
        assert!(q.add_uniform("x", UniformValue::Float(1.5)));
        match q.get_value("x") {
            Some(UniformValue::Float(v)) => assert_eq!(*v, 1.5),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn add_duplicate_returns_false_and_preserves_original() {
        let mut q = UniformQueue::new();
        assert!(q.add_uniform("x", UniformValue::Int(1)));
        assert!(!q.add_uniform("x", UniformValue::Int(2)));
        match q.get_value("x") {
            Some(UniformValue::Int(v)) => assert_eq!(*v, 1),
            _ => panic!("expected Int(1)"),
        }
    }

    #[test]
    fn set_value() {
        let mut q = UniformQueue::new();
        q.add_uniform("t", UniformValue::Float(0.0));
        assert!(q.set_uniform_value("t", UniformValue::Float(3.14)));
        match q.get_value("t") {
            Some(UniformValue::Float(v)) => assert!((v - 3.14_f32).abs() < 1e-6),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn remove() {
        let mut q = UniformQueue::new();
        q.add_uniform("v", UniformValue::Vec2([1.0, 2.0]));
        assert!(q.remove_uniform("v"));
        assert!(q.get_value("v").is_none());
    }

    #[test]
    fn push_constants_from_uniforms() {
        let mut q = UniformQueue::new();
        q.add_uniform("iMouse",      UniformValue::Vec4([1.0, 2.0, 3.0, 4.0]));
        q.add_uniform("iResolution", UniformValue::Vec3([800.0, 600.0, 0.0]));
        q.add_uniform("iTime",       UniformValue::Float(42.0));

        let pc = q.get_push_constants();
        assert_eq!(pc.i_mouse,      [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(pc.i_resolution, [800.0, 600.0, 0.0]);
        assert_eq!(pc.i_time,       42.0);
    }

    #[test]
    fn push_constants_size_is_32_bytes() {
        // Changing this breaks the GPU UBO layout.
        assert_eq!(std::mem::size_of::<PushConstants>(), 32);
    }
}
