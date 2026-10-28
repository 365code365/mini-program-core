//! 不是 ANGLE 的 WebGL 上下文（Android 系统 GLES）时，绘制前的内存安全校验（design-webgl.md §4）。
//!
//! ANGLE 的 WebGL 兼容上下文自己做这些；系统驱动不做 —— 越界的 drawArrays / drawElements 会让 GPU 读缓冲之外的内存。
//! 这里只管「读不读得出界」：每个程序用到的、启用了的属性，按缓冲尺寸 / 步长 / 偏移算最多能取几个顶点；
//! drawElements 再按索引缓冲的影子拷贝取最大索引（按 偏移 + 个数 + 类型 缓存，缓冲一改就作废）。

use std::collections::HashMap;

#[derive(Clone, Copy, Default)]
struct Attr {
    enabled: bool,
    buffer: u32,
    size: i32,
    ty: u32,
    stride: i32,
    offset: i64,
    divisor: u32,
}

#[derive(Clone, Default)]
struct Vao {
    element: u32,
    attribs: Vec<Attr>,
}

#[derive(Default)]
struct Buf {
    size: i64,
    /// 索引缓冲的影子拷贝（绑到过 ELEMENT_ARRAY_BUFFER 才留）
    shadow: Option<Vec<u8>>,
    max_cache: HashMap<(i64, i64, u32), u32>,
}

#[derive(Default)]
pub struct Tracker {
    buffers: HashMap<u32, Buf>,
    vaos: HashMap<u32, Vao>,
    cur_vao: u32,
    array_buffer: u32,
    programs: HashMap<u32, Vec<u32>>,
    cur_program: u32,
}

fn type_size(ty: u32) -> i64 {
    match ty {
        glow::BYTE | glow::UNSIGNED_BYTE => 1,
        glow::SHORT | glow::UNSIGNED_SHORT | glow::HALF_FLOAT | 0x8D61 => 2,
        _ => 4,
    }
}

impl Tracker {
    fn vao(&mut self) -> &mut Vao {
        let v = self.vaos.entry(self.cur_vao).or_default();
        if v.attribs.len() < 32 {
            v.attribs.resize(32, Attr::default());
        }
        v
    }

    pub fn bind_buffer(&mut self, target: u32, name: u32) {
        match target {
            glow::ARRAY_BUFFER => self.array_buffer = name,
            glow::ELEMENT_ARRAY_BUFFER => {
                self.vao().element = name;
                if name != 0 {
                    let b = self.buffers.entry(name).or_default();
                    if b.shadow.is_none() {
                        b.shadow = Some(vec![0; b.size as usize]);
                    }
                }
            }
            _ => {}
        }
    }

    fn bound(&mut self, target: u32) -> u32 {
        match target {
            glow::ARRAY_BUFFER => self.array_buffer,
            glow::ELEMENT_ARRAY_BUFFER => self.vao().element,
            _ => 0,
        }
    }

    /// bufferData：尺寸与（索引缓冲的）内容；data = None 是只给尺寸（内容为 0）
    pub fn buffer_data(&mut self, target: u32, size: i64, data: Option<&[u8]>) {
        let name = self.bound(target);
        if name == 0 {
            return;
        }
        let b = self.buffers.entry(name).or_default();
        b.size = size;
        b.max_cache.clear();
        if target == glow::ELEMENT_ARRAY_BUFFER || b.shadow.is_some() {
            let mut v = vec![0u8; size.max(0) as usize];
            if let Some(d) = data {
                let n = d.len().min(v.len());
                v[..n].copy_from_slice(&d[..n]);
            }
            b.shadow = Some(v);
        }
    }

    pub fn buffer_sub_data(&mut self, target: u32, offset: i64, data: &[u8]) {
        let name = self.bound(target);
        if let Some(b) = self.buffers.get_mut(&name) {
            b.max_cache.clear();
            if let Some(s) = b.shadow.as_mut() {
                let o = offset.max(0) as usize;
                if o + data.len() <= s.len() {
                    s[o..o + data.len()].copy_from_slice(data);
                }
            }
        }
    }

