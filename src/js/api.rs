//! 小程序 API 实现

use super::JsRuntime;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// 小程序 API
pub struct MiniAppApi {
    runtime: Arc<Mutex<JsRuntime>>,
    storage: Arc<Mutex<HashMap<String, String>>>,
}

impl MiniAppApi {
    pub fn new(runtime: Arc<Mutex<JsRuntime>>) -> Self {
        Self {
            runtime,
            storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// 初始化所有 API
    pub fn init(&self) -> Result<(), String> {
        println!("    init_module_system...");
        self.init_module_system().map_err(|e| format!("module: {}", e))?;
        println!("    init_console...");
        self.init_console().map_err(|e| format!("console: {}", e))?;
        println!("    init_wx_object...");
        self.init_wx_object().map_err(|e| format!("wx: {}", e))?;
        println!("    init_timer...");
        self.init_timer_api().map_err(|e| format!("timer: {}", e))?;
        println!("    init_storage...");
        self.init_storage_api().map_err(|e| format!("storage: {}", e))?;
        println!("    init_ui...");
        self.init_ui_api().map_err(|e| format!("ui: {}", e))?;
        println!("    init_canvas...");
        self.init_canvas_api().map_err(|e| format!("canvas: {}", e))?;
        println!("    init_app...");
        self.init_app().map_err(|e| format!("app: {}", e))?;
        println!("    init_component...");
        self.init_component().map_err(|e| format!("component: {}", e))?;
        self.init_missing_api_probe().map_err(|e| format!("probe: {}", e))?;
        Ok(())
    }

    /// 给 `wx` 装一层探针：读到**尚未实现**的 API 时打一条警告（每个名字只打一次）。
    ///
    /// 只在所有 API 注册完成之后装，所以已实现的照常返回。
    /// 关键是**仍然返回 `undefined`** —— 框架普遍用 `typeof wx.xxx === 'function'`
    /// 做能力探测，返回一个假函数会让它们走上不存在的分支。
    ///
    /// 动机：QuickJS 对着 undefined 调用只会抛 `TypeError: not a function`，
    /// 既没有名字也没有栈。跑第三方编译产物（uni-app / Taro）时，
    /// 这一行警告能直接指出缺哪个 API，省掉通读几十万行 vendor 包。
    fn init_missing_api_probe(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            if (typeof Proxy === 'function' && typeof wx === 'object' && !wx.__probed) {
                var __wxImpl = wx;
                var __warned = {};
                __wxImpl.__probed = true;
                wx = new Proxy(__wxImpl, {
                    get: function (target, key) {
                        if (!(key in target) && typeof key === 'string' && key.indexOf('__') !== 0) {
                            if (!__warned[key]) {
                                __warned[key] = true;
                                if (typeof console !== 'undefined' && console.warn) {
                                    console.warn('[未实现的 API] wx.' + key);
                                }
                            }
                            return undefined;
                        }
                        return target[key];
                    }
                });
                // uni-app / Taro 会把 uni 指向 wx，保持同一层探针
                if (typeof uni !== 'undefined' && uni === __wxImpl) { uni = wx; }
            }
        "#)?;
        Ok(())
    }
    
    fn init_wx_object(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval("var wx = wx || {};")?;
        Ok(())
    }
    
    /// 初始化 CommonJS 模块系统（require / module.exports）
    ///
    /// 官方小程序逻辑层以 CommonJS 组织多文件依赖。之前的实现直接把单个
    /// 文件整段 eval，无法处理 `require('../utils/util.js')` 这类依赖。
    fn init_module_system(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            // 打包器/框架运行时（uni-app 的 vendor、Vue、各类 polyfill）普遍会摸
            // `global` / `self` / `window` 之类的全局别名来做环境探测。
            // 小程序环境里 `globalThis` 是有的，把常见别名指过去即可 ——
            // 缺了 `global` 时 Vue 运行时会在第一次特征检测就 ReferenceError。
            if (typeof globalThis !== 'undefined') {
                if (typeof global === 'undefined') { globalThis.global = globalThis; }
                if (typeof self === 'undefined') { globalThis.self = globalThis; }
            }

            var __modules = {};       // key -> factory(module, exports, require)
            var __moduleCache = {};   // key -> module 实例
            
            function __normalizePath(path) {
                var parts = String(path).split('/');
                var stack = [];
                for (var i = 0; i < parts.length; i++) {
                    var p = parts[i];
                    if (p === '' || p === '.') continue;
                    if (p === '..') { stack.pop(); continue; }
                    stack.push(p);
                }
                return stack.join('/');
            }
            
            function __stripJs(key) {
                if (key.length > 3 && key.substring(key.length - 3) === '.js') {
                    return key.substring(0, key.length - 3);
                }
                return key;
            }
            
            function __dirname(path) {
                var idx = path.lastIndexOf('/');
                return idx >= 0 ? path.substring(0, idx) : '';
            }
            
            function __resolveModulePath(currentDir, request) {
                var full;
                if (request.charAt(0) === '/') {
                    full = request.substring(1);
                } else {
                    full = (currentDir ? currentDir + '/' : '') + request;
                }
                return __stripJs(__normalizePath(full));
            }
            
            function __defineModule(path, factory) {
                __modules[__stripJs(__normalizePath(path))] = factory;
            }
            
            function __require(currentDir, request) {
                var key = __resolveModulePath(currentDir, request);
                if (__moduleCache[key]) { return __moduleCache[key].exports; }
                var factory = __modules[key];
                if (!factory) {
                    throw new Error('Cannot find module: ' + request + ' (resolved: ' + key + ')');
                }
                var module = { exports: {} };
                __moduleCache[key] = module;
                var dir = __dirname(key);
                var localRequire = function(req) { return __require(dir, req); };
                factory.call(module.exports, module, module.exports, localRequire);
                return module.exports;
            }
            
            // 全局 require（相对项目根目录解析）
            function require(request) { return __require('', request); }

            // 以「模块作用域」立即执行一段源码（app.js / 页面 js 走这条）。
            //
            // 小程序里每个 .js 文件都是 CommonJS 模块，`module` / `exports` / `require`
            // 天然在作用域里。从前 app.js 是当**裸脚本**求值的，于是 TS/uni-app 这类
            // 编译产物一上来就 `Object.defineProperty(exports, ...)`，直接
            // ReferenceError: 'exports' is not defined。
            //
            // 与 __require 的区别是**不缓存**：页面 js 每次进入都要重新执行，
            // 这样 `Page({...})` 才会重新注册（返回上一页再进来不能是空白页）。
            function __runAsModule(path, factory) {
                var key = __stripJs(__normalizePath(path));
                var module = { exports: {} };
                var dir = __dirname(key);
                var localRequire = function(req) { return __require(dir, req); };
                factory.call(module.exports, module, module.exports, localRequire);
                return module.exports;
            }
        "#)?;
        Ok(())
    }
    
    fn init_console(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __console_buffer = [];
            // Error 参数要连栈一起打。框架（Vue 等）的错误处理器普遍是
            // `console.error(err)`，而 `String(err)` 只有 "TypeError: not a function" ——
            // 没有名字也没有位置，等于逼人去通读几十万行 vendor 包。
            function __fmtArg(a) {
                if (a instanceof Error) {
                    var s = (a.name || 'Error') + ': ' + (a.message || '');
                    if (a.stack) { s += '\n' + a.stack; }
                    return s;
                }
                if (a && typeof a === 'object') {
                    try { return JSON.stringify(a); } catch (e) { return String(a); }
                }
                return String(a);
            }
            function __fmtArgs(args) {
                return Array.prototype.map.call(args, __fmtArg).join(' ');
            }
            var console = {
                log: function() {
                    var msg = __fmtArgs(arguments);
                    __console_buffer.push('[LOG] ' + msg);
                    if (typeof __native_print === 'function') { __native_print(msg); }
                },
                error: function() {
                    var msg = __fmtArgs(arguments);
                    __console_buffer.push('[ERROR] ' + msg);
                    if (typeof __native_print === 'function') { __native_print('[ERROR] ' + msg); }
                },
                warn: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[WARN] ' + msg);
                    if (typeof __native_print === 'function') { __native_print('[WARN] ' + msg); }
                },
                info: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[INFO] ' + msg);
                    if (typeof __native_print === 'function') { __native_print(msg); }
                }
            };
        "#)?;
        Ok(())
    }
    
