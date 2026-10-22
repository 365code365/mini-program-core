// GPU 合成着色器：逐像素复刻 CPU 合成器（host::render::present_to_buffer + 导航栏 + 下拉指示器
// + 页面转场 / 侧滑返回的平移）。
//
// 为什么能和 CPU **逐字节**一致：
// - 源纹理一律是 Rgba8Uint，textureLoad 直接拿到整数，不经过 unorm→float、不经过采样器；
// - 混合用 u32 原样复刻 `(c*a + d*(255-a)) / 255`（截断除法，与 Rust 同）；
// - 压暗是 `u32(f32(c) * f)`：f 由 CPU 算好放进 uniform（不在着色器里做除法 —— Vulkan 的除法
//   允许 2.5 ULP 误差），乘法是正确舍入的，截断与 Rust 的 `as u32` 相同；
// - **不开硬件混合**：硬件混合是浮点舍入，会差 1 LSB；
// - 输出 `(k + 0.25) / 255`：Vulkan 规范允许 float→unorm 取「最接近的两个整数之一」，
//   有的实现截断、有的四舍五入。k + 0.25 无论哪种都落回 k（k = 255 时先被夹到 1.0）。
//
// 每一段的条件都对着 CPU 那边的循环边界写，改那边就要改这边（gpu-parity 守着）。

struct U {
    width: u32, height: u32, content_h: u32, scroll_px: i32,
    canvas_w: u32, canvas_h: u32, ring_rows: u32, page_bg: u32,
    has_tabbar: u32, tabbar_on: u32, tabbar_px: u32, tabbar_w: u32,
    tabbar_h: u32, fixed_on: u32, fixed_row0: u32, fixed_row_end: u32,
    fixed_w: u32, pull_rows: u32, mode: u32, fault_tint: u32,
    // 引擎画的导航栏：页面与 fixed 层整体下移 top_px 行；前 top_rows 行叠导航栏画布（宽 top_w）
    top_px: u32, top_rows: u32, top_w: u32,
    // 转场 / 侧滑：0 没有；1 缓冲里是上面那页、整帧右移，左边露出图；2 图在上面右移、左边露出缓冲里那页
    eb_mode: u32, eb_off: u32, eb_par: u32, eb_shadow: u32, eb_has_shot: u32,
    eb_dim: f32,
    // 直通帧（mode 1）：[pass_y0, pass_y1) 行取 CPU 合成结果；区间外自己合成，lut_on 时再查表压暗
    pass_y0: u32, pass_y1: u32, lut_on: u32,
    // 操作菜单面板缓存：直通帧里 [sheet_y0, sheet_y1) 行取 sheet_tex 第 y - sheet_top 行
    sheet_on: u32, sheet_top: i32, sheet_y0: u32, sheet_y1: u32,
    // 交界阴影各列的压暗系数（第 i 个 = 离交界 i 列，贴着边缘最深）
    eb_shadow_f: array<vec4f, 4>,
    // 覆盖层遮罩的 256 项压暗表（4 项一个字，小端）
    lut: array<vec4<u32>, 16>,
}

@group(0) @binding(0) var<uniform> u: U;
// 页面画布的环形条带镜像：画布第 r 行在 (r % ring_rows) 处，每层 1024 行
@group(0) @binding(1) var page: texture_2d_array<u32>;
@group(0) @binding(2) var tabbar: texture_2d<u32>;
@group(0) @binding(3) var fixed_layer: texture_2d<u32>;
@group(0) @binding(4) var pull_band: texture_2d<u32>;
// 直通帧：CPU 合成好的整帧（u32 0x00RRGGBB 按小端字节原样上传 → B,G,R,0）
@group(0) @binding(5) var pass_frame: texture_2d<u32>;
// 导航栏画布（RGBA）
@group(0) @binding(6) var top_band: texture_2d<u32>;
// 转场 / 侧滑的另一页留图（与直通帧同一种字节序：B,G,R,0）
@group(0) @binding(7) var eb_shot: texture_2d<u32>;
// 操作菜单面板缓存（host::ui_overlay::sheet_raster，同一种字节序）：面板画一次，每帧只在这里挪
@group(0) @binding(8) var sheet_tex: texture_2d<u32>;

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

// 与 present_to_buffer 的 fixed 层、导航栏、render_pull_indicator 同一套整数混合
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

// edge_back::DimLut：每个分量 `(c as f32 * f) as u32`
fn dim(c: vec3<u32>, f: f32) -> vec3<u32> {
    return vec3<u32>(u32(f32(c.x) * f), u32(f32(c.y) * f), u32(f32(c.z) * f));
}

fn shadow_factor(i: u32) -> f32 {
    return u.eb_shadow_f[i / 4u][i % 4u];
}

// 覆盖层遮罩：表由 CPU 按它自己的算式（截断 / 四舍五入、几层复合）算好，这里只查
fn lut_at(c: u32) -> u32 {
    return (u.lut[c / 16u][(c / 4u) % 4u] >> ((c % 4u) * 8u)) & 0xFFu;
}

