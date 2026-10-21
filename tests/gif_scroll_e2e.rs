//! 动图（GIF）在 SDK 引擎里**静止时、滚动时都按自己的帧率换帧**，而且只在换帧时重画。
//!
//! 从前 GIF 不算「在动」：页面静止时宿主停帧，GIF 停在第一帧；滑动时 SDK 只合成
//! （或只补新露出的条带），已经画好的 GIF 停在最后一次被画到的那一帧 —— Flutter 宿主里
//! 「一滑 GIF 就停」。修法见 `src/renderer/wxml_renderer/animated_image.rs`。
//!
//! 判据（都是像素级的，不看日志）：
//! - 静止约 1 秒：GIF 那块至少换过 4 种画面，且出新画面的帧数远少于 pump 次数
//!   （按 GIF 自己 120ms 一帧走，不是每个 vsync 都重画）；
//! - 每帧滚 1px、滚约 1 秒：按滚动位置把 GIF 那块抠出来，同样至少 4 种画面。
//!   空转守卫：这一段里必须出现过「只合成」（真的走了滚动的便宜路径）和「局部」（GIF 重画）。
//!
//! GIF 的帧按墙上时钟取（`image/load.rs`），所以这里用真实时间推进，不用动画时钟。
use mini_render::host::engine_backend::{FRAME_NEW, FRAME_WANTS_MORE};
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::MiniEngine;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const W: u32 = 375;
const H: u32 = 667;
const DPR: f32 = 2.0;
/// GIF 在内容里的位置（逻辑像素）：顶上一条 100px 的头，GIF 120×120 贴左边
const GIF_TOP: f32 = 100.0;
const GIF_SIZE: f32 = 120.0;

/// 引擎的根目录、数据目录是进程级的：同一进程里的用例串行跑
static SERIAL: Mutex<()> = Mutex::new(());

fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("gif-scroll-e2e");
    std::fs::remove_dir_all(&root).ok();
    let gif = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/sample-app/assets/loading.gif");
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        (
            "pages/p/p.wxml",
            r#"<view class="hd">顶部</view>
<image class="gif" src="/images/loading.gif"/>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>"#,
        ),
        (
            "pages/p/p.wxss",
            ".hd { height: 100px; background-color: #ffffff; }
.gif { display: block; width: 120px; height: 120px; }
.row { height: 44px; line-height: 44px; padding: 0 16px; border-bottom: 1px solid #eeeeee; background-color: #ffffff; }",
        ),
        ("pages/p/p.js", "Page({ data: { rows: Array.from({length: 40}, (_, i) => i + 1) } });"),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    std::fs::create_dir_all(root.join("images")).unwrap();
    std::fs::copy(&gif, root.join("images/loading.gif")).expect("sample-app 里的 loading.gif");
    root
}

struct Rig {
    e: MiniEngine,
    clock: Instant,
}

impl Rig {
    fn new() -> Self {
        let root = fixture();
        mini_render::renderer::components::reset_shared_component_state();
        let data = root.join("_data");
        let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), W, H, DPR)
            .expect("创建引擎");
        e.launch(None).expect("打开首页");
        let mut rig = Rig { e, clock: Instant::now() };
        for _ in 0..4 {
            rig.pump(16);
        }
        rig
    }

    /// 等 `ms` 毫秒（真实时间：GIF 按墙上时钟换帧），再 pump 一次
    fn pump(&mut self, ms: u64) -> u32 {
        std::thread::sleep(Duration::from_millis(ms));
        let now = self.clock.elapsed().as_millis() as u64 + 1;
        self.e.pump_ex(now)
    }

    /// 屏幕上 GIF 那块的指纹（按当前滚动位置换算；四边各让 4 个物理像素）
    fn gif_hash(&mut self) -> u64 {
        let scroll = self.e.scroll_position();
        let y0 = ((GIF_TOP - scroll) * DPR) as usize + 4;
        let side = (GIF_SIZE * DPR) as usize - 8;
        let stride = (W as f32 * DPR) as usize * 4;
        let px = self.e.rgba();
        let mut h = DefaultHasher::new();
        for row in y0..y0 + side {
            px[row * stride + 16..row * stride + 16 + side * 4].hash(&mut h);
        }
        h.finish()
    }
}

#[test]
fn 静止页面上的动图按自己的帧率换帧() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut r = Rig::new();
    assert!(r.e.wants_next_frame(), "看得见的动图要让宿主续帧，否则静止页面上它永远停在第一帧");
    let mut looks = HashSet::new();
    looks.insert(r.gif_hash());
    let (mut pumps, mut drawn) = (0, 0);
    let begin = Instant::now();
    while begin.elapsed() < Duration::from_millis(1000) {
        let f = r.pump(8);
        pumps += 1;
        assert!(f & FRAME_WANTS_MORE != 0, "第 {pumps} 次 pump：动图还在，不该让宿主停帧");
        if f & FRAME_NEW != 0 {
            drawn += 1;
            assert_eq!(r.e.last_frame_kind(), FrameKind::Damage, "只重画动图那块，不是整帧");
            looks.insert(r.gif_hash());
        }
    }
    assert!(looks.len() >= 4, "1 秒里 GIF（120ms 一帧）只出现了 {} 种画面", looks.len());
    // 120ms 一帧 → 1 秒换 8~9 次。每个 vsync 都重画的话 drawn ≈ pumps
    assert!(drawn <= 16, "{pumps} 次 pump 里出了 {drawn} 帧新画面：GIF 没换帧的时候也在重画");
}

#[test]
fn 滚动时动图照常换帧() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut r = Rig::new();
    let mut looks = HashSet::new();
    let mut kinds = Vec::new();
    // 每帧滚 1px（整数逻辑像素 × DPR 2 = 整数物理像素，抠图位置精确），约 1 秒
    for i in 1..=60 {
        r.e.scroll_to(i as f32);
        let f = r.pump(16);
        if f & FRAME_NEW != 0 {
            kinds.push(r.e.last_frame_kind());
            looks.insert(r.gif_hash());
        }
    }
    assert!(
        kinds.contains(&FrameKind::Compose),
        "空转守卫：滚动中应有只合成的帧（没走到滚动的便宜路径就没测到这个 bug），实际 {kinds:?}"
    );
    assert!(kinds.contains(&FrameKind::Damage), "滚动中应有重画 GIF 那块的局部帧，实际 {kinds:?}");
    assert!(looks.len() >= 4, "滚动约 1 秒，GIF 只出现了 {} 种画面（滑动时停帧了）", looks.len());
}

#[test]
fn 拖动页面时动图照常换帧() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let mut r = Rig::new();
    let mut kinds = Vec::new();
    let mut gif_frames = 0;
    r.e.pointer_down(200.0, 500.0);
    r.pump(16);
    for i in 1..=50 {
        r.e.pointer_move(200.0, 500.0 - 2.0 * i as f32);
        let f = r.pump(16);
        assert!(f & FRAME_WANTS_MORE != 0, "拖动中要续帧");
        if f & FRAME_NEW != 0 {
            let k = r.e.last_frame_kind();
            gif_frames += (k == FrameKind::Damage) as u32;
            kinds.push(k);
        }
    }
    r.e.pointer_up(200.0, 400.0);
    assert!(r.e.scroll_position() > 20.0, "拖动应滚起页面，实际 {}", r.e.scroll_position());
    // 800ms 的拖动里 GIF 换 6 次左右；每一次都要有一帧重画它（只合成的话画面停在旧帧）
    assert!(gif_frames >= 4, "拖动中只有 {gif_frames} 帧重画了 GIF，帧种类 {kinds:?}");
}
