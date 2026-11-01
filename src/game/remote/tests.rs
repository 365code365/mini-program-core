//! 回放端的单测：Rust 里按线格式拼命令（与 wg-remote.js 的编码器同一格式），回放后看像素 / 返回值
use super::*;
use crate::game::GameOptions;

pub(crate) enum A<'a> {
    U,
    I(i32),
    F(f64),
    S(&'a str),
    B(bool),
    Bytes(u8, Vec<u8>),
    Arr(Vec<A<'a>>),
}

fn put(out: &mut Vec<u8>, a: &A) {
    match a {
        A::U => out.push(0),
        A::B(b) => out.push(if *b { 3 } else { 2 }),
        A::F(f) => {
            out.push(4);
            out.extend_from_slice(&f.to_le_bytes());
        }
        A::I(i) => {
            out.push(5);
            out.extend_from_slice(&i.to_le_bytes());
        }
        A::S(s) => {
            out.push(6);
            out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        A::Bytes(k, b) => {
            out.push(7);
            out.push(*k);
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(b);
        }
        A::Arr(v) => {
            out.push(8);
            out.extend_from_slice(&(v.len() as u32).to_le_bytes());
            for x in v {
                put(out, x);
            }
        }
    }
}

pub(crate) fn call(out: &mut Vec<u8>, f: u8, sync: bool, args: &[A]) {
    out.push(f);
    out.push(sync as u8);
    out.extend_from_slice(&(args.len() as u16).to_le_bytes());
    for a in args {
        put(out, a);
    }
}

/// 建游戏实例会改进程级的数据命名空间**与包根**（game/mod.rs 的 set_app_path）：两把锁都要拿（同 ffi_game/tests.rs）。
/// 从前只拿了命名空间锁：并行跑的 MiniEngine 用例在 launch 之前包根被换成这里的临时目录，
/// 报「app.js 不存在 … 页面不存在」（2026-11-01 全量撞上 engine_input_tests 的 picker 用例，单跑必过）
fn locks() -> (std::sync::MutexGuard<'static, ()>, std::sync::RwLockWriteGuard<'static, ()>) {
    (crate::data_dir::SCOPE_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner()),
     crate::data_dir::ROOT_TEST_LOCK.write().unwrap_or_else(|e| e.into_inner()))
}

fn engine(name: &str) -> GameEngine {
    let dir = std::env::temp_dir().join(format!("remote-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("game.json"), "{}").unwrap();
    // 回放端不执行 game.js：写一行会抛错的，确认它真的没跑
    std::fs::write(dir.join("game.js"), "throw new Error('不该执行');").unwrap();
    let opts = GameOptions { remote: true, fixed_clock: true, webgl: Some("swiftshader".into()), ..Default::default() };
    GameEngine::new(dir.to_str().unwrap(), None, 40, 30, 1.0, opts).unwrap()
}

fn errors(e: &mut GameEngine) -> Vec<String> {
    let mut v = Vec::new();
    while let Some(x) = e.poll_event() {
        if x.contains("\"type\":\"error\"") {
            v.push(x);
        }
    }
    v
}

#[test]
fn 回放二维_帧尾才上屏_不执行_game_js() {
    let _locks = locks();
    let mut e = engine("2d");
    assert!(errors(&mut e).is_empty(), "game.js 不该被执行");
    let mut b = Vec::new();
    call(&mut b, 2, false, &[]); // __game_canvas_new → 0（主画布）
    call(&mut b, 3, false, &[A::I(0), A::S("2d")]);
    call(&mut b, 1, false, &[A::I(0), A::I(30), A::S("#ff0000")]);
    call(&mut b, 1, false, &[A::I(0), A::I(1), A::F(0.0), A::F(0.0), A::F(10.0), A::F(10.0)]);
    let r = e.replay(&b);
    assert!(!r.present && !r.frame_end, "没有帧尾不上屏");
    let mut b = Vec::new();
    call(&mut b, F_PRESENT, false, &[]);
    let r = e.replay(&b);
    assert!(r.present && r.frame_end);
    let (w, _, px) = e.frame_rgba();
    assert_eq!(w, 40);
    assert_eq!(&px[0..4], &[255, 0, 0, 255]);
    assert_eq!(&px[(15 * 40 + 15) * 4..][..3], &[0, 0, 0], "没画到的地方压黑底");
    // 同步：measureText 的返回值按 JSON 带回
    let mut b = Vec::new();
    call(&mut b, 1, true, &[A::I(0), A::I(27), A::S("hi")]);
    let r = e.replay(&b);
    let v: serde_json::Value = serde_json::from_str(r.result.as_deref().unwrap()).unwrap();
    assert!(v["width"].as_f64().unwrap() > 0.0, "{v}");
    // 抛错的调用：同步时带回 $err
    let mut b = Vec::new();
    call(&mut b, 1, true, &[A::I(0), A::I(45), A::F(0.0), A::F(0.0), A::F(0.0), A::F(0.0)]);
    let r = e.replay(&b);
    assert!(r.result.unwrap().contains("$err"));
    // getImageData：像素以 base64 带回（大于 256 字节）
    let mut b = Vec::new();
    call(&mut b, 1, true, &[A::I(0), A::I(45), A::F(0.0), A::F(0.0), A::F(10.0), A::F(10.0)]);
    let v: serde_json::Value = serde_json::from_str(&e.replay(&b).result.unwrap()).unwrap();
    let px = crate::wxfs::codec::unb64(v["$b"].as_str().unwrap()).unwrap();
    assert_eq!(&px[..4], &[255, 0, 0, 255]);
}

#[cfg(feature = "webgl")]
#[test]
fn 回放_webgl_页面号换成_gl_名字_删除后忘掉() {
    let _locks = locks();
    let mut e = engine("gl");
    let names = crate::game::webgl::op_names();
    let op = |n: &str| names.iter().position(|x| *x == n).unwrap() as i32;
    let mut b = Vec::new();
    call(&mut b, 2, false, &[]);
    call(&mut b, 6, true, &[A::I(0), A::I(1), A::S("{}")]);
    let r = e.replay(&b);
    assert!(r.result.as_deref().is_some_and(|s| s.contains("alpha")), "建上下文：{r:?}");
    let c1 = (CLIENT_BASE + 1) as i32;
    let c2 = (CLIENT_BASE + 2) as i32;
    let mut b = Vec::new();
    call(&mut b, 0, false, &[A::I(0), A::I(op("createTexture")), A::I(c1)]);
    call(&mut b, 0, false, &[A::I(0), A::I(op("createBuffer")), A::I(c2)]);
    call(&mut b, 0, false, &[A::I(0), A::I(op("bindTexture")), A::I(0x0DE1), A::I(c1)]);
    call(&mut b, 0, true, &[A::I(0), A::I(op("getParameter")), A::I(0x8069)]); // TEXTURE_BINDING_2D
    let r = e.replay(&b);
    assert_eq!(r.result.as_deref(), Some(c1.to_string().as_str()), "绑定查询反向换回页面号");
    let mut b = Vec::new();
    call(&mut b, 0, false, &[A::I(0), A::I(op("deleteTexture")), A::I(c1)]);
    call(&mut b, 0, true, &[A::I(0), A::I(op("getParameter")), A::I(0x8069)]);
    let r = e.replay(&b).result.unwrap();
    assert!(r == "null" || r == "0", "删掉绑着的纹理之后绑定为空：{r}");
    assert!(!e.remote.as_ref().unwrap().ids.contains_key(&(0, c1 as u32)), "删掉之后页面号不再对应名字");
    assert!(e.remote.as_ref().unwrap().ids.contains_key(&(0, c2 as u32)));
    // 清屏成绿色 + 帧尾：主画布读回
    let mut b = Vec::new();
    call(&mut b, 0, false, &[A::I(0), A::I(op("clearColor")), A::F(0.0), A::F(1.0), A::F(0.0), A::F(1.0)]);
    call(&mut b, 0, false, &[A::I(0), A::I(op("clear")), A::I(0x4000)]);
    call(&mut b, F_PRESENT, false, &[]);
    let r = e.replay(&b);
    assert!(r.present, "{r:?}");
    let (_, _, px) = e.frame_rgba();
    assert_eq!(&px[..3], &[0, 255, 0]);
    let errs = errors(&mut e);
    assert!(errs.is_empty(), "{errs:?}");
}
