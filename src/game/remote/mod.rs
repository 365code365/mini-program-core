//! 高性能+ 模式的渲染端（game-doc/13-高性能加模式.md）：游戏 JS 在 WKWebView 里跑（JIT），
//! 画布 / WebGL 调用由页面录成一串命令（`js/web_game/wg-remote.js`），每帧一包交到这里回放，画面经 ANGLE 直接上屏。
//!
//! 为什么回放的是 `__game_gl` / `__game_2d` 这些原生入口、而不是另写一套：页面里装的就是普通模式那几层 JS
//! （game-canvas.js / game-image.js / game-webgl.js：WebGL 的对象归属、错误语义、重载都在那里），它们只认这几个原生名字。
//! 页面那边把原生名字换成「录下来」，这边原样调同名的原生 —— 两种模式走的是同一份实现，行为不会漂。
//!
//! 线格式（小端）：`call*`，`call = u8 函数 | u8 标志(1 = 要返回值) | u16 参数个数 | 参数*`，
//! 参数 = `u8 类型` + 值：0 undefined、1 null、2 false、3 true、4 f64、5 i32、6 字符串（u32 字节数 + UTF-8）、
//! 7 字节（u8 视图种类 + u32 字节数 + 内容）、8 数组（u32 个数 + 参数*）、9 JSON（u32 + UTF-8）。
//!
//! **对象名字**：页面不能为每个 createXxx 同步等原生回名字，于是自己发号（[`CLIENT_BASE`] 起），作为最后一个参数跟着 create 发过来；
//! 这边记下「页面号 → GL 名字」，之后 GL 调用里落在这个号段、且在表里的整数换成真名字。返回名字的查询（getParameter 的绑定类）反过来换。
//! 号段在 1.78e9 附近：GL 枚举、尺寸、偏移都碰不到它；uniform 的整数值恰好等于一个活着的页面号的概率可以忽略（取舍，写在这里）。

mod wire;

use super::GameEngine;
use rquickjs::{Ctx, Function, Value};
use std::collections::HashMap;

/// 页面发号的起点与跨度（wg-remote.js 的 CLIENT_BASE 与它一致）
pub const CLIENT_BASE: u32 = 0x6A5A_0000;
pub const CLIENT_SPAN: u32 = 0x0100_0000;

/// 函数编号 → 原生名字（wg-remote.js 的 FN 表与它一一对应）
pub const FNS: &[&str] = &[
    "__game_gl", "__game_2d", "__game_canvas_new", "__game_canvas_ctx", "__game_canvas_resize", "__game_canvas_free",
    "__game_gl_create", "__game_gl_resize", "__game_gl_sync", "__game_gl_free", "__game_gl_lose", "__game_gl_restore",
    "__game_img_new", "__game_img_src", "__game_img_free", "__game_load_font", "__game_text_line_height",
    "__game_canvas_encode", "__game_canvas_save", "__game_canvas_size",
    // 回放端自己的（remote_init 装的）：程序链接完的全部信息、一次多问
    "__wgrProgramInfo", "__wgrBatch",
];
const F_GL: u8 = 0;
const F_PROGRAM: u8 = 20;
#[allow(dead_code)]
const F_BATCH: u8 = 21;
/// 帧尾：页面这一帧的 rAF 跑完了，收尾并上屏
pub const F_PRESENT: u8 = 0xF0;

#[derive(Default)]
pub struct Remote {
    /// (画布 id, 页面号) → GL 名字
    ids: HashMap<(u32, u32), u32>,
    /// (画布 id, 种类, GL 名字) → 页面号。GL 的名字按种类各自从 1 数（纹理 1 与缓冲 1 并存），所以带种类
    rev: HashMap<(u32, &'static str, u32), u32>,
    pub presents: u64,
    pub calls: u64,
}

/// 一包命令回放的结果
#[derive(Debug, Default)]
pub struct ReplayOut {
    /// 最后一条要返回值的调用的结果（JSON，见 wire.rs 的 `__wgrEnc`）
    pub result: Option<String>,
    /// 这包以帧尾结束、且主画布这一帧画过：宿主要上屏
    pub present: bool,
    /// 这包里有帧尾（不管画没画）：宿主回页面一个 ack（流控）
    pub frame_end: bool,
    pub calls: u32,
}

/// 页面装好之后在回放端装的 JS：异步回调转发给页面、返回值编码、程序信息、批量查询
const INIT_JS: &str = include_str!("init.js");

impl GameEngine {
    pub(super) fn remote_init(&mut self) {
        if let Err(e) = self.app.eval(INIT_JS) {
            self.host_log(&format!("❌ 高性能+ 回放端初始化失败：{e}"));
        }
        self.after_js();
    }

