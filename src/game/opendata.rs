//! 开放数据域（game-doc/04 G9.7，11 B-6，D-5）：第二个 JS 环境 + sharedCanvas + 单向消息 + 关系链 API 交宿主。
//!
//! - 独立的 QuickJS 运行时（与主域不共享任何 JS 对象）；只装 module / 开放数据域环境 / 图片 / 2D 画布 / 开放数据域的 wx。
//!   没有网络、存储、文件（微信的开放数据域也没有）；
//! - 画布与图片的原生表与主域共用（同一个 GameCanvases / ImageStore）：sharedCanvas 就是表里的一块，主域 drawImage 它；
//! - 主域第一次 `wx.getOpenDataContext()` 之后的下一帧启动，执行 game.json `openDataContext` 目录里的 index.js；
//! - 每帧（主域逻辑层之后、rAF 之前）：先投递主域发来的消息，再推进它的定时器；主域是 rAF 帧时它的 rAF 也跑；
//! - 关系链 / 托管数据 API：宿主在 hostApis 里点名 `openData.<名字>` 才交宿主（宿主事件 hostCall，`mr_game_host_reply` 回），
//!   编号从 1<<24 起，与主域的 hostCall 编号分开。

use super::GameEngine;
use crate::js::JsRuntime;
use rquickjs::IntoJs;
use serde_json::json;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

const ID_BASE: u32 = 1 << 24;

#[derive(Default)]
pub struct Shared {
    /// 主域要过 getOpenDataContext（下一帧启动）
    pub wanted: bool,
    /// 主域发来、还没投递的消息（JSON）
    pub inbox: Vec<String>,
    /// 开放数据域的日志行、宿主事件
    pub logs: Vec<String>,
    pub host: Vec<serde_json::Value>,
    pub pending: HashSet<u32>,
    next: u32,
    /// 开放数据域还有定时器 / rAF 挂着（宿主要接着出帧）
    pub busy: bool,
}

pub struct OpenData {
    rt: JsRuntime,
    guards: crate::js::Guards,
}

/// 主域那几个原生函数
pub fn register_main(rt: &JsRuntime, sh: Rc<RefCell<Shared>>, configured: bool) -> Result<(), String> {
    let s = sh.clone();
    rt.register_raw("__game_od_open", move |ctx, _| {
        s.borrow_mut().wanted = true;
        configured.into_js(&ctx)
    })?;
    rt.register_raw("__game_od_post", move |ctx, a| {
        let m = a.0.first().and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_else(|| "{}".into());
        sh.borrow_mut().inbox.push(m);
        Ok(rquickjs::Value::new_undefined(ctx))
    })?;
    Ok(())
}

impl GameEngine {
    /// pump 第 4.5 步：要启动就启动，再投递消息、推进定时器 / rAF
    pub(super) fn opendata_tick(&mut self, now_ms: f64, raf: bool) {
        if self.od_rt.is_none() && self.od.borrow().wanted {
            self.opendata_start();
        }
        let Some(od) = self.od_rt.as_ref() else { return };
        let msgs = std::mem::take(&mut self.od.borrow_mut().inbox);
        od.guards.watchdog.arm();
        let mut errs = Vec::new();
        for m in msgs {
            if let Err(e) = od.rt.call_str2("__odMessage", &m, "") {
                errs.push(e);
            }
        }
        match od.rt.eval(&format!("__odTick({now_ms}, {raf})")) {
            Ok(n) => self.od.borrow_mut().busy = n.trim() != "0",
            Err(e) => errs.push(e),
        }
        od.rt.pump_jobs();
        od.guards.watchdog.disarm();
        for e in errs {
            self.od.borrow_mut().logs.push(format!("[开放数据域][error] {e}"));
        }
        self.opendata_flush();
    }

    /// 开放数据域的日志、宿主事件交给宿主（日志行带「[开放数据域]」）
    pub(super) fn opendata_flush(&mut self) {
        let (logs, host) = {
            let mut s = self.od.borrow_mut();
            (std::mem::take(&mut s.logs), std::mem::take(&mut s.host))
        };
        for l in logs {
            self.host_log(&l);
        }
        for h in host {
            self.events.push_back(h.to_string());
        }
    }

