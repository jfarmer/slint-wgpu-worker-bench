//! A fixed clear-plus-triangle scene, rendered offscreen for readback and to a surface.

use serde_json::{json, Value};

pub const SIZE: u32 = 64;
const CLEAR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

const SHADER: &str = r#"
@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    var p = array<vec2<f32>, 3>(vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(0.0, 0.5));
    return vec4(p[i], 0.0, 1.0);
}
@fragment
fn fs() -> @location(0) vec4<f32> {
    return vec4(1.0, 0.5, 0.0, 1.0);
}
"#;

pub fn pipeline(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bench"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bench"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs"),
            targets: &[Some(format.into())],
            compilation_options: Default::default(),
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn draw(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("bench"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(CLEAR),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_pipeline(pipeline);
    pass.draw(0..3, 0..1);
}

fn fnv1a(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn is_bgra(format: wgpu::TextureFormat) -> bool {
    matches!(
        format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    )
}

/// Renders the scene into an offscreen texture of `format` and reads it back.
/// Returns the readback record, or an error for formats this check can't read as 8-bit RGBA.
pub fn offscreen(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    format: wgpu::TextureFormat,
) -> Result<Value, String> {
    if format.block_copy_size(None) != Some(4) {
        return Err(format!("readback of {format:?} is not implemented"));
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bench-offscreen"),
        size: wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let bytes_per_row = SIZE * 4; // 256, already aligned to COPY_BYTES_PER_ROW_ALIGNMENT
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bench-readback"),
        size: (bytes_per_row * SIZE) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let view = texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    draw(&mut encoder, &view, pipeline);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(SIZE),
            },
        },
        wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| format!("poll failed: {e}"))?;
    rx.recv()
        .map_err(|e| format!("map callback dropped: {e}"))?
        .map_err(|e| format!("map failed: {e}"))?;
    let mut rgba = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|e| format!("mapped range failed: {e:?}"))?
        .to_vec();
    buffer.unmap();
    if is_bgra(format) {
        for px in rgba.chunks_exact_mut(4) {
            px.swap(0, 2);
        }
    }
    let pixel = |x: u32, y: u32| {
        let i = ((y * SIZE + x) * 4) as usize;
        rgba[i..i + 4].to_vec()
    };
    Ok(json!({
        "format": format!("{format:?}"),
        "size": [SIZE, SIZE],
        "rgba_hash": fnv1a(&rgba),
        "pixels_rgba": {
            "corner_0_0": pixel(0, 0),
            "center_32_32": pixel(SIZE / 2, SIZE / 2),
            "corner_63_63": pixel(SIZE - 1, SIZE - 1),
            "top_32_4": pixel(SIZE / 2, 4),
        },
    }))
}

/// Renders the scene to the surface's next texture and presents it.
pub fn present(
    surface: &wgpu::Surface<'_>,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
) -> Result<&'static str, String> {
    let (texture, status) = match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(t) => (t, "success"),
        wgpu::CurrentSurfaceTexture::Suboptimal(t) => (t, "suboptimal"),
        wgpu::CurrentSurfaceTexture::Timeout => return Err("get_current_texture: Timeout".into()),
        wgpu::CurrentSurfaceTexture::Occluded => return Ok("occluded"),
        wgpu::CurrentSurfaceTexture::Outdated => return Err("get_current_texture: Outdated".into()),
        wgpu::CurrentSurfaceTexture::Lost => return Err("get_current_texture: Lost".into()),
        wgpu::CurrentSurfaceTexture::Validation => {
            return Err("get_current_texture: Validation".into())
        }
    };
    let view = texture.texture.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    draw(&mut encoder, &view, pipeline);
    queue.submit([encoder.finish()]);
    queue.present(texture);
    Ok(status)
}
