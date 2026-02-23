pub mod gpu_context;
pub mod shader_pipeline;
pub mod uniform_queue;
pub mod sampler2d;

#[cfg(not(target_arch = "wasm32"))]
pub mod renderer;
#[cfg(not(target_arch = "wasm32"))]
pub mod ffi;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
pub use ffi::*;