    fn init_timer_api(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __timers = {};
            var __timer_id = 0;
            
            function setTimeout(callback, delay) {
                delay = delay || 0;
                var id = ++__timer_id;
                __timers[id] = { callback: callback, delay: delay, type: 'timeout' };
                if (typeof __native_set_timer === 'function') {
                    __native_set_timer(String(id), String(delay), 'false');
                }
                return id;
            }
            
            function setInterval(callback, delay) {
                delay = delay || 0;
                var id = ++__timer_id;
                __timers[id] = { callback: callback, delay: delay, type: 'interval' };
                if (typeof __native_set_timer === 'function') {
                    __native_set_timer(String(id), String(delay), 'true');
                }
                return id;
            }
            
            function clearTimeout(id) {
                if (__timers[id]) {
                    delete __timers[id];
                    if (typeof __native_clear_timer === 'function') {
                        __native_clear_timer(String(id));
                    }
                }
            }
            
            function clearInterval(id) { clearTimeout(id); }
            
            function __trigger_timer(id) {
                var timer = __timers[id];
                if (timer && typeof timer.callback === 'function') {
                    try { timer.callback(); } catch (e) { console.error('Timer error:', e); }
                    if (timer.type === 'timeout') { delete __timers[id]; }
                }
            }
        "#)?;
        Ok(())
    }
    
    fn init_storage_api(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __storage = {};
            
            wx.setStorageSync = function(key, data) {
                var value = typeof data === 'string' ? data : JSON.stringify(data);
                __storage[key] = value;
                if (typeof __native_storage_set === 'function') { __native_storage_set(key, value); }
            };
            
            wx.getStorageSync = function(key) {
                var value = __storage[key];
                if (value === undefined && typeof __native_storage_get === 'function') {
                    value = __native_storage_get(key);
                    if (value) { __storage[key] = value; }
                }
                if (value === undefined || value === '') { return ''; }
                try { return JSON.parse(value); } catch(e) { return value; }
            };
            
            wx.removeStorageSync = function(key) {
                delete __storage[key];
                if (typeof __native_storage_remove === 'function') { __native_storage_remove(key); }
            };
            
            wx.clearStorageSync = function() {
                __storage = {};
                if (typeof __native_storage_clear === 'function') { __native_storage_clear(); }
            };
            
            wx.getStorageInfoSync = function() {
                var keys = Object.keys(__storage);
                var currentSize = 0;
                for (var i = 0; i < keys.length; i++) { currentSize += (__storage[keys[i]] || '').length; }
                return { keys: keys, currentSize: Math.ceil(currentSize / 1024), limitSize: 10240 };
            };
            
            wx.setStorage = function(options) {
                options = options || {};
                try { wx.setStorageSync(options.key, options.data); options.success && options.success(); }
                catch(e) { options.fail && options.fail({ errMsg: 'setStorage:fail' }); }
                options.complete && options.complete();
            };
            
            wx.getStorage = function(options) {
                options = options || {};
                try { var data = wx.getStorageSync(options.key); options.success && options.success({ data: data }); }
                catch(e) { options.fail && options.fail({ errMsg: 'getStorage:fail' }); }
                options.complete && options.complete();
            };
            
            wx.removeStorage = function(options) {
                options = options || {};
                try { wx.removeStorageSync(options.key); options.success && options.success(); }
                catch(e) { options.fail && options.fail({ errMsg: 'removeStorage:fail' }); }
                options.complete && options.complete();
            };
            
            wx.clearStorage = function(options) {
                options = options || {};
                try { wx.clearStorageSync(); options.success && options.success(); }
                catch(e) { options.fail && options.fail({ errMsg: 'clearStorage:fail' }); }
                options.complete && options.complete();
            };
        "#)?;
        Ok(())
    }
    
