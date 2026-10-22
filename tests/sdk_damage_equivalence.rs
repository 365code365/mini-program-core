//! SDK 局部重画（`host/engine_damage.rs`）与整帧重画**逐帧逐字节一致**。
//!
//! 同一串输入喂给两个引擎：A 走局部重画（只合成 / 局部 / 补条带），B 设 `MINI_NO_DAMAGE`
//! （有变化就三层全画 —— 引入局部重画之前的行为）。每一帧两边的合成结果必须一字节不差。
//! 局部重画少画一块的代价是残留脏像素（滑动时一闪、滑回来露出旧内容），肉眼很难稳定复现，
//! 所以只能这样逐帧比。
//!
//! 输入全是确定的：页面滚动用 `scroll_to`（不经过手势速度，松手惯性按真实时间算会两边不一样），
//! 动画时钟逐帧由测试给（`set_animation_clock`，不强制整帧），setData 用 `eval` 在同一帧下发。
//! scroll-view 那段拖动用手势，但松手前原地停够 100ms，速度为 0，没有惯性。
//!
//! 还有一道空转守卫：A 里「局部」「补条带」「只合成」都必须真的出现过，否则就是两边都在
//! 整帧、比了个寂寞（`damage-check` 第一版就这么空转过）。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;

const W: u32 = 375;
const H: u32 = 667;
const DPR: f32 = 2.0;

/// 小程序根目录与组件共享状态是进程级的：用例串行跑，各用各的夹具目录
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fixture() -> PathBuf {
    fixture_in("sdk-damage-equivalence")
}

fn fixture_in(dir: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir);
    std::fs::remove_dir_all(&root).ok();
    let page_wxml = r#"<view class="hd">顶部 {{count}}</view>
<view class="spin"></view>
<scroll-view scroll-y class="sv"><view wx:for="{{inner}}" wx:key="*this" class="srow">内层第 {{item}} 行</view></scroll-view>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>
<view class="mid">中间 {{count}}</view>
<view class="pulse"></view>
<view wx:for="{{rows}}" wx:key="*this" class="row">下半第 {{item}} 行</view>
<view class="bar">底栏 {{count}}</view>"#;
    let page_wxss = ".hd { height: 40px; line-height: 40px; padding: 0 16px; background-color: #ffffff; }
