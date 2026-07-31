//! H.264 解码与像素转换：拆样本、取 SPS/PPS、YUV→RGBA
//!
//! `VideoPlayer` 的一片，由 `video/mod.rs` 组合。**纯搬迁**：从 1064 行的
//! components/video.rs 按职责切开，一行逻辑没改。
use super::mp4::parse_mp4_video_track;
use super::*;

impl VideoPlayer {
    /// 加载视频
    pub fn load(&mut self) -> Result<(), String> {
        // 尝试多个可能的路径
        let paths_to_try = vec![
            self.src.clone(),
            self.src.trim_start_matches('/').to_string(),
            crate::app_dir::default_app().join(self.src.trim_start_matches('/')).to_string_lossy().to_string(),
            crate::app_dir::default_app().to_string_lossy().to_string() + "/" + self.src.trim_start_matches('/'),
        ];
        
        let mut actual_path = None;
        for path in &paths_to_try {
            if std::path::Path::new(path).exists() {
                actual_path = Some(PathBuf::from(path));
                break;
            }
        }
        
        let path = actual_path.ok_or_else(|| format!("Cannot find video: {}", self.src))?;
        self.file_path = Some(path.clone());
        
        println!("🎬 Loading video: {}", path.display());
        
        // 解析 MP4 并解码视频帧
        self.decode_video_manual(&path)?;
        
        // 解码音频
        if let Err(e) = self.decode_audio(&path) {
            println!("   ⚠️ Audio decode warning: {}", e);
        }
        
        Ok(())
    }

