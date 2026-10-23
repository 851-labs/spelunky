//! Adapter and device setup. Async because WebGPU only offers it that way; native callers block on it.

pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

pub fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env())
}

/// Picks an adapter (one that can draw to `surface`, if given) and opens a device on it.
pub async fn request(
    instance: wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<Gpu, String> {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: surface,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no graphics adapter: {e}"))?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("spelunky"),
            // Downlevel limits run everywhere WebGPU does; take the adapter's real texture size limit so
            // large windows still work.
            required_limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            ..Default::default()
        })
        .await
        .map_err(|e| format!("no graphics device: {e}"))?;
    Ok(Gpu {
        instance,
        adapter,
        device,
        queue,
    })
}