.spin { width: 40px; height: 40px; margin: 10px; background-color: #07c160; animation: spin 2s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.sv { height: 200px; background-color: #fafafa; }
.srow { height: 40px; line-height: 40px; padding: 0 12px; border-bottom: 1px solid #eeeeee; }
.row { height: 44px; line-height: 44px; padding: 0 16px; border-bottom: 1px solid #eeeeee; background-color: #ffffff; }
.mid { height: 60px; line-height: 60px; padding: 0 16px; background-color: #fff3e0; }
.pulse { width: 120px; height: 24px; margin: 8px 16px; background-color: #ff6600; animation: pulse 1s linear infinite; }
@keyframes pulse { from { opacity: 0.2; } to { opacity: 1; } }
.bar { position: fixed; left: 0; right: 0; bottom: 0; height: 50px; line-height: 50px; padding: 0 16px; background-color: #333333; color: #ffffff; }";
    let page_js = "const n = k => Array.from({length: k}, (_, i) => i + 1);
Page({ data: { count: 0, rows: n(30), inner: n(12) } });";
    for (rel, text) in [
        (
            "app.json",
            r#"{"pages":["pages/p/p","pages/q/q"],"tabBar":{"list":[{"pagePath":"pages/p/p","text":"首页"},{"pagePath":"pages/q/q","text":"我的"}]}}"#,
        ),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", page_wxml),
        ("pages/p/p.wxss", page_wxss),
        ("pages/p/p.js", page_js),
        ("pages/p/p.json", "{}"),
        ("pages/q/q.wxml", "<view>第二页</view>"),
        ("pages/q/q.wxss", ""),
        ("pages/q/q.js", "Page({});"),
        ("pages/q/q.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, no_damage: bool) -> MiniEngine {
    engine_at(root, no_damage, DPR)
}

fn engine_at(root: &PathBuf, no_damage: bool, dpr: f32) -> MiniEngine {
    let data = root.join("_data");
    // `MINI_DAMAGE_LOG=1 cargo test --test sdk_damage_equivalence -- --nocapture`：
    // 局部那一侧每帧打一行做了什么（不一致时先看是哪一种帧）
    let damage_log = !no_damage && std::env::var("MINI_DAMAGE_LOG").is_ok();
    let env = HostEnv { no_damage, damage_log, ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(
        &root.to_string_lossy(),
        Some(&data.to_string_lossy()),
        W,
        H,
        dpr,
        env,
        EngineOptions::default(),
    )
    .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

/// 两个引擎同步推进，每一帧逐字节比对
struct Pair {
    a: MiniEngine,
    b: MiniEngine,
    t: u64,
    frame: u32,
    kinds: Vec<FrameKind>,
    /// 参照那边出了帧、局部那边判定没变化没出的次数（动画滚出裁剪窗后停掉）
    idle_while_ref_drew: u32,
}

impl Pair {
    fn both(&mut self, f: impl Fn(&mut MiniEngine)) {
        f(&mut self.a);
        f(&mut self.b);
    }

    fn step(&mut self, what: &str) {
        self.t += 16;
        self.frame += 1;
        let secs = self.t as f32 / 1000.0;
        let t = self.t;
        self.both(|e| e.set_animation_clock(secs));
        let fa = self.a.pump_ex(t) & FRAME_NEW;
        let fb = self.b.pump_ex(t) & FRAME_NEW;
        // 局部那边可以少出帧（动画滚出了裁剪窗就停掉，不再空转），但不能多出：
        // 参照那边有变化就三层全画，它不出帧说明什么都没变
        assert!(
            fa == 0 || fb != 0,
            "第 {} 帧（{what}）：参照那边没出新画面，局部那边却出了（{}）",
            self.frame,
            self.a.last_frame_kind().label()
        );
        assert_eq!(
            self.a.scroll_position(),
            self.b.scroll_position(),
            "第 {} 帧（{what}）：两边滚动位置分叉了，输入不确定，后面比不了",
            self.frame
        );
        if fb == 0 {
            return;
        }
        if fa == 0 {
            // 局部那边判定「看得见的东西没变」：屏上还是上一帧，必须与参照这一帧一样
            self.idle_while_ref_drew += 1;
        } else {
            self.kinds.push(self.a.last_frame_kind());
        }
        let pa = self.a.rgba().to_vec();
        let pb = self.b.rgba();
        if pa.as_slice() != pb {
            let diff = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
            let pw = self.a.pixel_size().0;
            let (x, y) = (diff as u32 % pw, diff as u32 / pw);
            // 哪几行不一样（横线 = 一两行整行不一样；残留 = 一大块）
            let row = (pw * 4) as usize;
            let rows: Vec<usize> = (0..pa.len() / row).filter(|&r| pa[r * row..(r + 1) * row] != pb[r * row..(r + 1) * row]).collect();
            let px = |v: &[u8]| (v[diff * 4], v[diff * 4 + 1], v[diff * 4 + 2]);
            panic!(
                "第 {} 帧（{what}，局部那边做的是「{}」，滚动 {}）：合成结果不一致，第一个差异像素在 ({x},{y})：\
                 局部 {:?} / 整帧 {:?}；不一致的行（屏幕设备像素）{:?}",
                self.frame,
                if fa == 0 { "没出帧" } else { self.a.last_frame_kind().label() },
                self.a.scroll_position(),
                px(&pa),
                px(pb),
                &rows[..rows.len().min(12)],
            );
        }
    }
}

#[test]
fn 局部重画与整帧重画逐帧逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture();
    mini_render::renderer::components::reset_shared_component_state();
    let mut p = Pair {
        a: engine(&root, false),
        b: engine(&root, true),
        t: 1,
        frame: 0,
        kinds: Vec::new(),
        idle_while_ref_drew: 0,
    };
    for _ in 0..4 {
        p.step("首屏 + 动画");
    }

    // ① 慢慢往下滚（手指上推）：动画在窗里转、一路滑出条带、中途 setData
    for i in 1..=60 {
        let y = i as f32 * 14.0;
        p.both(|e| e.scroll_to(y));
        if i == 20 {
            p.both(|e| {
                e.eval("getCurrentPages()[0].setData({count: 1})").unwrap();
            });
        }
        p.step("往下滚");
    }
    // ② 快速往回滚：一次跳一大段（跨好几个条带）
    for i in (0..12).rev() {
        let y = i as f32 * 70.0;
        p.both(|e| e.scroll_to(y));
        p.step("往回滚");
    }
    // ③ 停在中间 setData（底栏在覆盖层里，「中间」那行可能在窗外）
    p.both(|e| e.scroll_to(300.0));
    p.step("停在中间");
    p.both(|e| {
        e.eval("getCurrentPages()[0].setData({count: 2})").unwrap();
    });
    for _ in 0..3 {
        p.step("setData 之后");
    }
    // ④ 回到顶上拖内层 scroll-view（盒子在 y≈100..300）：只重画它自己那块
    p.both(|e| e.scroll_to(0.0));
    p.step("回到顶");
    p.both(|e| e.pointer_down(150.0, 260.0));
    p.step("按下 scroll-view");
    for i in 1..=10 {
        let y = 260.0 - 12.0 * i as f32;
        p.both(|e| e.pointer_move(150.0, y));
        p.step("拖 scroll-view");
    }
    // 原地停够 100ms：松手速度为 0，没有惯性（两边的惯性按真实时间算，会分叉）
    for _ in 0..12 {
        p.both(|e| e.pointer_move(150.0, 140.0));
        p.step("停住");
    }
    p.both(|e| e.pointer_up(150.0, 140.0));
    for _ in 0..4 {
        p.step("松手");
    }
    // ⑤ 一路滚到底再回来（底部那段的动画、底栏）
    for i in 1..=20 {
        let y = i as f32 * 90.0;
        p.both(|e| e.scroll_to(y));
        p.step("滚到底");
    }
    for i in (0..20).rev() {
        p.both(|e| e.scroll_to(i as f32 * 45.0));
        p.step("再回来");
    }

    // 空转守卫：局部重画那几条路必须真的走到过
    let count = |k: fn(&FrameKind) -> bool| p.kinds.iter().filter(|x| k(x)).count();
    let damage = count(|k| *k == FrameKind::Damage);
    let band = count(|k| *k == FrameKind::Band);
    let compose = count(|k| *k == FrameKind::Compose);
    let full = count(|k| matches!(k, FrameKind::Full(_)));
    let mut labels: Vec<(&str, usize)> = Vec::new();
    for k in &p.kinds {
        match labels.iter_mut().find(|(l, _)| *l == k.label()) {
            Some((_, n)) => *n += 1,
            None => labels.push((k.label(), 1)),
        }
    }
    eprintln!(
        "A 侧 {} 帧：{labels:?}；参照出帧而局部判定没变化 {} 次",
        p.kinds.len(),
        p.idle_while_ref_drew
    );
    assert!(damage >= 30, "局部重画只有 {damage} 帧，测试没走到要测的路径");
    assert!(band >= 3, "补条带只有 {band} 帧");
    assert!(compose >= 1 || damage > 0, "只合成 {compose} 帧");
    assert!(full * 4 < p.kinds.len(), "整帧占了 {full}/{} 帧：局部重画基本没生效", p.kinds.len());
}

/// 真机上的情形：dpr 3、滚动位置带小数（松手惯性每帧走的不是整数）。条带边、补画的那几截
/// 都落在半个像素上。从前清屏（`clear_area`：上沿向下取整、下沿向上取整）比裁剪着重画
/// （`Canvas::clip_rect` 下沿截断）多清一行：那一行变成页面底色，快速往回滑时屏上一条横线
/// （荣耀 Android 15 真机录屏：绿按钮中间一条白线）。上面那个用例 dpr 2 + 整数滚动，边永远落在整像素上，测不出来
#[test]
fn dpr_3_小数滚动位置_补条带不留底色横线() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture_in("sdk-damage-equivalence-dpr3");
    mini_render::renderer::components::reset_shared_component_state();
    let mut p = Pair {
        a: engine_at(&root, false, 3.0),
        b: engine_at(&root, true, 3.0),
        t: 1,
        frame: 0,
        kinds: Vec::new(),
        idle_while_ref_drew: 0,
    };
    p.step("首屏");
    // 往下：一帧 23.37（×3 = 70.11 设备像素），窗边每帧都落在小数上
    for i in 1..=50 {
        let y = i as f32 * 23.37;
        p.both(|e| e.scroll_to(y));
        p.step("小数往下滚");
    }
    // 快速往回（补的是上面那一截，横线出在那一截的下沿）
    for i in (0..30).rev() {
        let y = i as f32 * 38.61 + 0.37;
        p.both(|e| e.scroll_to(y));
        p.step("小数往回滚");
    }
    let band = p.kinds.iter().filter(|k| **k == FrameKind::Band).count();
    assert!(band >= 3, "补条带只有 {band} 帧，测试没走到要测的路径（{:?}）", p.kinds);
}