    /// 宿主回关系链 API 的结果（mr_game_host_reply 的编号在开放数据域那一段时）
    pub(super) fn opendata_reply(&mut self, id: u32, ok: bool, json: &str) -> Option<bool> {
        if !self.od.borrow_mut().pending.remove(&id) {
            return None;
        }
        let od = self.od_rt.as_ref()?;
        let r = od.rt.eval(&format!("__odReply({id}, {ok}, {})", serde_json::Value::String(json.to_string())));
        od.rt.pump_jobs();
        if let Err(e) = r {
            self.od.borrow_mut().logs.push(format!("[开放数据域][error] {e}"));
        }
        self.opendata_flush();
        Some(true)
    }

    /// 图片解码完：开放数据域里建的那些也要回调（两个运行时都问一遍，各自只认自己的编号）
    pub(super) fn opendata_image_done(&self, id: u32, json: &str) {
        if let Some(od) = self.od_rt.as_ref() {
            let _ = od.rt.call_str2("__gameImageDone", &id.to_string(), json);
        }
    }

    fn opendata_start(&mut self) {
        let Some(dir) = self.config.open_data_context.clone() else {
            self.od.borrow_mut().wanted = false;
            return;
        };
        match self.opendata_build(&dir) {
            Ok(od) => {
                self.host_log(&format!("🔒 开放数据域启动：{dir}/index.js"));
                self.od_rt = Some(od);
            }
            Err(e) => {
                self.od.borrow_mut().wanted = false;
                self.report_error("opendata", &format!("开放数据域启动失败：{e}"));
            }
        }
    }

    fn opendata_build(&mut self, dir: &str) -> Result<OpenData, String> {
        let rt = JsRuntime::new()?;
        let guards = rt.install_guards(self.guards.watchdog.budget_ms(), 256 * 1024 * 1024);
        super::canvas::register(&rt, self.canvases.clone(), self.images.clone())?;
        super::image::register(&rt, self.images.clone())?;
        let sh = self.od.clone();
        rt.register_raw("__od_log", move |ctx, a| {
            let s = |i: usize| a.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
            sh.borrow_mut().logs.push(format!("[开放数据域]{} {}", if s(0) == "log" { String::new() } else { format!("[{}]", s(0)) }, s(1)));
            Ok(rquickjs::Value::new_undefined(ctx))
        })?;
        let c = self.canvases.clone();
        rt.register_raw("__od_shared", move |ctx, _| {
            let (id, w, h) = c.borrow_mut().shared();
            vec![id as f64, w as f64, h as f64].into_js(&ctx)
        })?;
        let info = self.app.eval("JSON.stringify(wx.getSystemInfoSync())").unwrap_or_else(|_| "{}".into());
        rt.register_raw("__od_sysinfo", move |ctx, _| info.clone().into_js(&ctx))?;
        // 关系链 API：宿主点名了才交宿主（-1 = 没有宿主，JS 那边 fail）
        let sh = self.od.clone();
        let apis: HashSet<String> = self.host_apis.iter().filter_map(|a| a.strip_prefix("openData.").map(str::to_string)).collect();
        rt.register_raw("__od_host", move |ctx, a| {
            let s = |i: usize| a.0.get(i).and_then(|v| v.as_string()).and_then(|s| s.to_string().ok()).unwrap_or_default();
            let api = s(0);
            if !apis.contains(&api) {
                return (-1).into_js(&ctx);
            }
            let mut w = sh.borrow_mut();
            w.next += 1;
            let id = ID_BASE + w.next;
            w.pending.insert(id);
            let args: serde_json::Value = serde_json::from_str(&s(1)).unwrap_or_default();
            w.host.push(json!({ "type": "hostCall", "api": format!("openData.{api}"), "id": id, "args": args }));
            id.into_js(&ctx)
        })?;
        for name in ["game-opendata-env", "game-image", "game-canvas", "game-opendata-wx"] {
            rt.eval(crate::js::game_prelude_source(name))?;
        }
        rt.eval(crate::js::prelude_source("module").ok_or("缺 module 前置代码")?)?;
        let root = self.root.clone();
        let mut files = Vec::new();
        super::worker::collect_js(&root, &root.join(dir), &mut files);
        files.sort();
        for (key, path) in files {
            let src = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            rt.eval(&format!("__defineModule({key:?}, function(module, exports, require) {{{src}\n}});"))?;
        }
        let entry = format!("{}/index.js", dir.trim_end_matches('/'));
        guards.watchdog.arm();
        let r = rt.eval(&format!("__require('', {entry:?})"));
        guards.watchdog.disarm();
        rt.pump_jobs();
        if let Err(e) = r {
            self.od.borrow_mut().logs.push(format!("[开放数据域][error] {e}"));
        }
        Ok(OpenData { rt, guards })
    }
}
