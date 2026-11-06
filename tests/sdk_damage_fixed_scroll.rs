//! 覆盖层里的 scroll-view（点餐页「已选菜品」抽屉）：拖它时页面一个像素不动，只重画覆盖层里它那块
//! （host/engine_fixed_partial.rs），与整帧重画逐字节相同。
//!
//! 单独一个测试二进制：小程序根目录是进程级的，与 sdk_damage_nested_scroll.rs 的用例并行跑会串目录。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;
use std::time::Duration;
/// 小程序根目录是进程级的：本文件的用例一个一个跑
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn engine(root: &PathBuf, no_damage: bool) -> MiniEngine {
    let data = root.join(if no_damage { "_b" } else { "_a" });
    let env = HostEnv { no_damage, damage_log: !no_damage && std::env::var("MINI_DAMAGE_LOG").is_ok(), ..HostEnv::default() };
    let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 375, 667, 3.0, env, EngineOptions::default())
        .expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(None).expect("打开首页");
    e
}

/// 覆盖层里的 scroll-view（点餐页「已选菜品」抽屉）：拖它时页面一个像素不动，只重画覆盖层里它那块
/// （engine_fixed_partial.rs），与整帧重画逐字节相同。拖动位置有一点按墙上时钟走，只比两边位置相同的帧
#[test]
fn 覆盖层里的_scroll_view_拖动只重画它那块_与整帧逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-fixed-scroll");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="page"><view wx:for="{{rows}}" wx:key="*this" class="prow"><text>页面第 {{item}} 行</text></view></view>
<view class="mask"><view class="drawer"><text class="dt">已选菜品</text>
<scroll-view scroll-y class="dl"><view wx:for="{{rows}}" wx:key="*this" class="dr"><view class="pic" style="background: linear-gradient(135deg, #8fd3f4, #84fab0);"></view><text>菜品 {{item}}</text></view></scroll-view>
</view></view>"#;
    let wxss = ".page { background-color: #f5f6f8; }
