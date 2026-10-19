// GPU 合成着色器：逐像素复刻 CPU 合成器（host::render::present_to_buffer + 下拉指示器混合）。
//
// 为什么能和 CPU **逐字节**一致：
// - 源纹理一律是 Rgba8Uint，textureLoad 直接拿到整数，不经过 unorm→float、不经过采样器；
// - 混合用 u32 原样复刻 `(c*a + d*(255-a)) / 255`（截断除法，与 Rust 同）；
// - **不开硬件混合**：硬件混合是浮点舍入，会差 1 LSB；
// - 输出 `(k + 0.25) / 255`：Vulkan 规范允许 float→unorm 取「最接近的两个整数之一」，
//   有的实现截断、有的四舍五入。k + 0.25 无论哪种都落回 k（k = 255 时先被夹到 1.0）。
//
// 每一段的条件都对着 present_to_buffer 的循环边界写，改那边就要改这边（gpu-parity 守着）。

struct U {
    width: u32, height: u32, content_h: u32, scroll_px: i32,
    canvas_w: u32, canvas_h: u32, ring_rows: u32, page_bg: u32,
    has_tabbar: u32, tabbar_on: u32, tabbar_px: u32, tabbar_w: u32,
    tabbar_h: u32, fixed_on: u32, fixed_row0: u32, fixed_row_end: u32,
    fixed_w: u32, pull_rows: u32, mode: u32, fault_tint: u32,
}

@group(0) @binding(0) var<uniform> u: U;
// 页面画布的环形条带镜像：画布第 r 行在 (r % ring_rows) 处，每层 1024 行
@group(0) @binding(1) var page: texture_2d_array<u32>;
@group(0) @binding(2) var tabbar: texture_2d<u32>;
@group(0) @binding(3) var fixed_layer: texture_2d<u32>;
@group(0) @binding(4) var pull_band: texture_2d<u32>;
// 直通帧：CPU 合成好的整帧（u32 0x00RRGGBB 按小端字节原样上传 → B,G,R,0）
@group(0) @binding(5) var pass_frame: texture_2d<u32>;

const SLICE_ROWS: u32 = 1024u;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    // 一个盖住全屏的三角形：(-1,1) (3,1) (-1,-3)
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    return vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

fn unpack_rgb(c: u32) -> vec3<u32> {
    return vec3<u32>((c >> 16u) & 0xFFu, (c >> 8u) & 0xFFu, c & 0xFFu);
}

// 与 present_to_buffer 的 fixed 层、render_pull_indicator 同一套整数混合
fn blend(c: vec4<u32>, d: vec3<u32>) -> vec3<u32> {
    if (c.a == 0u) {
        return d;
    }
    if (c.a == 255u) {
        return c.rgb;
    }
    let inv = 255u - c.a;
    return (c.rgb * c.a + d * inv) / 255u;
}

@fragment
fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    // 片元坐标在像素中心（x + 0.5），截断就是整数像素坐标
    let x = u32(pos.x);
    let y = u32(pos.y);
    var rgb = vec3<u32>(0u);
    if (u.mode == 1u) {
        let p = textureLoad(pass_frame, vec2<u32>(x, y), 0);
        rgb = vec3<u32>(p.z, p.y, p.x);
    } else {
        if (y < u.content_h) {
            // 主内容：画布内且 x < copy_w 取页面 RGB（忽略 alpha，与 CPU 一致），否则填页面底色
            let sy = i32(y) + u.scroll_px;
            if (sy >= 0 && u32(sy) < u.canvas_h && x < min(u.width, u.canvas_w)) {
                let k = u32(sy) % u.ring_rows;
                rgb = textureLoad(page, vec2<u32>(x, k % SLICE_ROWS), k / SLICE_ROWS, 0).rgb;
            } else {
                rgb = unpack_rgb(u.page_bg);
            }
        } else if (u.has_tabbar == 1u && u.tabbar_on == 1u) {
            // tabBar 区：CPU 不写的像素（画布比 tabbar_px 矮 / 比输出窄）保留缓冲初值 0
            let ty = y - u.content_h;
            if (ty < min(u.tabbar_px, u.tabbar_h) && x < min(u.width, u.tabbar_w)) {
                rgb = textureLoad(tabbar, vec2<u32>(x, ty), 0).rgb;
            }
        }
        if (u.fixed_on == 1u && y >= u.fixed_row0 && y < u.fixed_row_end && x < min(u.width, u.fixed_w)) {
            rgb = blend(textureLoad(fixed_layer, vec2<u32>(x, y), 0), rgb);
        }
        if (y < u.pull_rows) {
            rgb = blend(textureLoad(pull_band, vec2<u32>(x, y), 0), rgb);
        }
    }
    // 故障注入 `tint`：R 通道 +1（饱和），证明比较链路是活的
    if (u.fault_tint == 1u) {
        rgb.x = min(rgb.x + 1u, 255u);
    }
    return vec4f((vec3f(rgb) + 0.25) / 255.0, 1.0);
}