    /// 回放一包命令（页面一帧的录制，或一次同步调用连同它之前没发出去的命令）
    pub fn replay(&mut self, buf: &[u8]) -> ReplayOut {
        let Some(mut st) = self.remote.take() else { return ReplayOut::default() };
        let mut out = ReplayOut::default();
        {
            let rt = self.app.js_runtime();
            let rt = rt.lock().unwrap();
            rt.raw_context().with(|ctx| {
                let mut fns: Vec<Option<Function>> = vec![None; FNS.len()];
                let mut pos = 0usize;
                while pos < buf.len() {
                    let Some((f, flags, argc)) = wire::head(buf, &mut pos) else { break };
                    if f == F_PRESENT {
                        out.frame_end = true;
                        continue;
                    }
                    let mut args = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        match wire::read(&ctx, buf, &mut pos) {
                            Some(v) => args.push(v),
                            None => {
                                pos = buf.len();
                                break;
                            }
                        }
                    }
                    out.calls += 1;
                    // 批量查询只用来问常量（不带对象），不换名字
                    let gl_family = f == F_GL || f == F_PROGRAM;
                    let cid = args.first().and_then(|v| v.as_number()).unwrap_or(-1.0) as u32;
                    let op = if f == F_GL { args.get(1).and_then(|v| v.as_int()).unwrap_or(-1) } else { -1 };
                    let op_name = gl_op_name(op);
                    let create_client = (f == F_GL && (op_name.starts_with("create") || op_name == "fenceSync"))
                        .then(|| args.last().and_then(|v| v.as_number()).map(|n| n as u32))
                        .flatten();
                    if gl_family {
                        let skip = if f == F_GL { 2 } else { 1 };
                        let end = if create_client.is_some() { args.len() - 1 } else { args.len() };
                        for v in args.iter_mut().take(end).skip(skip.min(end)) {
                            st.translate(&ctx, cid, v);
                        }
                    }
                    let Some(func) = get_fn(&ctx, &mut fns, f) else { continue };
                    let r: Value = match func.call((rquickjs::function::Rest(args.clone()),)) {
                        Ok(v) => v,
                        Err(_) => {
                            let e = ctx.catch();
                            let msg = e.as_exception().and_then(|x| x.message()).unwrap_or_else(|| format!("{e:?}"));
                            if flags & 1 != 0 {
                                out.result = Some(serde_json::json!({ "$err": msg }).to_string());
                            }
                            continue;
                        }
                    };
                    if let Some(client) = create_client {
                        if let Some(real) = r.as_number().filter(|n| *n > 0.0) {
                            st.ids.insert((cid, client), real as u32);
                            st.rev.insert((cid, kind_of(op_name), real as u32), client);
                        }
                    } else if f == F_GL && op_name.starts_with("delete") {
                        if let Some(real) = args.get(2).and_then(|v| v.as_number()) {
                            if let Some(client) = st.rev.remove(&(cid, kind_of(op_name), real as u32)) {
                                st.ids.remove(&(cid, client));
                            }
                        }
                    }
                    if flags & 1 != 0 {
                        let r = match (f == F_GL).then(|| returns_name(op_name, &args)).flatten() {
                            Some(kind) => st.untranslate(&ctx, cid, kind, r),
                            None => r,
                        };
                        out.result = Some(wire::encode(&ctx, r));
                    }
                }
            });
        }
        st.calls += out.calls as u64;
        self.remote = Some(st);
        self.after_js();
        if out.frame_end {
            #[cfg(feature = "webgl")]
            self.webgl_frame_end();
            if std::mem::take(&mut self.canvases.borrow_mut().main_dirty) {
                self.frame.stale = true;
                self.frame.count += 1;
                out.present = true;
            }
            if let Some(r) = self.remote.as_mut() {
                r.presents += 1;
            }
        }
        out
    }