    /// 手动解析 MP4 并解码视频帧
    pub(super) fn decode_video_manual(&mut self, path: &PathBuf) -> Result<(), String> {
        let data = std::fs::read(path).map_err(|e| format!("Cannot read file: {}", e))?;
        
        // 只解析「视频」轨道的样本表（此前扁平解析把音/视频两条轨的 stsz/stco/stsc
        // 混在一起，导致视频样本偏移错乱，782 帧仅解出 7 帧）。
        let track = parse_mp4_video_track(&data).ok_or("未找到视频轨道".to_string())?;
        let width = track.width;
        let height = track.height;
        let timescale = track.timescale.max(1);
        let sample_sizes = track.sample_sizes;
        let chunk_offsets = track.chunk_offsets;
        let sample_to_chunk = track.sample_to_chunk;
        let sync_samples = track.sync_samples;
        let sample_durations = track.sample_durations;
        
        self.width = width;
        self.height = height;
        
        let sample_count = sample_sizes.len();
        if sample_count == 0 {
            return Err("No samples found".to_string());
        }
        
        // 计算时长和帧率
        let mut total_duration = 0u64;
        for (count, delta) in &sample_durations {
            total_duration += *count as u64 * *delta as u64;
        }
        self.duration = total_duration as f64 / timescale as f64;
        self.fps = if self.duration > 0.0 { sample_count as f64 / self.duration } else { 24.0 };
        
        println!("   Video: {}x{}, {:.1} fps, {:.1}s, {} samples", 
            self.width, self.height, self.fps, self.duration, sample_count);
        
        // 获取 SPS/PPS
        let (sps, pps) = self.extract_sps_pps_manual(path)?;
        
        // ── H.264 解码（`h264` 特性）──
        //
        // 关掉这个特性时不解帧：移动端本该把播放交给系统播放器（硬解、省电、
        // 遵守音频焦点），而且 `openh264-sys2` 在 iOS **模拟器**目标上直接
        // 编不过（它的 build.rs 不认 target_env = "sim"）。
        // 关掉后 `<video>` 仍然参与布局与命中，只是没有画面帧。
        #[cfg(feature = "h264")]
        {
            // 创建 H.264 解码器
            let mut decoder = H264Decoder::new()
                .map_err(|e| format!("H264 decoder init error: {:?}", e))?;
        
            // 构建样本偏移表
            let sample_offsets = self.build_sample_offsets(&sample_sizes, &chunk_offsets, &sample_to_chunk);
        
            // 计算每个样本的时间戳
            let timestamps = self.build_timestamps(&sample_durations, timescale);
        
            // 解码帧
            let start_time = Instant::now();
            let mut decoded_count = 0;
            let mut skipped_count = 0;
        
            use openh264::nal_units;
            use openh264::formats::YUVSource;
        
            // openh264 要求「逐个 NAL 单元」喂给 decode（见其文档 nal_units 示例）。
            // 之前把整个 access unit（SPS+PPS+slice 拼接）一次性传入，解码器只处理了
            // 第一个 NAL，导致 782 帧仅解出约 7 个关键帧。改为用 nal_units 拆分逐个解码。
        
            // 先喂入 SPS/PPS 参数集
            {
                let mut init = Vec::new();
                init.extend_from_slice(&[0, 0, 0, 1]);
                init.extend_from_slice(&sps);
                init.extend_from_slice(&[0, 0, 0, 1]);
                init.extend_from_slice(&pps);
                for nal in nal_units(&init) {
                    let _ = decoder.decode(nal);
                }
            }
        
            for (sample_idx, &(offset, size)) in sample_offsets.iter().enumerate() {
                if offset as usize + size as usize > data.len() {
                    continue;
                }
            
                let sample_data = &data[offset as usize..(offset as usize + size as usize)];
                let timestamp = timestamps.get(sample_idx).copied().unwrap_or(0.0);
                let is_idr = sync_samples.contains(&((sample_idx + 1) as u32));
            
                // 构建该样本的 Annex-B 流（仅关键帧前重置参数集）
                let mut nal_data = Vec::new();
                if is_idr {
                    nal_data.extend_from_slice(&[0, 0, 0, 1]);
                    nal_data.extend_from_slice(&sps);
                    nal_data.extend_from_slice(&[0, 0, 0, 1]);
                    nal_data.extend_from_slice(&pps);
                }
                let mut off = 0;
                while off + 4 <= sample_data.len() {
                    let nal_size = u32::from_be_bytes([
                        sample_data[off], sample_data[off+1], sample_data[off+2], sample_data[off+3]
                    ]) as usize;
                    off += 4;
                    if off + nal_size <= sample_data.len() {
                        nal_data.extend_from_slice(&[0, 0, 0, 1]);
                        nal_data.extend_from_slice(&sample_data[off..off + nal_size]);
                        off += nal_size;
                    } else {
                        break;
                    }
                }
            
                // 逐 NAL 解码，取本样本解出的（最后一个）画面帧
                let mut got: Option<(Vec<u8>, u32, u32)> = None;
                for nal in nal_units(&nal_data) {
                    if let Ok(Some(yuv)) = decoder.decode(nal) {
                        let (fw, fh) = yuv.dimensions();
                        got = Some((self.yuv_to_rgba(&yuv), fw as u32, fh as u32));
                    }
                }
            
                if let Some((rgba, fw, fh)) = got {
                    self.frames.push(VideoFrame { data: rgba, width: fw, height: fh, timestamp });
                    decoded_count += 1;
                } else {
                    skipped_count += 1;
                }
            
                if decoded_count >= 3600 {
                    println!("   ⚠️ Reached max frame limit");
                    break;
                }
            }
            // 用解码得到的真实尺寸修正整体宽高（tkhd 解析可能不准）
            if let Some(f) = self.frames.first() {
                self.width = f.width;
                self.height = f.height;
            }
            let decode_time = start_time.elapsed();
            if self.frames.is_empty() {
                return Err(format!("No frames decoded (skipped {})", skipped_count));
            }
            self.is_loaded = true;
            println!("✅ Video loaded: {} frames decoded, {} skipped ({:.1}s)",
                decoded_count, skipped_count, decode_time.as_secs_f64());
            return Ok(());
        }
        #[cfg(not(feature = "h264"))]
        {
            let _ = (&sps, &pps, &sync_samples, &sample_sizes, &chunk_offsets);
            println!("ℹ️  本构建未启用 h264 特性，<video> 不解码画面帧（交给宿主播放器）");
            Err("h264 decoding disabled in this build".to_string())
        }
    }

    /// 构建样本偏移表
    pub(super) fn build_sample_offsets(&self, sample_sizes: &[u32], chunk_offsets: &[u64], sample_to_chunk: &[(u32, u32, u32)]) -> Vec<(u64, u32)> {
        let mut result = Vec::new();
        let mut sample_idx = 0;
        
        for (chunk_idx, &chunk_offset) in chunk_offsets.iter().enumerate() {
            let chunk_num = (chunk_idx + 1) as u32;
            
            // 找到这个 chunk 的 samples_per_chunk
            let mut samples_per_chunk = 1u32;
            for (i, &(first_chunk, spc, _)) in sample_to_chunk.iter().enumerate() {
                if chunk_num >= first_chunk {
                    let next_first = sample_to_chunk.get(i + 1).map(|x| x.0).unwrap_or(u32::MAX);
                    if chunk_num < next_first {
                        samples_per_chunk = spc;
                        break;
                    }
                }
            }
            
            let mut offset = chunk_offset;
            for _ in 0..samples_per_chunk {
                if sample_idx >= sample_sizes.len() {
                    break;
                }
                let size = sample_sizes[sample_idx];
                result.push((offset, size));
                offset += size as u64;
                sample_idx += 1;
            }
        }
        
        result
    }