    fn init_ui_api(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __toastTimer = null;
            var __toastVisible = false;
            var __toastConfig = null;
            var __loadingVisible = false;
            var __loadingConfig = null;
            var __modalVisible = false;
            var __modalConfig = null;
            var __modalCallback = null;
            
            wx.showToast = function(options) {
                options = options || {};
                var title = options.title || '';
                var icon = options.icon || 'success';
                var duration = options.duration || 1500;
                var mask = options.mask || false;
                
                if (__toastTimer) { clearTimeout(__toastTimer); __toastTimer = null; }
                __toastVisible = true;
                __toastConfig = { title: title, icon: icon, mask: mask };
                
                if (typeof __native_show_toast === 'function') {
                    __native_show_toast(title, icon, String(duration), mask ? 'true' : 'false');
                }
                __native_print('[Toast] ' + title + ' (' + icon + ')');
                
                __toastTimer = setTimeout(function() { wx.hideToast(); }, duration);
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.hideToast = function(options) {
                options = options || {};
                if (__toastTimer) { clearTimeout(__toastTimer); __toastTimer = null; }
                __toastVisible = false;
                __toastConfig = null;
                if (typeof __native_hide_toast === 'function') { __native_hide_toast(); }
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.showLoading = function(options) {
                options = options || {};
                __loadingVisible = true;
                __loadingConfig = { title: options.title || '', mask: options.mask || false };
                if (typeof __native_show_loading === 'function') {
                    __native_show_loading(options.title || '', options.mask ? 'true' : 'false');
                }
                __native_print('[Loading] ' + (options.title || ''));
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.hideLoading = function(options) {
                options = options || {};
                __loadingVisible = false;
                __loadingConfig = null;
                if (typeof __native_hide_loading === 'function') { __native_hide_loading(); }
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.showModal = function(options) {
                options = options || {};
                __modalVisible = true;
                __modalConfig = {
                    title: options.title || '',
                    content: options.content || '',
                    showCancel: options.showCancel !== false,
                    cancelText: options.cancelText || '取消',
                    confirmText: options.confirmText || '确定'
                };
                __modalCallback = options;
                
                if (typeof __native_show_modal === 'function') {
                    __native_show_modal(__modalConfig.title, __modalConfig.content, 
                        __modalConfig.showCancel ? 'true' : 'false', __modalConfig.cancelText, __modalConfig.confirmText);
                }
                __native_print('[Modal] ' + __modalConfig.title + ': ' + __modalConfig.content);
            };
            
            function __handleModalResult(confirm) {
                if (__modalCallback) {
                    var result = { confirm: confirm, cancel: !confirm };
                    __modalCallback.success && __modalCallback.success(result);
                    __modalCallback.complete && __modalCallback.complete(result);
                }
                __modalVisible = false;
                __modalConfig = null;
                __modalCallback = null;
            }
            
            wx.showActionSheet = function(options) {
                options = options || {};
                __native_print('[ActionSheet] ' + (options.itemList || []).join(', '));
                options.success && options.success({ tapIndex: 0 });
                options.complete && options.complete();
            };
            
            // ── wx.createAnimation：小程序里「用 JS 操作样式」的正式做法 ──
            // 链式调用累积一步的目标值，step() 定格一步，export() 交给
            // `animation="{{animData}}"` 属性，渲染层按 actions 顺序插值。
            wx.createAnimation = function(option) {
                option = option || {};
                var defaults = {
                    duration: option.duration === undefined ? 400 : option.duration,
                    delay: option.delay === undefined ? 0 : option.delay,
                    timingFunction: option.timingFunction || 'linear',
                    transformOrigin: option.transformOrigin || '50% 50% 0'
                };
                var actions = [];
                var pending = { animates: [], option: null };
                function record(type, args) {
                    pending.animates.push({ type: type, args: args });
                    return api;
                }
                var api = {
                    // 变换（H5 端 runtime.js 有一份同语义实现）
                    translate: function(x, y) { return record('translate', [x || 0, y || 0]); },
                    translateX: function(v) { return record('translateX', [v || 0]); },
                    translateY: function(v) { return record('translateY', [v || 0]); },
                    rotate: function(deg) { return record('rotate', [deg || 0]); },
                    rotateZ: function(deg) { return record('rotate', [deg || 0]); },
                    scale: function(sx, sy) { return record('scale', [sx === undefined ? 1 : sx, sy === undefined ? sx : sy]); },
                    scaleX: function(v) { return record('scaleX', [v === undefined ? 1 : v]); },
                    scaleY: function(v) { return record('scaleY', [v === undefined ? 1 : v]); },
                    skew: function(x, y) { return record('skew', [x || 0, y || 0]); },
                    // 样式
                    opacity: function(v) { return record('opacity', [v]); },
                    backgroundColor: function(c) { return record('backgroundColor', [c]); },
                    width: function(v) { return record('width', [v]); },
                    height: function(v) { return record('height', [v]); },
                    // 定格一步
                    step: function(cfg) {
                        cfg = cfg || {};
                        actions.push({
                            animates: pending.animates,
                            option: {
                                transition: {
                                    duration: cfg.duration === undefined ? defaults.duration : cfg.duration,
                                    delay: cfg.delay === undefined ? defaults.delay : cfg.delay,
                                    timingFunction: cfg.timingFunction || defaults.timingFunction
                                },
                                transformOrigin: cfg.transformOrigin || defaults.transformOrigin
                            }
                        });
                        pending = { animates: [], option: null };
                        return api;
                    },
                    export: function() {
                        var out = { actions: actions };
                        actions = [];
                        return out;
                    }
                };
                return api;
            };

            // 下拉刷新：指示器与内容位移都由宿主负责，逻辑层只发「开始/结束」
            wx.startPullDownRefresh = function(options) {
                options = options || {};
                if (typeof __native_start_pull_down_refresh === 'function') {
                    __native_start_pull_down_refresh();
                }
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.stopPullDownRefresh = function(options) {
                options = options || {};
                if (typeof __native_stop_pull_down_refresh === 'function') {
                    __native_stop_pull_down_refresh();
                }
                options.success && options.success();
                options.complete && options.complete();
            };
            
            function __getUIState() {
                return JSON.stringify({
                    toast: __toastVisible ? __toastConfig : null,
                    loading: __loadingVisible ? __loadingConfig : null,
                    modal: __modalVisible ? __modalConfig : null
                });
            }
        "#)?;
        Ok(())
    }
    
    fn init_canvas_api(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __canvasContexts = {};
            
            wx.createCanvasContext = function(canvasId, component) {
                var ctx = {
                    _canvasId: canvasId,
                    _commands: [],
                    setFillStyle: function(color) { this._commands.push({ type: 'setFillStyle', color: color }); return this; },
                    setStrokeStyle: function(color) { this._commands.push({ type: 'setStrokeStyle', color: color }); return this; },
                    setLineWidth: function(width) { this._commands.push({ type: 'setLineWidth', width: width }); return this; },
                    setLineCap: function(cap) { this._commands.push({ type: 'setLineCap', cap: cap }); return this; },
                    setLineJoin: function(join) { this._commands.push({ type: 'setLineJoin', join: join }); return this; },
                    setFontSize: function(size) { this._commands.push({ type: 'setFontSize', size: size }); return this; },
                    setTextAlign: function(align) { this._commands.push({ type: 'setTextAlign', align: align }); return this; },
                    setTextBaseline: function(baseline) { this._commands.push({ type: 'setTextBaseline', baseline: baseline }); return this; },
                    setGlobalAlpha: function(alpha) { this._commands.push({ type: 'setGlobalAlpha', alpha: alpha }); return this; },
                    fillRect: function(x, y, w, h) { this._commands.push({ type: 'fillRect', x: x, y: y, width: w, height: h }); return this; },
                    strokeRect: function(x, y, w, h) { this._commands.push({ type: 'strokeRect', x: x, y: y, width: w, height: h }); return this; },
                    clearRect: function(x, y, w, h) { this._commands.push({ type: 'clearRect', x: x, y: y, width: w, height: h }); return this; },
                    beginPath: function() { this._commands.push({ type: 'beginPath' }); return this; },
                    closePath: function() { this._commands.push({ type: 'closePath' }); return this; },
                    moveTo: function(x, y) { this._commands.push({ type: 'moveTo', x: x, y: y }); return this; },
                    lineTo: function(x, y) { this._commands.push({ type: 'lineTo', x: x, y: y }); return this; },
                    arc: function(x, y, r, s, e, cc) { this._commands.push({ type: 'arc', x: x, y: y, r: r, sAngle: s, eAngle: e, counterclockwise: cc || false }); return this; },
                    quadraticCurveTo: function(cpx, cpy, x, y) { this._commands.push({ type: 'quadraticCurveTo', cpx: cpx, cpy: cpy, x: x, y: y }); return this; },
                    bezierCurveTo: function(cp1x, cp1y, cp2x, cp2y, x, y) { this._commands.push({ type: 'bezierCurveTo', cp1x: cp1x, cp1y: cp1y, cp2x: cp2x, cp2y: cp2y, x: x, y: y }); return this; },
                    fill: function() { this._commands.push({ type: 'fill' }); return this; },
                    stroke: function() { this._commands.push({ type: 'stroke' }); return this; },
                    fillText: function(text, x, y, maxWidth) { this._commands.push({ type: 'fillText', text: text, x: x, y: y, maxWidth: maxWidth }); return this; },
                    strokeText: function(text, x, y, maxWidth) { this._commands.push({ type: 'strokeText', text: text, x: x, y: y, maxWidth: maxWidth }); return this; },
                    drawImage: function(src, sx, sy, sw, sh, dx, dy, dw, dh) {
                        if (arguments.length === 3) { this._commands.push({ type: 'drawImage', src: src, dx: sx, dy: sy }); }
                        else if (arguments.length === 5) { this._commands.push({ type: 'drawImage', src: src, dx: sx, dy: sy, dWidth: sw, dHeight: sh }); }
                        else { this._commands.push({ type: 'drawImage', src: src, sx: sx, sy: sy, sWidth: sw, sHeight: sh, dx: dx, dy: dy, dWidth: dw, dHeight: dh }); }
                        return this;
                    },
                    save: function() { this._commands.push({ type: 'save' }); return this; },
                    restore: function() { this._commands.push({ type: 'restore' }); return this; },
                    translate: function(x, y) { this._commands.push({ type: 'translate', x: x, y: y }); return this; },
                    rotate: function(angle) { this._commands.push({ type: 'rotate', angle: angle }); return this; },
                    scale: function(sx, sy) { this._commands.push({ type: 'scale', scaleX: sx, scaleY: sy }); return this; },
                    draw: function(reserve, callback) {
                        if (typeof __native_canvas_draw === 'function') {
                            __native_canvas_draw(this._canvasId, JSON.stringify(this._commands));
                        }
                        if (!reserve) { this._commands = []; }
                        if (typeof callback === 'function') { setTimeout(callback, 0); }
                    }
                };
                __canvasContexts[canvasId] = ctx;
                return ctx;
            };
        "#)?;
        Ok(())
    }
    
    /// 初始化 App API
    fn init_app(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        
        rt.eval(r#"
            var __app = null;
            var __pages = {};
            var __pageStack = [];      // 真实页面栈
            var __currentPage = null;  // 栈顶页面（保持与 native 的兼容契约）
            var __pendingNavigation = null;
            
            function App(config) {
                __app = config;
                __app.globalData = __app.globalData || {};
                if (config.onLaunch) {
                    try { config.onLaunch(); } catch (e) { __native_print('[App.onLaunch] ' + e.message); }
                }
                return __app;
            }
            
            function getApp() {
                return __app;
            }
            
            // ---- setData 数据路径工具（支持 'a.b.c'、'list[0].x'） ----
            function __parseDataPath(path) {
                var tokens = [];
                var buf = '';
                for (var i = 0; i < path.length; i++) {
                    var ch = path[i];
                    if (ch === '.') {
                        if (buf !== '') { tokens.push(buf); buf = ''; }
                    } else if (ch === '[') {
                        if (buf !== '') { tokens.push(buf); buf = ''; }
                    } else if (ch === ']') {
                        if (buf !== '') { tokens.push(parseInt(buf, 10)); buf = ''; }
                    } else {
                        buf += ch;
                    }
                }
                if (buf !== '') { tokens.push(buf); }
                return tokens;
            }
            
            function __setByPath(obj, path, value) {
                var tokens = __parseDataPath(path);
                if (tokens.length === 0) { return; }
                var cur = obj;
                for (var i = 0; i < tokens.length - 1; i++) {
                    var key = tokens[i];
                    var next = tokens[i + 1];
                    if (cur[key] === undefined || cur[key] === null) {
                        cur[key] = (typeof next === 'number') ? [] : {};
                    }
                    cur = cur[key];
                }
                cur[tokens[tokens.length - 1]] = value;
            }
            
            // 通用 setData 实现（页面与组件共用）
            function __applySetData(instance, newData, callback) {
                if (newData) {
                    for (var key in newData) {
                        if (!newData.hasOwnProperty(key)) { continue; }
                        if (key.indexOf('.') >= 0 || key.indexOf('[') >= 0) {
                            __setByPath(instance.data, key, newData[key]);
                        } else {
                            instance.data[key] = newData[key];
                        }
                    }
                }
                if (typeof __native_page_update === 'function') {
                    __native_page_update();
                }
                if (typeof callback === 'function') {
                    try { callback(); } catch (e) { __native_print('[setData.cb] ' + e.message); }
                }
            }
            
            function Page(config) {
                // 创建页面实例
                var page = {
                    data: config.data || {},
                    __isPage: true,
                    route: __pendingRoute || '',
                    
                    // setData 方法 - 更新数据并触发重新渲染（支持数据路径）
                    setData: function(newData, callback) {
                        __applySetData(this, newData, callback);
                    },
                    
                    // 官方 API：selectComponent 等（占位，待渲染层接入组件树后完善）
                    selectComponent: function() { return null; },
                    selectAllComponents: function() { return []; }
                };
                
                // 复制所有方法/字段到页面实例
                for (var key in config) {
                    if (config.hasOwnProperty(key) && key !== 'data') {
                        if (typeof config[key] === 'function') {
                            page[key] = config[key].bind(page);
                        } else {
                            page[key] = config[key];
                        }
                    }
                }
                
                // 入栈并置为当前页面
                __pageStack.push(page);
                __currentPage = page;
                __pendingPageCreated = true;
                
                return page;
            }
            
            // uni-app / Taro 这类框架不直接调 `Page()`，而是走 `wx.createPage(定义)`
            // （对应 `wx.createComponent` / `wx.createApp`）。它们语义上就是官方那三个
            // 全局注册函数的别名，桥过去即可 —— 缺了它页面 js 跑完也没有任何页面被注册，
            // `__currentPage` 一直是 null，于是 data 全空：模板里的 `{{a}}` 渲染成空串、
            // `bindtap="{{c}}"` 拿不到处理函数名，页面只剩一张静态骨架。
            if (typeof wx === 'object') {
                if (typeof wx.createPage !== 'function') {
                    wx.createPage = function(config) { return Page(config || {}); };
                }
                if (typeof wx.createComponent !== 'function') {
                    wx.createComponent = function(config) { return Component(config || {}); };
                }
                if (typeof wx.createApp !== 'function') {
                    wx.createApp = function(config) { return App(config || {}); };
                }
            }

            // 清空页面栈（switchTab 语义：切换 tab 会销毁原有页面栈）
            function __resetPageStack() {
                __pageStack = [];
                __currentPage = null;
                return true;
            }
            // 页面出栈（供 navigateBack 使用）
            function __popPage(delta) {
                delta = delta || 1;
                for (var i = 0; i < delta && __pageStack.length > 1; i++) {
                    __pageStack.pop();
                }
                __currentPage = __pageStack[__pageStack.length - 1] || null;
                return __currentPage;
            }
            
            // native 在加载页面 JS 前可设置将要创建页面的路由
            var __pendingRoute = '';
            // 本次页面 js 是否已经建出页面实例。用来区分「用 Component 定义页面」
            // 和「页面 js 里顺手注册了一个自定义组件」——只有前者要建页面。
            var __pendingPageCreated = false;
            function __setPendingRoute(route) {
                __pendingRoute = route || '';
                __pendingPageCreated = false;
            }
            
            function getCurrentPages() {
                return __pageStack.slice();
            }
            
            // 获取当前页面实例（供 native 调用）
            function __getPageInstance() {
                return __currentPage;
            }
            
            // ==================== 完整生命周期分发 ====================
            // App 级生命周期：onLaunch/onShow/onHide/onError/onPageNotFound/
            //                 onUnhandledRejection/onThemeChange
            function __dispatchApp(name, arg) {
                if (__app && typeof __app[name] === 'function') {
                    try { return __app[name](arg); }
                    catch (e) { __native_print('[App.' + name + '] ' + e.message); }
                }
            }
            
            // Page 级生命周期：onLoad/onShow/onReady/onHide/onUnload/
            //   onPullDownRefresh/onReachBottom/onPageScroll/onResize/
            //   onTabItemTap/onShareAppMessage/onShareTimeline/onAddToFavorites
            // 同时联动页面内组件的 pageLifetimes(show/hide/resize)。
            function __dispatchPage(name, arg) {
                var result;
                if (__currentPage && typeof __currentPage[name] === 'function') {
                    try { result = __currentPage[name](arg); }
                    catch (e) { __native_print('[Page.' + name + '] ' + e.message); }
                }
                var mapped = { onShow: 'show', onHide: 'hide', onResize: 'resize' };
                if (mapped[name] && typeof __dispatchAllComponentsPageLifetime === 'function') {
                    __dispatchAllComponentsPageLifetime(mapped[name], arg);
                }
                // 页面卸载：销毁其组件实例，触发 detached
                if (name === 'onUnload' && typeof __detachAllComponents === 'function') {
                    __detachAllComponents();
                }
                return result;
            }
            
            // 调用页面方法（供 native 调用事件处理）。
            // 处理函数可能挂在页面上，也可能挂在页面内的自定义组件实例上
            // （组件模板里的 bindtap 绑的是组件自己的方法）。
            function __callPageMethod(methodName, eventData) {
                if (__hasHandler(methodName)) {
                    // 确保 eventData 是对象
                    if (typeof eventData === 'string') {
                        try {
                            eventData = JSON.parse(eventData);
                        } catch (e) {
                            eventData = {};
                        }
                    }
                    eventData = eventData || {};
                    
                    // 转换 dataset 中的值
                    var dataset = {};
                    for (var key in eventData) {
                        if (eventData.hasOwnProperty(key)) {
                            var val = eventData[key];
                            if (typeof val === 'string') {
                                // 尝试解析 JSON 对象/数组（单引号格式）
                                // 模板引擎将双引号替换为单引号以避免 HTML 属性冲突
                                if ((val.startsWith('{') && val.endsWith('}')) || 
                                    (val.startsWith('[') && val.endsWith(']'))) {
                                    try {
                                        // 将单引号替换回双引号后解析
                                        var jsonStr = val.replace(/'/g, '"');
                                        dataset[key] = JSON.parse(jsonStr);
                                    } catch (e) {
                                        dataset[key] = val;
                                    }
                                }
                                // 尝试转换为数字
                                else if (/^-?\d+(\.\d+)?$/.test(val)) {
                                    dataset[key] = parseFloat(val);
                                } else {
                                    dataset[key] = val;
                                }
                            } else {
                                dataset[key] = val;
                            }
                        }
                    }
                    
                    var targetInfo = { id: '', offsetLeft: 0, offsetTop: 0, dataset: dataset };
                    var event = {
                        type: 'tap',
                        timeStamp: Date.now(),
                        target: targetInfo,
                        currentTarget: targetInfo,
                        detail: dataset,
                        touches: [],
                        changedTouches: []
                    };
                    try {
                        __dispatchHandler(methodName, event);
                    } catch (e) {
                        __native_print('[Error] ' + methodName + ': ' + (e && e.message ? e.message : e));
                    }
                    return true;
                }
                return false;
            }

            /// 页面或其组件实例上是否存在该处理函数
            function __hasHandler(name) {
                if (!name) { return false; }
                if (__currentPage && typeof __currentPage[name] === 'function') { return true; }
                for (var tag in __pageComponents) {
                    if (!__pageComponents.hasOwnProperty(tag)) { continue; }
                    var inst = __componentInstances[__pageComponents[tag]];
                    if (inst && typeof inst[name] === 'function') { return true; }
                }
                return false;
            }
            
            // 获取页面数据（供 native 调用）
            function __getPageData() {
                if (__currentPage) {
                    return JSON.stringify(__currentPage.data);
                }
                return '{}';
            }

            // ==================== 页面内自定义组件（usingComponents） ====================
            // 标签名 -> 组件实例 id。渲染层按标签名取组件自己的 data 去渲染组件模板：
            // 组件的 `{{a}}` 与页面的 `{{a}}` 是两套数据，不能共用一个作用域。
            var __pageComponents = {};

            function __resetPageComponents() { __pageComponents = {}; }

            /// 在页面数据作用域里求一个表达式（用于 `u-p="{{r||''}}"` 这类「父传子」属性）
            function __evalInPageData(exprSrc) {
                try {
                    var d = (__currentPage && __currentPage.data) || {};
                    var f = new Function('d', 'with (d) { return (' + exprSrc + '); }');
                    return f(d);
                } catch (e) { return null; }
            }

            /// 挂载一个页面内自定义组件：建实例（会跑 created/attached/ready）并记下标签名。
            /// 框架型产物（uni-app / Taro）正是在 attached 里挂载自己的组件并触发首次 setData，
            /// 所以实例必须真的建出来，光有定义没有实例的话组件模板里全是空值。
            function __mountPageComponent(tag, path, propsExpr) {
                var props = propsExpr ? __evalInPageData(propsExpr) : null;
                if (props === null || typeof props !== 'object') { props = {}; }
                var inst = __createComponentInstance(path, props);
                if (!inst) { return ''; }
                __pageComponents[tag] = inst.id;
                return inst.id;
            }

            /// 渲染数据 = 页面 data + `$comp`（各组件实例的 data）
            function __getRenderData() {
                var out = {};
                if (__currentPage && __currentPage.data) {
                    for (var k in __currentPage.data) {
                        if (__currentPage.data.hasOwnProperty(k)) { out[k] = __currentPage.data[k]; }
                    }
                }
                var comp = {};
                var any = false;
                for (var tag in __pageComponents) {
                    if (!__pageComponents.hasOwnProperty(tag)) { continue; }
                    var inst = __componentInstances[__pageComponents[tag]];
                    if (inst) { comp[tag] = inst.data; any = true; }
                }
                if (any) { out['$comp'] = comp; }
                return JSON.stringify(out);
            }

            /// 把事件派发到「持有该处理函数的对象」：先找页面，再找页面内的组件实例。
            /// 组件模板里的 `bindtap="{{item.e}}"` 绑的是**组件**的方法，
            /// 只在页面上找的话点了没反应（底部导航点不动就是这个原因）。
            function __dispatchHandler(name, event) {
                if (!name) { return false; }
                if (__currentPage && typeof __currentPage[name] === 'function') {
                    __currentPage[name](event);
                    return true;
                }
                for (var tag in __pageComponents) {
                    if (!__pageComponents.hasOwnProperty(tag)) { continue; }
                    var inst = __componentInstances[__pageComponents[tag]];
                    if (inst && typeof inst[name] === 'function') {
                        inst[name](event);
                        return true;
                    }
                }
                return false;
            }
            
            // 导航 API
            wx.navigateTo = function(options) {
                __pendingNavigation = {
                    type: 'navigateTo',
                    url: options.url
                };
                __native_print('[Navigate] navigateTo: ' + options.url);
                options.success && options.success();
            };
            
            wx.navigateBack = function(options) {
                options = options || {};
                var delta = options.delta || 1;
                __pendingNavigation = {
                    type: 'navigateBack',
                    delta: delta
                };
                // 逻辑层页面栈出栈（native 侧应配合使用实例复用而非重新加载页面 JS）
                __popPage(delta);
                __native_print('[Navigate] navigateBack');
                options.success && options.success();
            };
            
            wx.switchTab = function(options) {
                __pendingNavigation = {
                    type: 'switchTab',
                    url: options.url
                };
                __native_print('[Navigate] switchTab: ' + options.url);
                options.success && options.success();
            };
            
            wx.redirectTo = function(options) {
                __pendingNavigation = {
                    type: 'navigateTo',
                    url: options.url
                };
                __native_print('[Navigate] redirectTo: ' + options.url);
                options.success && options.success();
            };
            
            wx.reLaunch = function(options) {
                __pendingNavigation = {
                    type: 'switchTab',
                    url: options.url
                };
                __native_print('[Navigate] reLaunch: ' + options.url);
                options.success && options.success();
            };
            
            // 系统信息 API
            //
            // 字段要给全：`safeArea` / `safeAreaInsets` / `statusBarHeight` 这些是
            // 布局算式的输入（刘海屏留白、吸底栏高度），应用普遍直接点进去取值。
            // 少一个字段就是一个 `cannot read property 'bottom' of undefined`。
            wx.getSystemInfoSync = function() {
                var w = 375, h = 667, statusBar = 20;
                return {
                    platform: 'devtools',
                    system: 'iOS 15.0',
                    brand: 'devtools',
                    model: 'iPhone',
                    version: '8.0.5',
                    SDKVersion: '3.0.0',
                    language: 'zh_CN',
                    fontSizeSetting: 16,
                    theme: 'light',
                    screenWidth: w,
                    screenHeight: h,
                    windowWidth: w,
                    windowHeight: h,
                    windowTop: 0,
                    windowBottom: 0,
                    screenTop: 0,
                    pixelRatio: 2,
                    devicePixelRatio: 2,
                    statusBarHeight: statusBar,
                    // 无刘海的等效安全区：整屏可用
                    safeArea: {
                        left: 0, right: w, top: statusBar, bottom: h,
                        width: w, height: h - statusBar
                    },
                    safeAreaInsets: { top: statusBar, right: 0, bottom: 0, left: 0 },
                    deviceOrientation: 'portrait',
                    benchmarkLevel: 1
                };
            };
            // 新版拆分出来的几个查询接口，字段取自同一份系统信息
            wx.getWindowInfo = function() {
                var i = wx.getSystemInfoSync();
                return {
                    pixelRatio: i.pixelRatio, screenWidth: i.screenWidth, screenHeight: i.screenHeight,
                    windowWidth: i.windowWidth, windowHeight: i.windowHeight,
                    statusBarHeight: i.statusBarHeight, safeArea: i.safeArea,
                    screenTop: i.screenTop, windowTop: i.windowTop, windowBottom: i.windowBottom
                };
            };
            wx.getDeviceInfo = function() {
                var i = wx.getSystemInfoSync();
                return { brand: i.brand, model: i.model, system: i.system, platform: i.platform,
                         benchmarkLevel: i.benchmarkLevel, deviceOrientation: i.deviceOrientation };
            };
            wx.getAppBaseInfo = function() {
                var i = wx.getSystemInfoSync();
                return { SDKVersion: i.SDKVersion, version: i.version, language: i.language,
                         theme: i.theme, fontSizeSetting: i.fontSizeSetting };
            };
            wx.getSystemSetting = function() {
                return { bluetoothEnabled: false, locationEnabled: false, wifiEnabled: true,
                         deviceOrientation: 'portrait' };
            };
            wx.getAppAuthorizeSetting = function() {
                return { albumAuthorized: 'authorized', cameraAuthorized: 'authorized',
                         locationAuthorized: 'authorized', microphoneAuthorized: 'authorized',
                         notificationAuthorized: 'authorized' };
            };
            
            wx.getSystemInfo = function(options) {
                options = options || {};
                var info = wx.getSystemInfoSync();
                options.success && options.success(info);
                options.complete && options.complete();
            };

            // 启动参数：框架（uni-app / Taro）在 onLaunch 里必读，缺了会整个
            // onLaunch 抛异常 —— 于是 Vue 的响应式层根本没起来，页面只剩静态骨架。
            wx.getLaunchOptionsSync = function() {
                return {
                    path: (typeof __launchPath === 'string' ? __launchPath : ''),
                    scene: 1001,
                    query: {},
                    shareTicket: '',
                    referrerInfo: {},
                    forwardMaterials: [],
                    chatType: undefined,
                    apiCategory: 'default'
                };
            };
            // 「本次进入」的参数，字段与启动参数同构
            wx.getEnterOptionsSync = function() { return wx.getLaunchOptionsSync(); };

            // ───────── 网络请求 ─────────
            // 回调式、非阻塞：这里只把回调登记下来并把请求交给原生，
            // 响应由宿主每帧取回后调 __resolveRequest。
            var __requestSeq = 0;
            var __requestTasks = {};

            wx.request = function(options) {
                options = options || {};
                var id = ++__requestSeq;
                var url = options.url || '';
                var method = (options.method || 'GET').toUpperCase();
                var header = options.header || options.headers || {};
                // 微信按 dataType/method 决定怎么序列化 body：GET 的 data 拼进 query，
                // 其余情况对象转 JSON。
                var body = '';
                var data = options.data;
                if (data !== undefined && data !== null) {
                    if (method === 'GET' || method === 'HEAD') {
                        var parts = [];
                        if (typeof data === 'object') {
                            for (var k in data) {
                                if (data.hasOwnProperty(k)) {
                                    parts.push(encodeURIComponent(k) + '=' + encodeURIComponent(data[k]));
                                }
                            }
                        }
                        if (parts.length) {
                            url += (url.indexOf('?') >= 0 ? '&' : '?') + parts.join('&');
                        }
                    } else if (typeof data === 'string') {
                        body = data;
                    } else {
                        try { body = JSON.stringify(data); } catch (e) { body = String(data); }
                    }
                }

                var task = {
                    id: id,
                    aborted: false,
                    success: options.success,
                    fail: options.fail,
                    complete: options.complete,
                    // 微信默认 dataType='json'：2xx 且能解析就把 data 解析成对象
                    dataType: options.dataType === undefined ? 'json' : options.dataType,
                    abort: function() { this.aborted = true; }
                };
                __requestTasks[id] = task;

                if (typeof __native_request === 'function') {
                    __native_request(String(id), method, url, JSON.stringify(header), body,
                                     String(options.timeout || 0));
                } else {
                    // 没有原生桥（纯 JS 单测环境）：立刻失败，不要让调用方一直等
                    __resolveRequest(id, 0, '{}', '', 'request:fail 原生网络桥未注册');
                }

                // 返回 RequestTask（微信语义：可 abort）
                return {
                    abort: function() { task.abort(); },
                    onHeadersReceived: function() {},
                    offHeadersReceived: function() {}
                };
            };

            // 由原生每帧回调：把一次请求的结果分发给 success / fail / complete
            function __resolveRequest(id, statusCode, headerJson, body, errMsg) {
                var task = __requestTasks[id];
                if (!task) { return false; }
                delete __requestTasks[id];
                if (task.aborted) {
                    task.complete && task.complete({ errMsg: 'request:fail abort' });
                    return true;
                }
                var header = {};
                try { header = JSON.parse(headerJson || '{}'); } catch (e) {}
                if (errMsg) {
                    var failRes = { errMsg: errMsg };
                    task.fail && task.fail(failRes);
                    task.complete && task.complete(failRes);
                    return true;
                }
                var data = body;
                if (task.dataType === 'json') {
                    try { data = JSON.parse(body); } catch (e) { /* 不是 JSON 就原样给字符串 */ }
                }
                var res = {
                    data: data,
                    statusCode: statusCode,
                    header: header,
                    cookies: [],
                    errMsg: 'request:ok'
                };
                // 注意：4xx/5xx 在微信里**仍然走 success**，只是 statusCode 不是 2xx
                task.success && task.success(res);
                task.complete && task.complete(res);
                return true;
            }

            // downloadFile：本引擎没有可写的临时文件系统，直接把 URL 当
            // tempFilePath 回去 —— <image src> 支持远程 URL，绝大多数用法（下载图片
            // 后拿去显示）因此仍然成立。真正需要落盘的场景要另做，不假装支持。
            wx.downloadFile = function(options) {
                options = options || {};
                var url = options.url || '';
                return wx.request({
                    url: url,
                    method: 'GET',
                    dataType: 'text',
                    success: function(res) {
                        options.success && options.success({
                            tempFilePath: url,
                            filePath: url,
                            statusCode: res.statusCode,
                            errMsg: 'downloadFile:ok'
                        });
                    },
                    fail: options.fail,
                    complete: options.complete
                });
            };

            // 动态字体：本引擎的字体来自系统（见 text.rs 的字体发现），不支持按 URL
            // 远程加载。这里必须**存在且回调 success** —— 应用普遍在 onLaunch 里连着
            // 加载好几个字体，抛异常会把整个 onLaunch 打断（框架的响应式层就此起不来，
            // 页面只剩静态骨架）。降级为「用系统字体渲染」，比整个应用起不来好得多。
            wx.loadFontFace = function(options) {
                options = options || {};
                var family = options.family || '';
                __native_print('[loadFontFace] 忽略远程字体 ' + family + '（改用系统字体）');
                options.success && options.success({ status: 'loaded' });
                options.complete && options.complete({ status: 'loaded' });
            };
        "#)?;
        
        Ok(())
    }
    
    /// 初始化自定义组件 / Behavior 运行时
    ///
    /// 提供 `Component()`、`Behavior()` 以及组件实例工厂。渲染层在遇到自定义
    /// 组件标签时可调用 `__createComponentInstance` 创建实例。properties 默认值、
    /// data、methods、observers、生命周期（created/attached/ready）、triggerEvent
    /// 均已支持。渲染层与组件树的接线属于后续 native 侧工作。
    fn init_component(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __componentDefs = {};       // path -> { def, merged }
            var __componentInstances = {};  // instanceId -> instance
            var __componentSeq = 0;
            var __pendingComponentPath = '';
            var __componentEventHandlers = {}; // instanceId:eventName -> pageMethodName
            
            function __setPendingComponentPath(p) { __pendingComponentPath = p || ''; }
            
            // Behavior 返回定义本身，供 behaviors 数组引用
            function Behavior(def) { return def || {}; }
            
            // 合并 behaviors 与顶层生命周期
            function __mergeBehaviors(def) {
                var result = {
                    properties: {}, data: {}, methods: {},
                    observers: {}, lifetimes: {}, pageLifetimes: {}
                };
                function mergeOne(src) {
                    if (!src) { return; }
                    if (src.behaviors) {
                        for (var i = 0; i < src.behaviors.length; i++) { mergeOne(src.behaviors[i]); }
                    }
                    var maps = ['properties', 'data', 'methods', 'observers', 'lifetimes', 'pageLifetimes'];
                    for (var m = 0; m < maps.length; m++) {
                        var name = maps[m];
                        if (src[name]) {
                            for (var k in src[name]) {
                                if (src[name].hasOwnProperty(k)) { result[name][k] = src[name][k]; }
                            }
                        }
                    }
                    // 旧式：生命周期直接写在 def 顶层
                    var top = ['created', 'attached', 'ready', 'moved', 'detached'];
                    for (var t = 0; t < top.length; t++) {
                        if (typeof src[top[t]] === 'function') { result.lifetimes[top[t]] = src[top[t]]; }
                    }
                }
                mergeOne(def);
                return result;
            }
            
            function Component(def) {
                def = def || {};
                var path = __pendingComponentPath || ('__comp_' + (++__componentSeq));
                __componentDefs[path] = { def: def, merged: __mergeBehaviors(def) };
                // 微信允许**用 Component 构造器定义页面**，uni-app / Taro 正是走这条：
                // 它们的 `wx.createPage` 把 Vue 组件选项转成组件定义后调 `Component()`，
                // 而不是 `Page()`。只认 `Page()` 的话页面永远注册不上 ——
                // `__currentPage` 为 null、data 全空，模板里 `{{a}}` 渲染成空串、
                // `bindtap="{{c}}"` 也拿不到处理函数名，最后只剩一张静态骨架。
                //
                // 判据是「当前正在加载某个页面的 js」（__pendingRoute 非空且还没建页面）。
                // 页面 js 里注册真正的自定义组件时 __pendingComponentPath 会被置上，
                // 那种情况不当页面处理。
                if (__pendingRoute && !__pendingPageCreated && !__pendingComponentPath) {
                    var __page = Page(__componentDefToPage(def));
                    // 组件实例该有的东西：`properties` 是**求值后的属性值**（不是类型表），
                    // uni-app 的 initVueIds 会去读 `properties.uI` 并 split；
                    // 还有 triggerEvent 等实例方法。缺了就在 attached 里抛 not a function。
                    var __props = {};
                    if (def.properties) {
                        for (var __p in def.properties) {
                            if (def.properties.hasOwnProperty(__p)) {
                                __props[__p] = __propDefault(def.properties[__p]);
                            }
                        }
                    }
                    __page.properties = __props;
                    __page.is = __pendingRoute;
                    if (typeof __page.triggerEvent !== 'function') {
                        __page.triggerEvent = function() {};
                    }
                    if (typeof __page.getRelationNodes !== 'function') {
                        __page.getRelationNodes = function() { return []; };
                    }
                    if (typeof __page.selectOwnerComponent !== 'function') {
                        __page.selectOwnerComponent = function() { return null; };
                    }
                    // 组件式页面的**创建钩子**必须真的跑：`created` / `attached` 是组件
                    // 自己的生命周期，与页面的 onLoad 是两套。uni-app 正是在 attached 里
                    // 挂载 Vue 组件并触发首次 setData —— 漏掉它页面就只有一份空 data，
                    // 模板全部渲染成空串（表现为「只有静态骨架」）。
                    var __lt = def.lifetimes || {};
                    var __hooks = [def.created, __lt.created, def.attached, __lt.attached];
                    for (var __i = 0; __i < __hooks.length; __i++) {
                        if (typeof __hooks[__i] === 'function') {
                            try { __hooks[__i].call(__page); }
                            catch (e) { if (typeof console !== 'undefined') { console.error(e); } }
                        }
                    }
                }
                return def;
            }

            /// 把「用 Component 写的页面」的定义摊平成 Page() 认识的形状。
            ///
            /// 组件构造器里页面生命周期（onLoad / onShow / onReady…）与事件处理函数都写在
            /// `methods` 下，而 Page() 期望它们在顶层；`lifetimes.attached` 对应页面的
            /// 创建时机，也一并映射过去。
            function __componentAsPageConfig(def) { return __componentDefToPage(def); }
            function __componentDefToPage(def) {
                var out = {};
                for (var k in def) {
                    if (!def.hasOwnProperty(k)) { continue; }
                    if (k === 'methods' || k === 'lifetimes' || k === 'pageLifetimes' || k === 'behaviors') { continue; }
                    out[k] = def[k];
                }
                if (def.methods) {
                    for (var m in def.methods) {
                        if (def.methods.hasOwnProperty(m)) { out[m] = def.methods[m]; }
                    }
                }
                // properties 的默认值也算页面初始 data（组件式页面常把状态放这儿）
                if (def.properties) {
                    out.data = out.data || {};
                    for (var p in def.properties) {
                        if (def.properties.hasOwnProperty(p) && !(p in out.data)) {
                            out.data[p] = __propDefault(def.properties[p]);
                        }
                    }
                }
                if (def.lifetimes) {
                    // attached ≈ 页面 onLoad 之前的创建期；没有 onLoad 时用它兜底
                    if (typeof def.lifetimes.attached === 'function' && typeof out.onLoad !== 'function') {
                        out.__attached = def.lifetimes.attached;
                    }
                    if (typeof def.lifetimes.ready === 'function' && typeof out.onReady !== 'function') {
                        out.onReady = def.lifetimes.ready;
                    }
                }
                return out;
            }
            
            // 解析 properties 默认值
            function __propDefault(prop) {
                if (prop === null || prop === undefined) { return null; }
                if (prop === String) { return ''; }
                if (prop === Number) { return 0; }
                if (prop === Boolean) { return false; }
                if (prop === Array) { return []; }
                if (prop === Object) { return null; }
                if (typeof prop === 'object') {
                    if (prop.value !== undefined) { return prop.value; }
                    var ty = prop.type;
                    if (ty === String) { return ''; }
                    if (ty === Number) { return 0; }
                    if (ty === Boolean) { return false; }
                    if (ty === Array) { return []; }
                    return null;
                }
                return null;
            }
            
            // 创建组件实例（供渲染层调用）
            function __createComponentInstance(path, initialProps) {
                var entry = __componentDefs[path];
                if (!entry) { return null; }
                var merged = entry.merged;
                var id = '__ci_' + (++__componentSeq);
                
                var data = {};
                for (var pk in merged.properties) {
                    if (merged.properties.hasOwnProperty(pk)) { data[pk] = __propDefault(merged.properties[pk]); }
                }
                for (var dk in merged.data) {
                    if (merged.data.hasOwnProperty(dk)) { data[dk] = merged.data[dk]; }
                }
                initialProps = initialProps || {};
                for (var ik in initialProps) {
                    if (initialProps.hasOwnProperty(ik)) { data[ik] = initialProps[ik]; }
                }
                
                var instance = {
                    is: path,
                    id: id,
                    data: data,
                    properties: data,
                    __isComponent: true,
                    __observers: merged.observers,
                    setData: function(newData, callback) {
                        __applySetData(this, newData, callback);
                        __runObservers(this, newData);
                    },
                    triggerEvent: function(name, detail, options) {
                        __triggerComponentEvent(this, name, detail, options);
                    },
                    selectComponent: function() { return null; },
                    selectAllComponents: function() { return []; }
                };
                
                for (var mk in merged.methods) {
                    if (merged.methods.hasOwnProperty(mk)) { instance[mk] = merged.methods[mk].bind(instance); }
                }
                
                __componentInstances[id] = instance;
                
                var lc = merged.lifetimes;
                if (typeof lc.created === 'function') { try { lc.created.call(instance); } catch (e) { __native_print('[Component.created] ' + e.message); } }
                if (typeof lc.attached === 'function') { try { lc.attached.call(instance); } catch (e) { __native_print('[Component.attached] ' + e.message); } }
                if (typeof lc.ready === 'function') { try { lc.ready.call(instance); } catch (e) { __native_print('[Component.ready] ' + e.message); } }
                
                return instance;
            }
            
            // 属性变化触发 observers
            function __runObservers(instance, changed) {
                if (!instance.__observers || !changed) { return; }
                for (var key in changed) {
                    if (!changed.hasOwnProperty(key)) { continue; }
                    var field = key.split('.')[0].split('[')[0];
                    var obs = instance.__observers[key] || instance.__observers[field];
                    if (typeof obs === 'function') {
                        try { obs.call(instance, changed[key]); } catch (e) { __native_print('[observer] ' + e.message); }
                    }
                }
            }
            
            // 组件 triggerEvent -> 宿主页面绑定的处理器
            function __triggerComponentEvent(instance, name, detail, options) {
                var key = instance.id + ':' + name;
                var handlerName = __componentEventHandlers[key];
                if (handlerName && __currentPage && typeof __currentPage[handlerName] === 'function') {
                    var event = {
                        type: name,
                        timeStamp: Date.now(),
                        detail: detail || {},
                        target: { id: instance.id, dataset: {} },
                        currentTarget: { id: instance.id, dataset: {} }
                    };
                    try { __currentPage[handlerName](event); }
                    catch (e) { __native_print('[triggerEvent] ' + e.message); }
                }
            }
            
            // 渲染层登记组件事件绑定
            function __bindComponentEvent(instanceId, eventName, pageMethod) {
                __componentEventHandlers[instanceId + ':' + eventName] = pageMethod;
            }
            
            // ==================== 组件生命周期补全 ====================
            // 页面 show/hide/resize 联动所有组件的 pageLifetimes
            function __dispatchAllComponentsPageLifetime(name, arg) {
                for (var id in __componentInstances) {
                    if (!__componentInstances.hasOwnProperty(id)) { continue; }
                    var inst = __componentInstances[id];
                    var entry = __componentDefs[inst.is];
                    if (!entry) { continue; }
                    var pl = entry.merged.pageLifetimes;
                    if (pl && typeof pl[name] === 'function') {
                        try { pl[name].call(inst, arg); }
                        catch (e) { __native_print('[Component.pageLifetimes.' + name + '] ' + e.message); }
                    }
                }
            }
            
            // 组件从节点树移除：触发 detached 生命周期并回收实例
            function __detachComponentInstance(id) {
                var inst = __componentInstances[id];
                if (!inst) { return; }
                var entry = __componentDefs[inst.is];
                if (entry && typeof entry.merged.lifetimes.detached === 'function') {
                    try { entry.merged.lifetimes.detached.call(inst); }
                    catch (e) { __native_print('[Component.detached] ' + e.message); }
                }
                delete __componentInstances[id];
            }
            
            // 组件在节点树中被移动：触发 moved 生命周期
            function __moveComponentInstance(id) {
                var inst = __componentInstances[id];
                if (!inst) { return; }
                var entry = __componentDefs[inst.is];
                if (entry && typeof entry.merged.lifetimes.moved === 'function') {
                    try { entry.merged.lifetimes.moved.call(inst); }
                    catch (e) { __native_print('[Component.moved] ' + e.message); }
                }
            }
            
            // 页面卸载时回收其全部组件实例
            function __detachAllComponents() {
                var ids = [];
                for (var id in __componentInstances) {
                    if (__componentInstances.hasOwnProperty(id)) { ids.push(id); }
                }
                for (var i = 0; i < ids.length; i++) { __detachComponentInstance(ids[i]); }
            }
        "#)?;
        Ok(())
    }
}