    /// 回放端：是不是高性能+ 的渲染端
    pub fn is_remote(&self) -> bool {
        self.remote.is_some()
    }
}

impl Remote {
    fn translate<'js>(&self, ctx: &Ctx<'js>, cid: u32, v: &mut Value<'js>) {
        if let Some(n) = v.as_int() {
            let u = n as u32;
            if (CLIENT_BASE..CLIENT_BASE + CLIENT_SPAN).contains(&u) {
                if let Some(real) = self.ids.get(&(cid, u)) {
                    *v = Value::new_int(ctx.clone(), *real as i32);
                }
            }
        } else if let Some(arr) = v.as_array() {
            for i in 0..arr.len() {
                if let Ok(mut x) = arr.get::<Value>(i) {
                    self.translate(ctx, cid, &mut x);
                    let _ = arr.set(i, x);
                }
            }
        }
    }

    fn untranslate<'js>(&self, ctx: &Ctx<'js>, cid: u32, kind: &'static str, v: Value<'js>) -> Value<'js> {
        match v.as_number() {
            Some(n) if n > 0.0 => match self.rev.get(&(cid, kind, n as u32)).or_else(|| (kind == "Texture").then(|| self.rev.get(&(cid, "Renderbuffer", n as u32))).flatten()) {
                Some(c) => Value::new_number(ctx.clone(), *c as f64),
                None => v,
            },
            _ => v,
        }
    }
}

fn get_fn<'js>(ctx: &Ctx<'js>, fns: &mut [Option<Function<'js>>], f: u8) -> Option<Function<'js>> {
    let slot = fns.get_mut(f as usize)?;
    if slot.is_none() {
        *slot = ctx.globals().get::<_, Function>(FNS[f as usize]).ok();
    }
    slot.clone()
}

fn gl_op_name(op: i32) -> &'static str {
    #[cfg(feature = "webgl")]
    {
        if op >= 0 {
            return super::webgl::op_names().get(op as usize).copied().unwrap_or("");
        }
    }
    let _ = op;
    ""
}

/// create / delete 的对象种类（`createTexture` → `Texture`；fenceSync / deleteSync → `Sync`）
fn kind_of(op: &'static str) -> &'static str {
    match op {
        "fenceSync" => "Sync",
        _ => op.strip_prefix("create").or_else(|| op.strip_prefix("delete")).unwrap_or(op),
    }
}

/// 返回值是对象名字的查询（getParameter 的绑定类、getFramebufferAttachmentParameter(OBJECT_NAME)）与它的种类
fn returns_name(op: &str, args: &[Value]) -> Option<&'static str> {
    let p = |i: usize| args.get(i).and_then(|v| v.as_number()).unwrap_or(0.0) as u32;
    match op {
        "getParameter" => Some(match p(2) {
            0x8894 | 0x8895 | 0x8F36 | 0x8F37 | 0x88ED | 0x88EF | 0x8C8F | 0x8A28 => "Buffer",
            0x8B8D => "Program",
            0x8CA6 | 0x8CAA => "Framebuffer",
            0x8CA7 => "Renderbuffer",
            0x8069 | 0x8514 | 0x8C1D | 0x806A => "Texture",
            0x85B5 => "VertexArray",
            0x8919 => "Sampler",
            0x8E25 => "TransformFeedback",
            _ => return None,
        }),
        // 挂的可能是纹理也可能是渲染缓冲：先按纹理找，找不到再按渲染缓冲（untranslate）
        "getFramebufferAttachmentParameter" if p(4) == 0x8CD1 => Some("Texture"),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
