use std::sync::Arc;

use wgpu::{Adapter, Device, Instance, Queue};

/// Core GPU state (instance, adapter, device, queue).
#[derive(Clone)]
pub struct GpuContext {
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

// Assert Send + Sync at compile time.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<GpuContext>();
    assert_sync::<GpuContext>();
};

impl GpuContext {
    /// Blocking init (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Result<Self, String> {
        pollster::block_on(Self::new_async())
    }

    /// Async init (all targets).
    pub async fn new_async() -> Result<Self, String> {
        let backends = if cfg!(target_os = "android") {
            wgpu::Backends::VULKAN | wgpu::Backends::GL
        } else {
            wgpu::Backends::all()
        };

        let instance = Instance::new(&wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await;

        // Fallback if high-perf adapter unavailable
        let adapter = match adapter {
            Some(a) => a,
            None => {
                log::warn!("No high-performance adapter found, trying fallback...");
                instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::LowPower,
                        force_fallback_adapter: false,
                        compatible_surface: None,
                    })
                    .await
                    .ok_or_else(|| "failed to find a suitable GPU adapter".to_string())?
            }
        };

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("flutter_vulkan_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| format!("failed to open GPU device: {e}"))?;

        log::info!(
            "GpuContext initialised — adapter: {:?}, backend: {:?}",
            adapter.get_info().name,
            adapter.get_info().backend,
        );

        Ok(Self {
            instance: Arc::new(instance),
            adapter: Arc::new(adapter),
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }
}