.prow { height: 60px; margin: 6px 10px; background-color: #fff; border-radius: 8px; }
.mask { position: fixed; left: 0; top: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.45); }
.drawer { position: absolute; left: 0; right: 0; bottom: 0; height: 360px; background: #fff; border-radius: 16px 16px 0 0; }
.dt { display: block; height: 40px; line-height: 40px; padding: 0 16px; }
.dl { height: 300px; }
.dr { display: flex; flex-direction: row; align-items: center; height: 64px; padding: 0 16px; border-bottom: 1px solid #eee; }
.pic { width: 44px; height: 44px; border-radius: 8px; margin-right: 10px; }";
    let js = "Page({ data: { rows: Array.from({length: 24}, (_, i) => i + 1) } });";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", js),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    let (mut compared, mut kinds) = (0, Vec::new());
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str, kinds: &mut Vec<FrameKind>| {
        t += 16;
        std::thread::sleep(Duration::from_millis(8));
        let fa = a.pump_ex(t) & FRAME_NEW;
        let fb = b.pump_ex(t) & FRAME_NEW;
        if fa != 0 {
            kinds.push(a.last_frame_kind());
        }
        if (fa != 0 || fb != 0) && a.rgba() == b.rgba() {
            compared += 1;
        } else if fa != 0 || fb != 0 {
            // 位置不同（按时间采样差了半像素）时不比；位置相同却不一样才是错
            assert!(
                a.scroll_view_offsets() != b.scroll_view_offsets(),
                "{what}：两边位置一样，局部那边「{}」却与整帧不一致",
                a.last_frame_kind().label()
            );
        }
    };
    for _ in 0..4 {
        step(&mut a, &mut b, "首屏", &mut kinds);
    }
    kinds.clear();
    // 抽屉列表在屏幕底部 300px 里：从 y=600 往上拖 200px，停 150ms 再松手；再往回拖
    for (from, to) in [(600.0f32, 400.0f32), (420.0, 620.0)] {
        for e in [&mut a, &mut b] {
            e.pointer_down(180.0, from);
        }
        step(&mut a, &mut b, "按下", &mut kinds);
        for i in 1..=20 {
            let y = from + (to - from) * i as f32 / 20.0;
            for e in [&mut a, &mut b] {
                e.pointer_move(180.0, y);
            }
            step(&mut a, &mut b, "拖", &mut kinds);
        }
        std::thread::sleep(Duration::from_millis(150));
        for e in [&mut a, &mut b] {
            e.pointer_up(180.0, to);
        }
        for _ in 0..20 {
            step(&mut a, &mut b, "松手", &mut kinds);
        }
    }
    assert!(compared >= 10, "比过的帧太少：{compared}");
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert!(full == 0, "拖覆盖层里的列表不该整帧：{kinds:?}");
}

/// 返回上一页：被盖住那一页的画布与覆盖层是留着的（engine_keep_canvas.rs），返回那一帧不整页重画；
/// 盖着时逻辑层改了它的数据（播放进度、迷你条上的歌名）只重画那几块。每一帧都与整帧重画逐字节一致。
/// 网络图片 / 字体这类签名看不出来的变化不在这里测（那种情况照旧整页重画）
#[test]
fn 返回上一页_放回留着的画布_盖着时改的数据只重画那几块_与整帧逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-keep-canvas");
    std::fs::remove_dir_all(&root).ok();
    let a_wxml = r#"<view class="hd" style="background: linear-gradient(135deg, #3a2a5a, #1f6f5a);"><text class="t">歌单 {{n}}</text></view>
<view wx:for="{{rows}}" wx:key="*this" class="row"><text>第 {{item}} 首</text></view>
<view class="mp"><view class="cover spin"></view><text class="name">正在播 {{song}}</text></view>"#;
    let a_wxss = ".hd { height: 160px; opacity: 0.9; }
.t { color: #ffffff; font-size: 20px; }
.row { height: 56px; line-height: 56px; padding: 0 16px; border-bottom: 1px solid #eeeeee; }
.mp { position: fixed; left: 8px; right: 8px; bottom: 8px; height: 54px; background: rgba(255,255,255,0.98); border-radius: 27px; box-shadow: 0 3px 12px rgba(0,0,0,0.12); display: flex; flex-direction: row; align-items: center; padding: 0 12px; }
.cover { width: 40px; height: 40px; border-radius: 20px; background-color: #6a9cff; border: 3px solid #222222; }
.spin { animation: spin 12s linear infinite; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.name { margin-left: 10px; }";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/a/a","pages/b/b"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/a/a.wxml", a_wxml),
        ("pages/a/a.wxss", a_wxss),
        ("pages/a/a.js", "Page({ data: { n: 1, song: '江湖', rows: Array.from({length: 20}, (_, i) => i + 1) } });"),
        ("pages/a/a.json", "{}"),
        ("pages/b/b.wxml", r#"<view class="b">播放页 {{k}}</view>"#),
        ("pages/b/b.wxss", ".b { height: 600px; background-color: #14121e; color: #ffffff; }"),
        ("pages/b/b.js", "Page({ data: { k: 0 } });"),
        ("pages/b/b.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    let mut kinds = Vec::new();
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str, kinds: &mut Vec<FrameKind>| {
        t += 16;
        let secs = t as f32 / 1000.0;
        a.set_animation_clock(secs);
        b.set_animation_clock(secs);
        let (fa, fb) = (a.pump_ex(t) & FRAME_NEW, b.pump_ex(t) & FRAME_NEW);
        assert!(fa == 0 || fb != 0, "{what}：参照没出帧，局部却出了");
        if fb == 0 {
            return;
        }
        if fa != 0 {
            kinds.push(a.last_frame_kind());
        }
        let (pa, pb) = (a.rgba().to_vec(), b.rgba());
        if pa.as_slice() != pb {
            let i = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
            let w = a.pixel_size().0 as usize;
            panic!("{what}（局部做的是「{}」）：第一个差异像素在 ({},{})", a.last_frame_kind().label(), i % w, i / w);
        }
    };
    for i in 0..4 {
        step(&mut a, &mut b, &format!("歌单页 {i}"), &mut kinds);
    }
    for round in 0..2 {
        for e in [&mut a, &mut b] {
            e.eval("wx.navigateTo({ url: '/pages/b/b' })").unwrap();
        }
        for i in 0..6 {
            step(&mut a, &mut b, &format!("第 {round} 轮 播放页 {i}"), &mut kinds);
        }
        // 盖着时逻辑层改了歌单页的数据（全局播放器的订阅者照样 setData）
        let js = format!("getCurrentPages()[0].setData({{ song: '第 {round} 首', n: {} }})", 10 + round);
        for e in [&mut a, &mut b] {
            e.eval(&js).unwrap();
        }
        step(&mut a, &mut b, &format!("第 {round} 轮 盖着时改数据"), &mut kinds);
        kinds.clear();
        for e in [&mut a, &mut b] {
            assert!(e.back(), "退得回去");
        }
        for i in 0..8 {
            step(&mut a, &mut b, &format!("第 {round} 轮 返回之后 {i}"), &mut kinds);
        }
        let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
        assert_eq!(full, 0, "第 {round} 轮：返回那一帧不该整页重画：{kinds:?}");
        // 滚一下：条带外的那截照常补
        for e in [&mut a, &mut b] {
            e.scroll_to(300.0 * (round + 1) as f32);
        }
        for i in 0..4 {
            step(&mut a, &mut b, &format!("第 {round} 轮 滚动 {i}"), &mut kinds);
        }
    }
}

/// 后台解完的图（本地大图走 `image_net::get_or_load`，与远程图同一条路）：只重画用到它的那几块
/// （host/engine_image_arrival.rs），不整帧。四种位置：普通流（圆角 + 阴影）、横滑 scroll-view 里、
/// 带 transform 的子树里、覆盖层里。图都到齐之后的画面与「图早就在缓存里、整帧画出来」的新引擎逐字节相同
#[test]
fn 图片后台解完_只重画用到它的那几块_与图早就在时整帧画的逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-image-arrival");
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    // 超过 768KB 才走后台解码（image/load.rs 的 LARGE_FILE_BYTES）：噪声图压不小
    let mut seed = 0x2545_f491u32;
    for k in 1..=4 {
        let img = image::RgbaImage::from_fn(520, 520, |_, _| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            let b = seed.to_le_bytes();
            image::Rgba([b[0], b[1], b[2], 255])
        });
        img.save(root.join(format!("big{k}.png"))).unwrap();
        assert!(std::fs::metadata(root.join(format!("big{k}.png"))).unwrap().len() > 768 * 1024);
    }
    let wxml = r#"<view class="hd">顶部</view>
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>
<image class="a" src="/big1.png" />
<scroll-view scroll-x class="sv"><view wx:for="{{rows}}" wx:key="*this" class="cell"><image class="t" src="/big2.png" /></view></scroll-view>
<view class="tilt"><image class="t" src="/big3.png" /></view>
<view wx:for="{{rows}}" wx:key="*this" class="row">下半第 {{item}} 行</view>
<view class="fx"><image class="t" src="/big4.png" /><text>迷你播放条</text></view>"#;
    let wxss = ".hd { height: 40px; background-color: #ffffff; }
.row { height: 44px; line-height: 44px; padding: 0 16px; border-bottom: 1px solid #eeeeee; }
.a { width: 200px; height: 120px; margin: 12px; border-radius: 12px; box-shadow: 0 6px 14px rgba(0,0,0,0.4); }
.sv { height: 90px; white-space: nowrap; }
.cell { display: inline-block; width: 80px; height: 80px; margin: 5px; }
.t { width: 70px; height: 70px; border-radius: 8px; }
.tilt { width: 90px; height: 90px; margin: 30px; transform: rotate(12deg); background-color: #f0f4ff; }
.fx { position: fixed; left: 8px; right: 8px; bottom: 8px; height: 80px; background: rgba(255,255,255,0.95); display: flex; flex-direction: row; align-items: center; }";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", "Page({ data: { rows: Array.from({length: 6}, (_, i) => i + 1) } });"),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let partial0 = mini_render::host::image_arrival_partial_count();
    let mut a = engine(&root, false);
    let mut t = 1u64;
    let mut kinds = Vec::new();
    // 首帧（整帧、图还是占位）之后一直出帧到图都解完、再多出几帧
    let started = std::time::Instant::now();
    let mut settled = 0;
    while settled < 4 {
        t += 16;
        if a.pump_ex(t) & FRAME_NEW != 0 && t > 17 {
            kinds.push(a.last_frame_kind());
        }
        if t == 17 {
            // 首帧之后马上滚：图到的时候页面已经滚过一段，失效范围要按画布坐标算
            a.scroll_to(100.0);
        }
        if a.net_pending() {
            settled = 0;
        } else {
            settled += 1;
        }
        assert!(started.elapsed() < Duration::from_secs(20), "图一直没解完");
        std::thread::sleep(Duration::from_millis(5));
    }
    let partial = mini_render::host::image_arrival_partial_count() - partial0;
    assert!(partial >= 1, "图到了没走局部那条路（{kinds:?}）");
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert_eq!(full, 0, "图到了不该整帧：{kinds:?}");
    // 参照：图都在缓存里了，新开一个引擎整帧画同一个位置
    let mut b = engine(&root, true);
    for i in 0..4 {
        t += 16;
        b.pump_ex(t);
        if i == 0 {
            b.scroll_to(100.0);
        }
    }
    assert_eq!(a.scroll_position(), b.scroll_position(), "两边滚到同一个位置");
    let (pa, pb) = (a.rgba().to_vec(), b.rgba());
    if pa.as_slice() != pb {
        let i = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
        let w = a.pixel_size().0 as usize;
        panic!("图到齐之后与整帧画的不一致：第一个差异像素在 ({},{})（{kinds:?}）", i % w, i / w);
    }
}

/// 远程图 `mode="widthFix"`：下完才知道比例，框要按比例重排（engine_image_arrival.rs 的 Relayout）。
/// 从前只整帧重画、布局缓存照旧命中（数据没变），图下完了还按默认 240px 高的框画（压扁），下面的内容也不挪。
/// 图从本机起的一个小 HTTP 服务上取，判据：与「图早就在缓存里」新开的引擎逐字节相同
#[test]
fn 远程_width_fix_图下完_按比例重排() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-image-widthfix");
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    let mut png = Vec::new();
    image::RgbaImage::from_fn(200, 100, |x, y| image::Rgba([(x % 256) as u8, (y * 2) as u8, 160, 255]))
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        use std::io::{Read, Write};
        for s in listener.incoming().flatten() {
            let mut s = s;
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf);
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", png.len());
            let _ = s.write_all(&png);
        }
    });
    // 每次一个新地址：磁盘缓存 / 解码缓存都是进程级的，别吃到上一次的
    let url = format!("http://127.0.0.1:{port}/w{}.png", std::process::id());
    let wxml = format!(
        r#"<view class="hd">顶部</view><image class="w" mode="widthFix" src="{url}" /><view class="after">图下面的字</view>"#
    );
    let wxss = ".hd { height: 40px; } .w { width: 300px; } .after { height: 60px; background-color: #07c160; }";
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/p/p"]}"#),
        ("app.js", "App({});"),
        ("app.wxss", ""),
        ("pages/p/p.wxml", wxml.as_str()),
        ("pages/p/p.wxss", wxss),
        ("pages/p/p.js", "Page({ data: {} });"),
        ("pages/p/p.json", "{}"),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    let mut a = engine(&root, false);
    let mut t = 1u64;
    let started = std::time::Instant::now();
    let mut settled = 0;
    while settled < 4 {
        t += 16;
        a.pump_ex(t);
        settled = if a.net_pending() { 0 } else { settled + 1 };
        assert!(started.elapsed() < Duration::from_secs(20), "图一直没下完");
        std::thread::sleep(Duration::from_millis(5));
    }
    let mut b = engine(&root, true);
    for _ in 0..3 {
        t += 16;
        b.pump_ex(t);
    }
    // 下完了没有：参照那边的框是按比例排的（300×150），不是默认的 300×240
    let w = b.pixel_size().0 as usize;
    let (pa, pb) = (a.rgba().to_vec(), b.rgba().to_vec());
    if pa != pb {
        let i = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
        panic!("widthFix 图下完之后与整帧画的不一致：第一个差异像素在 ({},{})", i % w, i / w);
    }
    // 空转守卫：图下完之后 .after 的绿条在 40 + 150 = 190px 处（3 倍屏 570 行），不在 40 + 240 = 280px
    let px = |y: usize| &pb[(y * w + 30) * 4..(y * w + 30) * 4 + 4];
    assert_eq!(px(575), &[0x07, 0xc1, 0x60, 0xff][..], "绿条该从 190px 开始（图按 2:1 排成 300×150）");
}

/// 盖着时到的图：留着的画布不作废，返回时只重画用到这张图的那块（engine_keep_canvas.rs 的 `arrived`）。
/// 从前图一到留着的画布就作废，返回那一帧整页重画（播放页上新歌封面到了，歌单页返回 57ms，一加）。
/// 判据：返回那几帧没有整帧、走了图片局部那条路，且与整帧重画（参照引擎）逐字节相同
#[test]
fn 盖着时到的图_返回时只重画用到它的那块_与整帧逐字节一致() {
    let _g = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-keep-image");
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();
    let mut seed = 0x9e37_79b9u32;
    let img = image::RgbaImage::from_fn(520, 520, |_, _| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let b = seed.to_le_bytes();
        image::Rgba([b[0], b[1], b[2], 255])
    });
    // 每次一个新文件名：解码缓存是进程级的，别吃到上一次的
    let name = format!("cover{}.png", std::process::id());
    img.save(root.join(&name)).unwrap();
    let a_wxml = format!(
        r#"<view class="hd">歌单</view><image class="cv" src="/{name}" /><view wx:for="{{{{rows}}}}" wx:key="*this" class="row">第 {{{{item}}}} 首</view>"#
    );
    for (rel, text) in [
        ("app.json", r#"{"pages":["pages/a/a","pages/b/b"]}"#.to_string()),
        ("app.js", "App({});".into()),
        ("app.wxss", "".into()),
        ("pages/a/a.wxml", a_wxml),
        ("pages/a/a.wxss", ".hd { height: 60px; } .cv { width: 160px; height: 160px; margin: 10px; border-radius: 12px; } .row { height: 50px; border-bottom: 1px solid #eeeeee; }".into()),
        ("pages/a/a.js", "Page({ data: { rows: Array.from({length: 16}, (_, i) => i + 1) } });".into()),
        ("pages/a/a.json", "{}".into()),
        ("pages/b/b.wxml", r#"<view class="b">播放页</view>"#.into()),
        ("pages/b/b.wxss", ".b { height: 600px; background-color: #14121e; color: #ffffff; }".into()),
        ("pages/b/b.js", "Page({});".into()),
        ("pages/b/b.json", "{}".into()),
    ] {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    // 只开一个局部重画的引擎：图片的「到了」是进程级的，两个引擎一起跑时谁先 pump 谁收走（参照那边收走了，这边就测不到）。
    // 参照在最后新开：图已经在缓存里，整帧画出来就是对的样子
    let mut a = engine(&root, false);
    let mut t = 1u64;
    let mut kinds = Vec::new();
    let mut step = |a: &mut MiniEngine, kinds: &mut Vec<FrameKind>| {
        t += 16;
        if a.pump_ex(t) & FRAME_NEW != 0 {
            kinds.push(a.last_frame_kind());
        }
    };
    // 首帧：图还是占位（后台在解）。等它解完但先别 pump，接着跳走：这一帧先换页、后收图（engine_frame.rs 的顺序）
    step(&mut a, &mut kinds);
    let started = std::time::Instant::now();
    while mini_render::renderer::components::image_net::has_pending() {
        assert!(started.elapsed() < Duration::from_secs(20), "图一直没解完");
        std::thread::sleep(Duration::from_millis(5));
    }
    a.eval("wx.navigateTo({ url: '/pages/b/b' })").unwrap();
    for _ in 0..40 {
        step(&mut a, &mut kinds);
    }
    let partial0 = mini_render::host::image_arrival_partial_count();
    kinds.clear();
    assert!(a.back(), "退得回去");
    for _ in 0..60 {
        step(&mut a, &mut kinds);
    }
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert_eq!(full, 0, "盖着时到的图不该让返回那一帧整页重画：{kinds:?}");
    assert!(mini_render::host::image_arrival_partial_count() > partial0, "返回时没按到的图重画那一块：{kinds:?}");
    let mut b = engine(&root, true);
    for k in 0..4 {
        b.pump_ex(t + 16 * (k + 1));
    }
    let (pa, pb) = (a.rgba().to_vec(), b.rgba());
    if pa.as_slice() != pb {
        let i = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
        let w = a.pixel_size().0 as usize;
        panic!("返回之后与整帧画的不一致：第一个差异像素在 ({},{})（{kinds:?}）", i % w, i / w);
    }
}
