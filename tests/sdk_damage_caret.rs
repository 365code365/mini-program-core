//! 聚焦输入框的光标闪烁：每次翻转只重画输入框那一块（engine_press.rs `damage_caret`），
//! 与整帧重画（`MINI_NO_DAMAGE`）**逐帧逐字节一致**。
//!
//! 从前光标每翻一次就 `mark_full`：键盘弹着的整段时间里每 500ms 整条带 + 覆盖层 + tabBar 重画一次。
//! 光标按墙上时钟闪（`cursor_blink_visible`），两个引擎共用同一个相位零点；
//! 一次 pump 恰好跨过翻转点时两边看到的相位可能不同，那一步不比（见 `step`）。
use mini_render::compositor::HostEnv;
use mini_render::host::engine_backend::FRAME_NEW;
use mini_render::host::engine_damage::FrameKind;
use mini_render::host::{EngineOptions, MiniEngine};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn fixture() -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("sdk-damage-caret");
    std::fs::remove_dir_all(&root).ok();
    let wxml = r#"<view class="hd">顶部</view>
<input class="ipt" value="{{v}}" placeholder="写点什么" />
<view wx:for="{{rows}}" wx:key="*this" class="row">第 {{item}} 行</view>
<view class="bar">底部固定栏 <input class="ipt2" placeholder="固定栏里的输入框" /></view>"#;
    let wxss = ".hd { height: 40px; background-color: #ffffff; }
.ipt { height: 40px; margin: 10px; padding: 0 8px; background-color: #f2f2f2; font-size: 16px; }
.row { height: 44px; line-height: 44px; padding: 0 16px; border-bottom: 1px solid #eeeeee; background-color: #ffffff; }
.bar { position: fixed; left: 0; right: 0; bottom: 0; height: 56px; background-color: #fafafa; }
.ipt2 { height: 36px; margin: 10px; background-color: #ffffff; }";
    let js = "Page({ data: { v: 'hello', rows: Array.from({length: 20}, (_, i) => i + 1) } });";
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

/// 点一下（两个引擎同一串输入），返回 A 有没有收到 `inputFocus`
fn tap(a: &mut MiniEngine, b: &mut MiniEngine, x: f32, y: f32) -> bool {
    for e in [&mut *a, &mut *b] {
        e.pointer_down(x, y);
        e.pointer_up(x, y);
    }
    let mut focused = false;
    while let Some(ev) = a.poll_event() {
        focused |= ev.contains("\"inputFocus\"");
    }
    while b.poll_event().is_some() {}
    focused
}

/// 两边各 pump 一帧再逐字节比。跨过光标翻转点的那一步（两边可能看到不同相位）跳过
fn blink_round(a: &mut MiniEngine, b: &mut MiniEngine, t: &mut u64, what: &str) -> Vec<FrameKind> {
    let mut kinds = Vec::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(1700) {
        std::thread::sleep(Duration::from_millis(40));
        *t += 16;
        let (fa, fb) = (a.pump_ex(*t) & FRAME_NEW, b.pump_ex(*t) & FRAME_NEW);
        if fa != 0 {
            kinds.push(a.last_frame_kind());
        }
        if fa == 0 && fb == 0 {
            continue;
        }
        let mut same = a.rgba().to_vec().as_slice() == b.rgba();
        if !same {
            // 两次 pump 之间恰好跨过翻转点：两边相位不同。紧接着再各 pump 一次，相位就对上了
            *t += 16;
            if a.pump_ex(*t) & FRAME_NEW != 0 {
                kinds.push(a.last_frame_kind());
            }
            b.pump_ex(*t);
            same = a.rgba().to_vec().as_slice() == b.rgba();
        }
        let (pa, pb) = (a.rgba().to_vec(), b.rgba());
        if !same {
            let i = pa.chunks_exact(4).zip(pb.chunks_exact(4)).position(|(x, y)| x != y).unwrap_or(0);
            let w = a.pixel_size().0 as usize;
            panic!("{what}（局部做的是「{}」）：第一个差异像素在 ({},{})", a.last_frame_kind().label(), i % w, i / w);
        }
    }
    kinds
}

#[test]
fn 光标闪烁只重画输入框_且逐字节一致() {
    let root = fixture();
    mini_render::renderer::components::reset_shared_component_state();
    let (mut a, mut b) = (engine(&root, false), engine(&root, true));
    let mut t = 1u64;
    for _ in 0..2 {
        t += 16;
        a.pump_ex(t);
        b.pump_ex(t);
    }
    // 页面里的输入框（y = 40 + 10 + 20）
    assert!(tap(&mut a, &mut b, 120.0, 70.0), "没聚焦到页面里的输入框（坐标落空，下面等于没测）");
    let kinds = blink_round(&mut a, &mut b, &mut t, "页面输入框");
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    let damage = kinds.iter().filter(|k| **k == FrameKind::Damage).count();
    // 空转守卫：1.7s 里至少翻转两次，每次都是局部
    assert!(damage >= 2, "光标翻转没走局部重画（局部 {damage} 帧）：{kinds:?}");
    // 聚焦那一下本身可能整帧（键盘 / 布局），之后的闪烁不许
    assert!(full <= 1, "光标闪烁不该整帧：{kinds:?}");

    // 覆盖层里的输入框：重画覆盖层，页面不动，照样逐字节一致
    assert!(tap(&mut a, &mut b, 120.0, 667.0 - 28.0), "没聚焦到固定栏里的输入框");
    let kinds = blink_round(&mut a, &mut b, &mut t, "固定栏输入框");
    let full = kinds.iter().filter(|k| matches!(k, FrameKind::Full(_))).count();
    assert!(full <= 1, "固定栏光标闪烁不该整帧：{kinds:?}");
}