// 不算转场的这一帧在 (x, y) 处的颜色：页面 → tabBar → fixed → 导航栏 → 下拉指示器
fn base(x: u32, y: u32) -> vec3<u32> {
    var rgb = vec3<u32>(0u);
    if (y < u.content_h) {
        // 导航栏占的那几行：present_to_buffer 不写（保留缓冲初值 0），由导航栏盖上
        if (y >= u.top_px) {
            // 主内容：画布内且 x < copy_w 取页面 RGB（忽略 alpha，与 CPU 一致），否则填页面底色
            let sy = i32(y - u.top_px) + u.scroll_px;
            if (sy >= 0 && u32(sy) < u.canvas_h && x < min(u.width, u.canvas_w)) {
                let k = u32(sy) % u.ring_rows;
                rgb = textureLoad(page, vec2<u32>(x, k % SLICE_ROWS), k / SLICE_ROWS, 0).rgb;
            } else {
                rgb = unpack_rgb(u.page_bg);
            }
        }
    } else if (u.has_tabbar == 1u && u.tabbar_on == 1u) {
        // tabBar 区：CPU 不写的像素（画布比 tabbar_px 矮 / 比输出窄）保留缓冲初值 0
        let ty = y - u.content_h;
        if (ty < min(u.tabbar_px, u.tabbar_h) && x < min(u.width, u.tabbar_w)) {
            rgb = textureLoad(tabbar, vec2<u32>(x, ty), 0).rgb;
        }
    }
    // fixed 画布是页面视口坐标：第 fy 行落在输出的 fy + top_px 行
    if (u.fixed_on == 1u && y >= u.top_px && x < min(u.width, u.fixed_w)) {
        let fy = y - u.top_px;
        if (fy >= u.fixed_row0 && fy < u.fixed_row_end) {
            rgb = blend(textureLoad(fixed_layer, vec2<u32>(x, fy), 0), rgb);
        }
    }
    if (y < u.top_rows && x < min(u.width, u.top_w)) {
        rgb = blend(textureLoad(top_band, vec2<u32>(x, y), 0), rgb);
    }
    if (y < u.pull_rows) {
        rgb = blend(textureLoad(pull_band, vec2<u32>(x, y), 0), rgb);
    }
    return rgb;
}

fn shot_at(x: u32, y: u32) -> vec3<u32> {
    if (u.eb_has_shot == 0u) {
        return vec3<u32>(0u);
    }
    let p = textureLoad(eb_shot, vec2<u32>(x, y), 0);
    return vec3<u32>(p.z, p.y, p.x);
}

// edge_back::compose / compose_shot_over 的逐像素版
fn edge_back(x: u32, y: u32) -> vec3<u32> {
    let off = u.eb_off;
    let w = u.width;
    var rgb: vec3<u32>;
    if (u.eb_mode == 1u) {
        if (off == 0u) {
            return base(x, y);
        }
        if (x >= off) {
            rgb = base(x - off, y);
        } else {
            rgb = dim(shot_at(min(x + u.eb_par, w - 1u), y), u.eb_dim);
        }
    } else {
        if (x < off) {
            rgb = dim(base(min(x + u.eb_par, w - 1u), y), u.eb_dim);
        } else {
            rgb = shot_at(x - off, y);
        }
    }
    // 交界处的投影：上面那一页的左边缘压在下面那一页上（只在左边露出的那一条里）
    if (x < off && x + u.eb_shadow >= off) {
        rgb = dim(rgb, shadow_factor(off - 1u - x));
    }
    return rgb;
}

@fragment
fn fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    // 片元坐标在像素中心（x + 0.5），截断就是整数像素坐标
    let x = u32(pos.x);
    let y = u32(pos.y);
    var rgb = vec3<u32>(0u);
    if (u.mode == 1u && y >= u.pass_y0 && y < u.pass_y1) {
        let p = textureLoad(pass_frame, vec2<u32>(x, y), 0);
        rgb = vec3<u32>(p.z, p.y, p.x);
    } else if (u.mode == 1u && u.sheet_on == 1u && y >= u.sheet_y0 && y < u.sheet_y1) {
        // 面板正文是不透明的：整行取缓存，底下的页面、遮罩都不用算
        let p = textureLoad(sheet_tex, vec2<u32>(x, u32(i32(y) - u.sheet_top)), 0);
        rgb = vec3<u32>(p.z, p.y, p.x);
    } else {
        if (u.eb_mode != 0u) {
            rgb = edge_back(x, y);
        } else {
            rgb = base(x, y);
        }
        // 覆盖层的整屏遮罩（只在直通帧里、覆盖层行区间外；侧滑叠覆盖层时整帧直通，不会走到这里）
        if (u.mode == 1u && u.lut_on == 1u) {
            rgb = vec3<u32>(lut_at(rgb.x), lut_at(rgb.y), lut_at(rgb.z));
        }
    }
    // 故障注入 `tint`：R 通道 +1（饱和），证明比较链路是活的
    if (u.fault_tint == 1u) {
        rgb.x = min(rgb.x + 1u, 255u);
    }
    return vec4f((vec3f(rgb) + 0.25) / 255.0, 1.0);
}
