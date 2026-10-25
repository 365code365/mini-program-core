//! 兴趣圈 · 帖子详情（sample/forum `pages/post/post`）：页面不滚，滚的是一个定高的 `<scroll-view>`
//! （高 = 窗口高 − 底部输入条）。真机诊断 20261025-100253-98de9a：在它上面拖 / 惯性时每一帧都是
//! 「整帧·动画 整条带」，一帧画 6~20ms、GPU 再整张传 12MB，90Hz 屏上 22~43 帧/秒。
//!
//! 判据：按下开拖、拖动、拖到底还在推、松手、惯性，全程不出整帧（只局部重画这个 scroll-view 的盒子，
//! 底下的输入条、导航栏不跟着重画）。三处从前各自逼出整帧：engine_input.rs 松手、input.rs 锁定 scroll-view
//! 与到边界交给页面那一步、engine_damage.rs 一块大矩形被换成整条带。
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::MiniEngine;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[test]
fn 帖子详情_拖动内层_scroll_view_不出整帧() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/forum");
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forum-post-jank");
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0).expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(Some("pages/post/post?id=p100")).expect("打开帖子");
    let mut t = 1000u64;
    let mut log: Vec<(FrameKind, f32)> = Vec::new();
    let mut step = |e: &mut MiniEngine, log: &mut Vec<(FrameKind, f32)>| {
        t += 11;
        let s = Instant::now();
        let f = e.pump_ex(t) & FRAME_NEW;
        let ms = s.elapsed().as_secs_f32() * 1000.0;
        if f != 0 {
            log.push((e.last_frame_kind(), ms));
        }
    };
    // 真机那份帖子评论更多（楼中楼回复也多）：评论翻四倍，让 scroll-view 有几屏可滚
    e.eval("var p=getCurrentPages()[0]; p.raw=[].concat(p.raw,p.raw,p.raw,p.raw).map(function(f,i){return Object.assign({},f,{id:'x'+i, floor:i+1})}); p.render();").unwrap();
    for _ in 0..10 {
        step(&mut e, &mut log);
    }
    log.clear();
    // 在 scroll-view 中间慢慢往上拖（每次 480px、30 步，五遍：后两遍拖到底还在推），松手后让惯性走完
    let (x, y0) = (180.0, 650.0);
    for _ in 0..5 {
        e.pointer_down(x, y0);
        step(&mut e, &mut log);
        for i in 1..=30 {
            std::thread::sleep(Duration::from_millis(8));
            e.pointer_move(x, y0 - 16.0 * i as f32);
            step(&mut e, &mut log);
        }
        e.pointer_up(x, y0 - 480.0);
        for _ in 0..60 {
            std::thread::sleep(Duration::from_millis(8));
            step(&mut e, &mut log);
        }
    }
    // 排查用：`FORUM_DUMP=路径` 把停下来那一帧存成 PNG（配 `MINI_NO_SCROLL_TILES=1` 跑一遍对照分块与逐帧画）
    if let Some(path) = std::env::var_os("FORUM_DUMP") {
        let (w, h) = e.pixel_size();
        image::save_buffer(path, e.rgba(), w, h, image::ColorType::Rgba8).unwrap();
    }
    let full: Vec<_> = log.iter().filter(|(k, _)| matches!(k, FrameKind::Full(_))).collect();
    let worst = log.iter().map(|(_, ms)| *ms).fold(0.0, f32::max);
    let summary: Vec<String> = log.iter().map(|(k, ms)| format!("{}:{ms:.1}", k.label())).collect();
    eprintln!("帧 {}，整帧 {}，最慢 {worst:.1}ms\n{}", log.len(), full.len(), summary.join(" "));
    assert!(log.len() > 20, "拖动没出帧：{summary:?}");
    assert!(full.is_empty(), "拖动 / 惯性期间整帧 {} 次：{summary:?}", full.len());
}

/// 分块缓存里画不出按压态：楼中楼回复（`.sub`，`hover-class="sub-hover"`）按住时这一帧逐帧画，
/// 底色从 #f7f8fa 变成 #eceef2；松手过了 `hover-stay-time` 回到原色。点下去照样触发 `replySub`
#[test]
fn 帖子详情_scroll_view_里按住回复有按压态_点了能回复() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/forum");
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forum-post-press");
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0).expect("创建引擎");
    e.set_animation_clock(0.0);
    e.launch(Some("pages/post/post?id=p100")).expect("打开帖子");
    let mut t = 1000u64;
    let mut pump = |e: &mut MiniEngine, n: u32| {
        for _ in 0..n {
            t += 16;
            std::thread::sleep(Duration::from_millis(16));
            e.pump_ex(t);
        }
    };
    pump(&mut e, 5);
    // 先滚一点（让 scroll-view 走分块），停稳
    e.pointer_down(180.0, 650.0);
    for i in 1..=10 {
        e.pointer_move(180.0, 650.0 - 12.0 * i as f32);
        pump(&mut e, 1);
    }
    e.pointer_up(180.0, 530.0);
    pump(&mut e, 90);
    let (w, h) = e.pixel_size();
    let px = |e: &mut MiniEngine, x: u32, y: u32| {
        let p = e.rgba();
        let i = ((y * w + x) * 4) as usize;
        (p[i], p[i + 1], p[i + 2])
    };
    // 找一块回复区（#f7f8fa）的中间
    let mut spot = None;
    'find: for y in (300..h - 400).step_by(6) {
        for x in (300..w - 100).step_by(6) {
            // 往下 25 行：跳过 `.subs` 的上内边距，落在第一条回复（`.sub`）里
            if px(&mut e, x, y) == (0xf7, 0xf8, 0xfa) && px(&mut e, x, y + 25) == (0xf7, 0xf8, 0xfa) {
                spot = Some((x, y + 25));
                break 'find;
            }
        }
    }
    let (sx, sy) = spot.expect("视口里该有一块楼中楼回复");
    let (lx, ly) = (sx as f32 / 3.0, sy as f32 / 3.0);
    e.pointer_down(lx, ly);
    pump(&mut e, 8);
    assert_eq!(px(&mut e, sx, sy), (0xec, 0xee, 0xf2), "按住回复区要画成 sub-hover");
    e.pointer_up(lx, ly);
    // hover-stay-time 70ms 按墙上时钟走：机器忙（全量回归并行跑）时多给几帧
    pump(&mut e, 40);
    assert_eq!(px(&mut e, sx, sy), (0xf7, 0xf8, 0xfa), "松手后回到原色");
    let ph = e.eval("getCurrentPages()[0].data.placeholder").unwrap_or_default();
    assert!(ph.contains("回复"), "点回复区要进 replySub（输入框提示换成「回复 xx」）：{ph}");
}