    pub fn delete_buffer(&mut self, name: u32) {
        self.buffers.remove(&name);
        if self.array_buffer == name {
            self.array_buffer = 0;
        }
        for v in self.vaos.values_mut() {
            if v.element == name {
                v.element = 0;
            }
            for a in v.attribs.iter_mut() {
                if a.buffer == name {
                    a.buffer = 0;
                }
            }
        }
    }

    pub fn enable_attrib(&mut self, i: u32, on: bool) {
        if let Some(a) = self.vao().attribs.get_mut(i as usize) {
            a.enabled = on;
        }
    }

    pub fn attrib_pointer(&mut self, i: u32, size: i32, ty: u32, stride: i32, offset: i64) {
        let buffer = self.array_buffer;
        if let Some(a) = self.vao().attribs.get_mut(i as usize) {
            (a.buffer, a.size, a.ty, a.stride, a.offset) = (buffer, size, ty, stride, offset);
        }
    }

    pub fn divisor(&mut self, i: u32, d: u32) {
        if let Some(a) = self.vao().attribs.get_mut(i as usize) {
            a.divisor = d;
        }
    }

    pub fn bind_vao(&mut self, name: u32) {
        self.cur_vao = name;
    }

    pub fn delete_vao(&mut self, name: u32) {
        self.vaos.remove(&name);
        if self.cur_vao == name {
            self.cur_vao = 0;
        }
    }

    pub fn use_program(&mut self, p: u32) {
        self.cur_program = p;
    }

    /// linkProgram 之后：记下这个程序用到的属性位置
    pub fn linked(&mut self, p: u32, locations: Vec<u32>) {
        self.programs.insert(p, locations);
    }

    pub fn delete_program(&mut self, p: u32) {
        self.programs.remove(&p);
    }

    /// 画 `vertices` 个顶点、`instances` 个实例够不够得着（不够 = INVALID_OPERATION）
    fn check_attribs(&mut self, vertices: i64, instances: i64) -> bool {
        let used = self.programs.get(&self.cur_program).cloned().unwrap_or_default();
        let attribs = self.vao().attribs.clone();
        for loc in used {
            let Some(a) = attribs.get(loc as usize) else { continue };
            if !a.enabled {
                continue;
            }
            // WebGL 不许客户端数组：启用了却没绑缓冲 = INVALID_OPERATION
            let Some(b) = self.buffers.get(&a.buffer) else { return false };
            let n = if a.divisor == 0 { vertices } else { (instances + a.divisor as i64 - 1) / a.divisor as i64 };
            if n <= 0 {
                continue;
            }
            let elem = a.size as i64 * type_size(a.ty);
            let stride = if a.stride == 0 { elem } else { a.stride as i64 };
            let need = a.offset + (n - 1) * stride + elem;
            if need > b.size {
                return false;
            }
        }
        true
    }

    pub fn draw_arrays(&mut self, first: i32, count: i32, instances: i32) -> bool {
        if count <= 0 || instances <= 0 {
            return true;
        }
        self.check_attribs(first as i64 + count as i64, instances as i64)
    }

    pub fn draw_elements(&mut self, count: i32, ty: u32, offset: i64, instances: i32) -> bool {
        if count <= 0 || instances <= 0 {
            return true;
        }
        let el = self.vao().element;
        let ts = type_size(ty);
        let Some(b) = self.buffers.get_mut(&el) else { return false };
        if offset < 0 || offset % ts != 0 || offset + count as i64 * ts > b.size {
            return false;
        }
        let key = (offset, count as i64, ty);
        let max = match b.max_cache.get(&key) {
            Some(m) => *m,
            None => {
                let Some(s) = b.shadow.as_ref() else { return false };
                let bytes = &s[offset as usize..(offset + count as i64 * ts) as usize];
                let m = match ts {
                    1 => bytes.iter().map(|x| *x as u32).max().unwrap_or(0),
                    2 => bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]) as u32).max().unwrap_or(0),
                    _ => bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).max().unwrap_or(0),
                };
                b.max_cache.insert(key, m);
                m
            }
        };
        self.check_attribs(max as i64 + 1, instances as i64)
    }
}
