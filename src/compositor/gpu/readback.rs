//! 离屏目标 → 256 字节对齐的缓冲 → 去填充 → RGBA（设计 §5.5）。
//!
//! 只给无头快照、自检、对拍用：每次都要等 GPU，上屏路径绝不走这里。
use super::device::Gpu;
use super::GpuError;

/// `copy_texture_to_buffer` 要求每行字节数是 256 的倍数
fn padded_row(w: u32) -> u32 {
    (w * 4).div_ceil(256) * 256
}

pub fn read_rgba(gpu: &Gpu, tex: &wgpu::Texture, w: u32, h: u32) -> Result<Vec<u8>, GpuError> {
    let row = padded_row(w);
    let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("读回"),
        size: row as u64 * h as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("读回") });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
        },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    gpu.wait_idle();
    match rx.recv() {
        Ok(Ok(())) => {}
        Ok(Err(e)) => return Err(GpuError::Readback(e.to_string())),
        Err(_) => return Err(GpuError::Readback("map_async 回调没有触发".into())),
    }
    if let Some(e) = gpu.health.fatal() {
        return Err(e);
    }
    let data = slice.get_mapped_range().map_err(|e| GpuError::Readback(format!("{e:?}")))?;
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h as usize {
        let start = y * row as usize;
        out.extend_from_slice(&data[start..start + (w * 4) as usize]);
    }
    drop(data);
    buf.unmap();
    Ok(out)
}

/// CPU 合成结果（0x00RRGGBB）转成与读回同一格式的 RGBA，便于逐字节比较
pub fn cpu_rgba(buf: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len() * 4);
    for px in buf {
        out.extend_from_slice(&[(px >> 16) as u8, (px >> 8) as u8, *px as u8, 255]);
    }
    out
}

/// 第一个不一致的像素：(x, y, cpu 0xRRGGBB, gpu 0xRRGGBB)
pub fn first_mismatch(cpu: &[u8], gpu: &[u8], w: u32) -> Option<(u32, u32, u32, u32)> {
    let i = cpu.chunks_exact(4).zip(gpu.chunks_exact(4)).position(|(a, b)| a != b)?;
    let px = |s: &[u8]| ((s[i * 4] as u32) << 16) | ((s[i * 4 + 1] as u32) << 8) | s[i * 4 + 2] as u32;
    Some(((i as u32) % w.max(1), (i as u32) / w.max(1), px(cpu), px(gpu)))
}
