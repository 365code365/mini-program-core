//! MP4 容器解析：遍历 box、找到视频轨、取出样本表
//!
//! `video` 模块的一片。**纯搬迁**，一行逻辑没改。
/// 视频轨道的样本表信息（从 MP4 的视频 trak 中提取）。
pub(super) struct Mp4VideoTrack {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) timescale: u32,
    pub(super) sample_sizes: Vec<u32>,
    pub(super) chunk_offsets: Vec<u64>,
    pub(super) sample_to_chunk: Vec<(u32, u32, u32)>,
    pub(super) sync_samples: Vec<u32>,
    pub(super) sample_durations: Vec<(u32, u32)>,
}

/// 列出 [start, end) 范围内的直接子 box：返回 (类型, 内容起始, box 结束)。
/// 正确处理 32 位 size、`size==1` 的 64 位 largesize、`size==0`（到末尾）。
fn mp4_child_boxes(data: &[u8], start: usize, end: usize) -> Vec<([u8; 4], usize, usize)> {
    let mut boxes = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        let size32 = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let typ = [data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]];
        let (content, box_size) = if size32 == 1 {
            if pos + 16 > end { break; }
            let large = u64::from_be_bytes([
                data[pos + 8], data[pos + 9], data[pos + 10], data[pos + 11],
                data[pos + 12], data[pos + 13], data[pos + 14], data[pos + 15],
            ]) as usize;
            (pos + 16, large)
        } else if size32 == 0 {
            (pos + 8, end - pos)
        } else {
            (pos + 8, size32)
        };
        if box_size < 8 || pos + box_size > end { break; }
        boxes.push((typ, content, pos + box_size));
        pos += box_size;
    }
    boxes
}

/// 在给定范围内查找第一个指定类型的 box，返回其 (内容起始, 结束)。
fn mp4_find<'a>(data: &[u8], start: usize, end: usize, want: &[u8; 4]) -> Option<(usize, usize)> {
    for (typ, c, e) in mp4_child_boxes(data, start, end) {
        if &typ == want { return Some((c, e)); }
    }
    None
}

/// 解析 MP4，仅提取「视频」轨道（hdlr == 'vide'）的样本表。
pub(super) fn parse_mp4_video_track(data: &[u8]) -> Option<Mp4VideoTrack> {
    let (moov_c, moov_e) = mp4_find(data, 0, data.len(), b"moov")?;
    // 遍历所有 trak，选出视频轨
    for (typ, trak_c, trak_e) in mp4_child_boxes(data, moov_c, moov_e) {
        if &typ != b"trak" { continue; }
        let (mdia_c, mdia_e) = match mp4_find(data, trak_c, trak_e, b"mdia") { Some(v) => v, None => continue };
        // 判定 handler 是否为视频
        let is_video = mp4_find(data, mdia_c, mdia_e, b"hdlr")
            .map(|(hc, _)| hc + 16 <= data.len() && &data[hc + 8..hc + 12] == b"vide")
            .unwrap_or(false);
        if !is_video { continue; }
        
        // mdhd -> timescale
        let mut timescale = 1u32;
        if let Some((mc, _)) = mp4_find(data, mdia_c, mdia_e, b"mdhd") {
            let version = data.get(mc)?;
            let ts_off = if *version == 1 { mc + 20 } else { mc + 12 };
            if ts_off + 4 <= data.len() {
                timescale = u32::from_be_bytes([data[ts_off], data[ts_off + 1], data[ts_off + 2], data[ts_off + 3]]);
            }
        }
        // tkhd -> 宽高（16.16 定点，取整数部分）
        let (mut width, mut height) = (0u32, 0u32);
        if let Some((tc, _)) = mp4_find(data, trak_c, trak_e, b"tkhd") {
            let version = data.get(tc).copied().unwrap_or(0);
            // tkhd width/height（16.16 定点）距内容起始：v0=76，v1=88
            let off = if version == 1 { tc + 88 } else { tc + 76 };
            if off + 8 <= data.len() {
                width = u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]) >> 16;
                height = u32::from_be_bytes([data[off + 4], data[off + 5], data[off + 6], data[off + 7]]) >> 16;
            }
        }
        
        // 定位 stbl
        let (minf_c, minf_e) = mp4_find(data, mdia_c, mdia_e, b"minf")?;
        let (stbl_c, stbl_e) = mp4_find(data, minf_c, minf_e, b"stbl")?;
        
        let mut sample_sizes = Vec::new();
        let mut chunk_offsets = Vec::new();
        let mut sample_to_chunk = Vec::new();
        let mut sync_samples = Vec::new();
        let mut sample_durations = Vec::new();
        
        for (btyp, bc, _be) in mp4_child_boxes(data, stbl_c, stbl_e) {
            let read_u32 = |o: usize| u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
            match &btyp {
                b"stts" => {
                    let cnt = read_u32(bc + 4) as usize;
                    let mut o = bc + 8;
                    for _ in 0..cnt { if o + 8 > data.len() { break; } sample_durations.push((read_u32(o), read_u32(o + 4))); o += 8; }
                }
                b"stss" => {
                    let cnt = read_u32(bc + 4) as usize;
                    let mut o = bc + 8;
                    for _ in 0..cnt { if o + 4 > data.len() { break; } sync_samples.push(read_u32(o)); o += 4; }
                }
                b"stsc" => {
                    let cnt = read_u32(bc + 4) as usize;
                    let mut o = bc + 8;
                    for _ in 0..cnt { if o + 12 > data.len() { break; } sample_to_chunk.push((read_u32(o), read_u32(o + 4), read_u32(o + 8))); o += 12; }
                }
                b"stsz" => {
                    let default_size = read_u32(bc + 4);
                    let cnt = read_u32(bc + 8) as usize;
                    if default_size == 0 {
                        let mut o = bc + 12;
                        for _ in 0..cnt { if o + 4 > data.len() { break; } sample_sizes.push(read_u32(o)); o += 4; }
                    } else {
                        sample_sizes = vec![default_size; cnt];
                    }
                }
                b"stco" => {
                    let cnt = read_u32(bc + 4) as usize;
                    let mut o = bc + 8;
                    for _ in 0..cnt { if o + 4 > data.len() { break; } chunk_offsets.push(read_u32(o) as u64); o += 4; }
                }
                b"co64" => {
                    let cnt = read_u32(bc + 4) as usize;
                    let mut o = bc + 8;
                    for _ in 0..cnt {
                        if o + 8 > data.len() { break; }
                        chunk_offsets.push(u64::from_be_bytes([
                            data[o], data[o + 1], data[o + 2], data[o + 3],
                            data[o + 4], data[o + 5], data[o + 6], data[o + 7],
                        ]));
                        o += 8;
                    }
                }
                _ => {}
            }
        }
        
        return Some(Mp4VideoTrack {
            width, height, timescale,
            sample_sizes, chunk_offsets, sample_to_chunk, sync_samples, sample_durations,
        });
    }
    None
}
