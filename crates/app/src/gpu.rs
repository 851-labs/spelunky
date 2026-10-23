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

/// The browser's instance: WebGPU where the browser has a working adapter, else WebGL2. Whether it does
/// is only known asynchronously, and `navigator.gpu` alone doesn't say (Chrome on Linux has it but no
/// adapter).
#[cfg(target_arch = "wasm32")]
pub async fn web_instance() -> wgpu::Instance {
    wgpu::util::new_instance_with_webgpu_detection(
        wgpu::InstanceDescriptor::new_without_display_handle(),
    )
    .await
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
    // Downlevel limits run everywhere WebGPU does, and WebGL2's lower ones (no compute or storage, which
    // the renderer doesn't use) everywhere it does. Take the adapter's real texture size limit so large
    // windows still work.
    let limits = if adapter.get_info().backend == wgpu::Backend::Gl {
        wgpu::Limits::downlevel_webgl2_defaults()
    } else {
        wgpu::Limits::downlevel_defaults()
    };
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("spelunky"),
            required_limits: limits.using_resolution(adapter.limits()),
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