    /// 构建时间戳表
    pub(super) fn build_timestamps(&self, sample_durations: &[(u32, u32)], timescale: u32) -> Vec<f64> {
        let mut timestamps = Vec::new();
        let mut current_time = 0u64;
        
        for &(count, delta) in sample_durations {
            for _ in 0..count {
                timestamps.push(current_time as f64 / timescale as f64);
                current_time += delta as u64;
            }
        }
        
        timestamps
    }

    /// 从 MP4 提取 SPS/PPS (手动解析)
    pub(super) fn extract_sps_pps_manual(&self, path: &PathBuf) -> Result<(Vec<u8>, Vec<u8>), String> {
        let data = std::fs::read(path).map_err(|e| format!("Cannot read file: {}", e))?;
        
        // 搜索 avcC box
        let avcc_marker = b"avcC";
        let mut pos = 0;
        while pos + 4 < data.len() {
            if &data[pos..pos+4] == avcc_marker {
                // 找到 avcC，解析内容
                // avcC 格式:
                // 1 byte: configurationVersion
                // 1 byte: AVCProfileIndication
                // 1 byte: profile_compatibility
                // 1 byte: AVCLevelIndication
                // 1 byte: lengthSizeMinusOne (低2位)
                // 1 byte: numOfSequenceParameterSets (低5位)
                // 然后是 SPS 列表
                // 1 byte: numOfPictureParameterSets
                // 然后是 PPS 列表
                
                let avcc_start = pos + 4;
                if avcc_start + 6 >= data.len() {
                    pos += 1;
                    continue;
                }
                
                let num_sps = data[avcc_start + 5] & 0x1F;
                let mut offset = avcc_start + 6;
                
                let mut sps = Vec::new();
                for _ in 0..num_sps {
                    if offset + 2 > data.len() { break; }
                    let sps_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + sps_len > data.len() { break; }
                    sps = data[offset..offset + sps_len].to_vec();
                    offset += sps_len;
                }
                
                if offset >= data.len() {
                    pos += 1;
                    continue;
                }
                
                let num_pps = data[offset];
                offset += 1;
                
                let mut pps = Vec::new();
                for _ in 0..num_pps {
                    if offset + 2 > data.len() { break; }
                    let pps_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                    offset += 2;
                    if offset + pps_len > data.len() { break; }
                    pps = data[offset..offset + pps_len].to_vec();
                    offset += pps_len;
                }
                
                if !sps.is_empty() && !pps.is_empty() {
                    println!("   Found SPS ({} bytes), PPS ({} bytes) via manual parse", sps.len(), pps.len());
                    return Ok((sps, pps));
                }
            }
            pos += 1;
        }
        
        Err("Cannot find avcC in file".to_string())
    }

    /// YUV 转 RGBA
    #[cfg(feature = "h264")]
    pub(super) fn yuv_to_rgba(&self, yuv: &openh264::decoder::DecodedYUV) -> Vec<u8> {
        use openh264::formats::YUVSource;
        
        let (width, height) = yuv.dimensions();
        let mut rgba = vec![0u8; width * height * 4];
        
        let (y_stride, u_stride, v_stride) = yuv.strides();
        
        let y_data = yuv.y();
        let u_data = yuv.u();
        let v_data = yuv.v();
        
        for row in 0..height {
            for col in 0..width {
                let y_idx = row * y_stride + col;
                let uv_row = row / 2;
                let uv_col = col / 2;
                let u_idx = uv_row * u_stride + uv_col;
                let v_idx = uv_row * v_stride + uv_col;
                
                let y = y_data.get(y_idx).copied().unwrap_or(0) as f32;
                let u = u_data.get(u_idx).copied().unwrap_or(128) as f32 - 128.0;
                let v = v_data.get(v_idx).copied().unwrap_or(128) as f32 - 128.0;
                
                let r = (y + 1.402 * v).clamp(0.0, 255.0) as u8;
                let g = (y - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
                let b = (y + 1.772 * u).clamp(0.0, 255.0) as u8;
                
                let idx = (row * width + col) * 4;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }
        
        rgba
    }
}