/// 分块与局部重画叠在一起：开 / 关局部重画的两个引擎喂同一串拖动（慢拖、停住再松手，不带惯性），
/// 滚到同一位置的每一帧合成结果逐字节相同（块的位置只取决于块号，两边各自建块也一样）
#[test]
fn 帖子详情_分块滚动_局部重画与整帧逐字节一致() {
    use mini_render::compositor::HostEnv;
    use mini_render::host::EngineOptions;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample/forum");
    let mk = |no_damage: bool| {
        let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(if no_damage { "forum-eq-b" } else { "forum-eq-a" });
        std::fs::remove_dir_all(&data).ok();
        let env = HostEnv { no_damage, ..HostEnv::default() };
        let mut e = MiniEngine::new_with_options(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0, env, EngineOptions::default())
            .expect("创建引擎");
        e.set_animation_clock(0.0);
        e.launch(Some("pages/post/post?id=p100")).expect("打开帖子");
        e
    };
    let (mut a, mut b) = (mk(false), mk(true));
    let mut t = 1000u64;
    let mut frames = 0;
    let mut step = |a: &mut MiniEngine, b: &mut MiniEngine, what: &str| {
        t += 16;
        let fa = a.pump_ex(t) & FRAME_NEW;
        let fb = b.pump_ex(t) & FRAME_NEW;
        // 拖动的位置有一点按墙上时钟走（事件时间采样），两个引擎偶尔差半个像素：只比位置相同的帧
        let same_pos = a.scroll_view_offset("scroll-view_0_0") == b.scroll_view_offset("scroll-view_0_0");
        if (fa != 0 || fb != 0) && same_pos {
            frames += 1;
            if a.rgba() != b.rgba() {
                let (w, _) = a.pixel_size();
                let (pa, pb) = (a.rgba(), b.rgba());
                let diff: Vec<usize> = (0..pa.len() / 4).filter(|i| pa[i * 4..i * 4 + 4] != pb[i * 4..i * 4 + 4]).collect();
                let (y0, y1) = (diff[0] / w as usize, diff[diff.len() - 1] / w as usize);
                panic!("{what}（第 {frames} 帧）：局部那边「{}」与整帧不一致：{} 个像素，行 {y0}..{y1}；偏移 {:?} / {:?}", a.last_frame_kind().label(), diff.len(), a.scroll_view_offset("scroll-view_0_0"), b.scroll_view_offset("scroll-view_0_0"));
            }
        }
    };
    for _ in 0..5 {
        step(&mut a, &mut b, "首屏");
    }
    // 往上拖两段、往回拖一段；每步 7px（物理 21px，跨块边界好几次），停 200ms 再松手
    for (from, to) in [(700.0f32, 200.0f32), (700.0, 250.0), (250.0, 600.0)] {
        a.pointer_down(180.0, from);
        b.pointer_down(180.0, from);
        step(&mut a, &mut b, "按下");
        let n = ((to - from).abs() / 7.0) as i32;
        for i in 1..=n {
            let y = from + (to - from) * i as f32 / n as f32;
            a.pointer_move(180.0, y);
            b.pointer_move(180.0, y);
            step(&mut a, &mut b, "拖动");
        }
        std::thread::sleep(Duration::from_millis(200));
        a.pointer_up(180.0, to);
        b.pointer_up(180.0, to);
        for _ in 0..40 {
            step(&mut a, &mut b, "松手后");
        }
    }
    assert!(frames > 60, "位置相同、比过的帧太少：{frames}");
}

/// 排查用（`PROBE_APP=restaurant PROBE_PATH=pages/menu/menu`）：打开页面拖两下，不判
#[test]
fn 排查_任意页面拖两下() {
    let (Some(app), Some(path)) = (std::env::var_os("PROBE_APP"), std::env::var("PROBE_PATH").ok()) else { return };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sample").join(app);
    let data = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("probe");
    std::fs::remove_dir_all(&data).ok();
    let mut e = MiniEngine::new(&root.to_string_lossy(), Some(&data.to_string_lossy()), 360, 785, 3.0).expect("创建引擎");
    e.launch(Some(&path)).expect("打开");
    let mut t = 1000u64;
    for _ in 0..10 { t += 16; e.pump_ex(t); }
    e.pointer_down(250.0, 600.0);
    for i in 1..=20 { e.pointer_move(250.0, 600.0 - 15.0 * i as f32); t += 16; std::thread::sleep(Duration::from_millis(8)); e.pump_ex(t); }
    e.pointer_up(250.0, 300.0);
    for _ in 0..20 { t += 16; std::thread::sleep(Duration::from_millis(8)); e.pump_ex(t); }
}
