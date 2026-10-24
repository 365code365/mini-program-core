//! setData 让内容**变高**（列表 onReachBottom 追加）时只重画「最高的变化往下」那一截
//! （`renderer/wxml_renderer/invalidate_grow.rs` + `host/engine_page_pass.rs::grow_canvas_to_content`），
//! 而且与整帧重画逐帧逐字节一致。
//!
//! 从前内容高一变就整帧（换画布 + 整条带重画 + 整张重传）：一加 PLY110 上「节点与滚动」页每加载一次
//! 就是一个 14~38ms 的长帧。比法与 `sdk_damage_equivalence.rs` 相同：A 走局部重画，B 设
//! `MINI_NO_DAMAGE`（有变化就三层全画），同一串输入、每帧比合成结果。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;

static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 列表卡片：圆角 + 阴影 + 底边线；外面套一个有底色、圆角、阴影的容器（追加时它整个变高）；
/// 列表下面还有一行提示（追加时往下挪）、一条 fixed 底栏。`anim` 给追加的行带上入场动画
///
/// `root_level`：卡片直接摆在页面根上、插在「底部提示」与 fixed 悬浮按钮**前面**（「节点与滚动」页的写法）——
/// 按位置配对时追加点后面全错位
fn fixture(dir: &str, anim: bool, root_level: bool) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir);
    std::fs::remove_dir_all(&root).ok();
    // 顶上那行不随数据变：变了的话切线就在顶上，整条带都得重画（那是对的，但测不到要测的）
    let cards = r#"<view wx:for="{{rows}}" wx:key="*this" class="card {{item > 12 && anim ? 'fade' : ''}}">第 {{item}} 条 · 卡片内容</view>"#;
    // 带动画时列表底下还有个一直在转的加载圈（下载来的「头条新闻」底部的 refresh-ring）：追加之后它被挤到
    // 新的底下，旧位置要擦、新位置要画
    let ring = if anim { r#"<view class="ring"><view class="dot"></view></view>"# } else { "" };
    let wxml = if root_level {
        format!(
            r#"<view class="hd">列表</view>
{cards}
<view class="tip">{{{{tip}}}}</view>
<view class="fab" wx:if="{{{{rows.length > 14}}}}"><text>↑</text><text>{{{{rows.length}}}}</text></view>"#
        )
    } else {
        format!(
            r#"<view class="hd">列表</view>
<view class="list">{cards}</view>
<view class="tip">{{{{tip}}}}</view>
{ring}
<view class="bar">底栏 {{{{rows.length}}}}</view>"#
        )
    };
    let wxss = ".hd { height: 40px; line-height: 40px; padding: 0 16px; background-color: #e8f8ee; }
.list { margin: 12px; padding: 8px; background-color: #f2f2f2; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.15); }
.card { height: 56px; line-height: 56px; margin-bottom: 10px; padding: 0 14px; background-color: #ffffff; border-radius: 8px;
        border-bottom: 2px solid #07c160; box-shadow: 0 1px 4px rgba(0,0,0,0.2); }
.fade { animation: fade 0.5s linear; }
@keyframes fade { from { opacity: 0; } to { opacity: 1; } }
.ring { width: 40px; height: 40px; margin: 8px auto; border: 4px solid #07c160; border-radius: 6px; animation: spin 0.9s linear infinite; }
.dot { width: 12px; height: 12px; margin-left: 30px; background-color: #e64340; }
@keyframes spin { from { transform: rotate(0deg) translateY(-6px); } to { transform: rotate(360deg) translateY(-6px); } }
.tip { height: 44px; line-height: 44px; text-align: center; color: #999999; }
.bar { position: fixed; left: 0; right: 0; bottom: 0; height: 44px; background-color: #333333; color: #ffffff; }
.fab { position: fixed; right: 16px; bottom: 24px; width: 50px; height: 50px; border-radius: 25px; background-color: rgba(0,0,0,0.65); color: #ffffff; }";
    let js = format!(
        "const n = (a, b) => Array.from({{length: b - a + 1}}, (_, i) => a + i);
Page({{ data: {{ rows: n(1, 12), tip: '上拉加载更多', anim: {anim} }},
  more() {{ const r = this.data.rows; this.setData({{ rows: r.concat(n(r.length + 1, r.length + 5)), tip: '已加载 ' + (r.length + 5) + ' 条' }}); }} }});"
    );
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#.to_string()),
        ("app.js", "App({});".into()),
        ("app.wxss", "".into()),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss.into()),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}".into()),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    root
}

fn engine(root: &PathBuf, no_damage: bool) -> MiniEngine {
    let data = root.join(if no_damage { "_data_b" } else { "_data_a" });
    let env = HostEnv { no_damage, damage_log: !no_damage && std::env::var("MINI_DAMAGE_LOG").is_ok(), ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 3.0, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

struct Pair {
    a: MiniEngine,
    b: MiniEngine,
    t: u64,
    frame: u32,
    kinds: Vec<FrameKind>,
}

impl Pair {
    fn new(root: &PathBuf) -> Self {
        mini_render::renderer::components::reset_shared_component_state();
        Self { a: engine(root, false), b: engine(root, true), t: 1, frame: 0, kinds: Vec::new() }
    }

    fn both(&mut self, f: impl Fn(&mut MiniEngine)) {
        f(&mut self.a);
        f(&mut self.b);
    }

    fn step(&mut self, what: &str) {
        self.t += 16;
        self.frame += 1;
        let (t, secs) = (self.t, self.t as f32 / 1000.0);
        self.both(|e| e.set_animation_clock(secs));
        let fa = self.a.pump_ex(t) & FRAME_NEW;
        let fb = self.b.pump_ex(t) & FRAME_NEW;
        assert!(fa == 0 || fb != 0, "第 {} 帧（{what}）：参照没出帧，局部却出了", self.frame);
        assert_eq!(self.a.scroll_position(), self.b.scroll_position(), "第 {} 帧（{what}）：滚动位置分叉", self.frame);
        if fb == 0 {
            return;
        }
        if fa != 0 {
            self.kinds.push(self.a.last_frame_kind());
        }
        let pa = self.a.rgba().to_vec();
        let pb = self.b.rgba();
        if pa.as_slice() != pb {
            let row = (self.a.pixel_size().0 * 4) as usize;
            let rows: Vec<usize> = (0..pa.len() / row).filter(|&r| pa[r * row..(r + 1) * row] != pb[r * row..(r + 1) * row]).collect();
            // 第一行里哪几列不一样、两边各是什么颜色（看是谁留下的脏像素）
            let r0 = rows[0];
            let cols: Vec<usize> = (0..row / 4).filter(|&c| pa[r0 * row + c * 4..r0 * row + c * 4 + 4] != pb[r0 * row + c * 4..r0 * row + c * 4 + 4]).collect();
            let at = |v: &[u8], c: usize| (v[r0 * row + c * 4], v[r0 * row + c * 4 + 1], v[r0 * row + c * 4 + 2]);
            eprintln!(
                "第一行不一致的列 {}..{}（{} 列），局部 {:?} / 整帧 {:?}",
                cols[0],
                cols[cols.len() - 1],
                cols.len(),
                at(&pa, cols[0]),
                at(pb, cols[0])
            );
            panic!(
                "第 {} 帧（{what}，局部那边做的是「{}」，滚动 {}）：合成结果不一致，不一致的行 {:?}（共 {} 行）",
                self.frame,
                if fa == 0 { "没出帧" } else { self.a.last_frame_kind().label() },
                self.a.scroll_position(),
                &rows[..rows.len().min(12)],
                rows.len()
            );
        }
    }

    fn more(&mut self, what: &str) {
        self.both(|e| {
            e.eval("__currentPage.more()").unwrap();
        });
        self.step(what);
    }
}

#[test]
fn 列表追加_只重画变化往下那截_且逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    appends("sdk-damage-grow", false);
}

#[test]
fn 列表插在底部提示前面_配不上对的那段两边都走_且逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    appends("sdk-damage-grow-root", true);
}

fn appends(dir: &str, root_level: bool) {
    let root = fixture(dir, false, root_level);
    let mut p = Pair::new(&root);
    p.step("首屏");
    p.step("第二帧");
    // ① 滚到底再追加（onReachBottom 的样子）：新的几条接在视口下沿后面
    for i in 1..=30 {
        p.both(|e| e.scroll_to(i as f32 * 20.0));
        p.step("滚到底");
    }
    p.kinds.clear();
    p.more("到底追加");
    let append = p.kinds.clone();
    for i in 1..=25 {
        p.both(|e| e.scroll_to(600.0 + i as f32 * 16.0));
        p.step("滑进新加的那几条");
    }
    // ② 停在中间追加：变化全在裁剪窗下面
    p.both(|e| e.scroll_to(200.0));
    p.step("回到中间");
    p.more("中间追加");
    for _ in 0..3 {
        p.step("追加之后");
    }
    // ③ 连着追加两次、再一路滚到底
    p.more("连续追加 1");
    p.more("连续追加 2");
    for i in 1..=60 {
        p.both(|e| e.scroll_to(200.0 + i as f32 * 30.0));
        p.step("滚到新的底");
    }
    for i in (0..30).rev() {
        p.both(|e| e.scroll_to(i as f32 * 60.0));
        p.step("滚回顶");
    }
    // 空转守卫：追加那一帧真的走的是局部（不是整帧），且数据真的追加了
    assert!(
        append.iter().all(|k| !matches!(k, FrameKind::Full(_))) && !append.is_empty(),
        "到底追加那一帧没走局部重画：{append:?}"
    );
    let n = p.a.eval("String(__currentPage.data.rows.length)").unwrap_or_default();
    assert_eq!(n.trim(), "32", "追加没生效（12 条起，追加了 4 次、每次 5 条）");
    // 切线落在窗里靠上时要补的面积超过条带的一大半，宿主改走一趟整条带（`Full(Animation)` 那条分支，
    // 不换画布、不重排）—— 那是面积权衡，不是失效分析退回整帧；这里只不许出现 setData 逼出来的整帧
    let data_full = p.kinds.iter().filter(|k| matches!(k, FrameKind::Full(mini_render::host::engine_damage::FullReason::Data))).count();
    assert_eq!(data_full, 0, "追加不该整帧：{:?}", p.kinds);
}

/// 只有高度变了的容器：底边那两个大圆角（加阴影）在里面那个孩子的顶边**上面**就开始弯了。
/// 切线要让出「圆角 + 阴影」那一截，否则旧的圆角弧线留在新容器的直边上（`invalidate_grow.rs::bottom_band`）
///
/// 两种页面各跑一遍：`top` 600px 时整页超过一屏，孩子变高 = 内容变高（走切线）；`top` 200px 时整页不满
/// 一屏（根节点最少一屏高），内容高不变，走普通的收框 —— 那条路从前不算阴影，框外的旧阴影留着
fn grows_with_radius(dir: &str, top: u32) {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir);
    std::fs::remove_dir_all(&root).ok();
    let wxss = format!(
        ".top {{ height: {top}px; background-color: #e8f8ee; }}
.box {{ margin: 16px; background-color: #07c160; border-radius: 60px; box-shadow: 0 6px 16px rgba(0,0,0,0.35); }}
.body {{ height: 120px; }} .grow {{ background-color: #10aeff; }}"
    );
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#.to_string()),
        ("app.js", "App({});".into()),
        ("app.wxss", "".into()),
        ("pages/p/p.wxml", r#"<view class="top"></view><view class="box"><view class="body"></view><view class="grow" style="height: {{h}}px;"></view></view>"#.into()),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", "Page({ data: { h: 20 } });".into()),
        ("pages/p/p.json", "{}".into()),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let mut p = Pair::new(&root);
    p.step("首屏");
    // 滚到能看见盒子底边
    p.both(|e| e.scroll_to(top as f32 * 0.6));
    p.step("滚到盒子");
    p.kinds.clear();
    for h in [40, 90, 300] {
        p.both(|e| {
            e.eval(&format!("__currentPage.setData({{ h: {h} }})")).unwrap();
        });
        p.step("里面的孩子变高");
        p.step("之后");
    }
    assert!(p.kinds.iter().any(|k| *k == FrameKind::Damage), "变高没走局部重画：{:?}", p.kinds);
}

#[test]
fn 容器变高_底边大圆角让出那一截_逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    grows_with_radius("sdk-damage-grow-radius", 600);
}

#[test]
fn 内容高不变的盒子变高_阴影那圈也重画_逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    grows_with_radius("sdk-damage-grow-radius-short", 200);
}

/// 追加的行带入场动画、底下还有个一直在转的加载圈（绘制坐标≠布局坐标）：按整段动画最多画到框外多远
/// 取切线（invalidate_grow.rs `tail_reach`），不再整帧 —— 结果仍逐字节一致。
/// 从前这里一律整帧：下载来的「头条新闻」每次上拉加载更多都是一个 40~57ms 的 pump
#[test]
fn 追加的行带动画_底下有转圈_按动画外沿取切线_仍逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = fixture("sdk-damage-grow-anim", true, false);
    let mut p = Pair::new(&root);
    p.step("首屏");
    for i in 1..=30 {
        p.both(|e| e.scroll_to(i as f32 * 20.0));
        p.step("滚到底");
    }
    p.kinds.clear();
    p.more("追加带动画的行");
    let first = p.kinds.first().cloned();
    for i in 1..=40 {
        p.both(|e| e.scroll_to(600.0 + i as f32 * 12.0));
        p.step("动画中滑进新行");
    }
    p.more("再追加一次（转圈又往下挪）");
    for _ in 0..20 {
        p.step("转圈继续转");
    }
    use mini_render::host::engine_damage::FullReason;
    assert!(
        !matches!(first, Some(FrameKind::Full(FullReason::Data))),
        "带动画的追加不该因为 setData 整帧：{:?}",
        p.kinds
    );
    assert!(first.is_some(), "追加那一帧没出帧");
}

/// 页面根上一个 `wx:if` 的 fixed 悬浮按钮出现 / 消失（「节点与滚动」页滚过 200px 的那个）：
/// 从前「根节点数变化 → 整帧」，现在 fixed 的孩子单独配，只重画覆盖层（invalidate_pair.rs）。
/// 样式表里有 `:nth-child` 时 fixed 孩子一增一减会挪动后面兄弟的序号，不能单独配 —— 照旧整帧，结果仍一致
fn fab_toggles(dir: &str, nth_rule: bool) -> Vec<FrameKind> {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir);
    std::fs::remove_dir_all(&root).ok();
    let nth = if nth_rule { ".row:nth-child(2n) { background-color: #f6f6f6; }" } else { "" };
    let wxss = format!(
        ".row {{ height: 44px; line-height: 44px; padding: 0 16px; border-bottom: 1px solid #eeeeee; background-color: #ffffff; }}
{nth}
.fab {{ position: fixed; right: 16px; bottom: 24px; width: 50px; height: 50px; border-radius: 25px; background-color: rgba(0,0,0,0.65); color: #ffffff; }}"
    );
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#.to_string()),
        ("app.js", "App({});".into()),
        ("app.wxss", "".into()),
        (
            "pages/p/p.wxml",
            r#"<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>
<view class="fab" wx:if="{{show}}"><text>↑</text></view>
<view class="row">最后一行</view>"#
                .into(),
        ),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", "Page({ data: { show: false, rows: Array.from({length: 30}, (_, i) => i + 1) } });".into()),
        ("pages/p/p.json", "{}".into()),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let mut p = Pair::new(&root);
    p.step("首屏");
    p.both(|e| e.scroll_to(300.0));
    p.step("滚到中间");
    p.kinds.clear();
    for show in [true, false, true, false] {
        p.both(|e| {
            e.eval(&format!("__currentPage.setData({{ show: {show} }})")).unwrap();
        });
        p.step("悬浮按钮出现 / 消失");
        p.step("之后");
    }
    p.kinds
}

#[test]
fn fixed_悬浮按钮出现消失_只重画覆盖层_逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let kinds = fab_toggles("sdk-damage-fab", false);
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert_eq!(full, 0, "fixed 按钮出现 / 消失不该整页重画：{kinds:?}");
    assert!(kinds.len() >= 4, "按钮没真的变（{kinds:?}）");
}

#[test]
fn 有_nth_child_规则时_fixed_孩子不单独配_退回整帧_仍逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let kinds = fab_toggles("sdk-damage-fab-nth", true);
    assert!(kinds.iter().any(|k| matches!(k, FrameKind::Full(_))), "有 :nth-child 时应退回整帧：{kinds:?}");
}
