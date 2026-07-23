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
        "#)?;
        Ok(())
    }
    
    fn init_console(&self) -> Result<(), String> {
        let rt = self.runtime.lock().unwrap();
        rt.eval(r#"
            var __console_buffer = [];
            var console = {
                log: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
                    __console_buffer.push('[LOG] ' + msg);
                    if (typeof __native_print === 'function') { __native_print(msg); }
                },
                error: function() {
                    var msg = Array.prototype.slice.call(arguments).join(' ');
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
            
            // 下拉刷新相关
            wx.startPullDownRefresh = function(options) {
                options = options || {};
                __native_print('[PullDownRefresh] start');
                options.success && options.success();
                options.complete && options.complete();
            };
            
            wx.stopPullDownRefresh = function(options) {
                options = options || {};
                __native_print('[PullDownRefresh] stop');
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
            var __currentPage = null;
            var __pendingNavigation = null;
            
            function App(config) {
                __app = config;
                if (config.onLaunch) {
                    config.onLaunch();
                }
                return __app;
            }
            
            function getApp() {
                return __app;
            }
            
            function Page(config) {
                // 创建页面实例
                var page = {
                    data: config.data || {},
                    
                    // setData 方法 - 更新数据并触发重新渲染
                    setData: function(newData, callback) {
                        // 合并数据
                        for (var key in newData) {
                            if (newData.hasOwnProperty(key)) {
                                this.data[key] = newData[key];
                            }
                        }
                        // 通知 native 层数据更新
                        if (typeof __native_page_update === 'function') {
                            __native_page_update(JSON.stringify(this.data));
                        }
                        // 执行回调
                        if (callback) {
                            callback();
                        }
                    }
                };
                
                // 复制所有方法到页面实例
                for (var key in config) {
                    if (config.hasOwnProperty(key) && key !== 'data') {
                        if (typeof config[key] === 'function') {
                            page[key] = config[key].bind(page);
                        } else {
                            page[key] = config[key];
                        }
                    }
                }
                
                // 保存当前页面
                __currentPage = page;
                
                return page;
            }
            
            function getCurrentPages() {
                return __currentPage ? [__currentPage] : [];
            }
            
            // 获取当前页面实例（供 native 调用）
            function __getPageInstance() {
                return __currentPage;
            }
            
            // 调用页面方法（供 native 调用事件处理）
            function __callPageMethod(methodName, eventData) {
                if (__currentPage && typeof __currentPage[methodName] === 'function') {
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
                    
                    var event = {
                        type: 'tap',
                        currentTarget: {
                            dataset: dataset
                        },
                        detail: dataset
                    };
                    try {
                        __currentPage[methodName](event);
                    } catch (e) {
                        __native_print('[Error] ' + methodName + ': ' + e.message);
                    }
                    return true;
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
                __pendingNavigation = {
                    type: 'navigateBack',
                    delta: options.delta || 1
                };
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
            wx.getSystemInfoSync = function() {
                return {
                    platform: 'mini-app',
                    version: '1.0.0',
                    SDKVersion: '1.0.0',
                    windowWidth: 375,
                    windowHeight: 667,
                    pixelRatio: 2
                };
            };
            
            wx.getSystemInfo = function(options) {
                options = options || {};
                var info = wx.getSystemInfoSync();
                options.success && options.success(info);
                options.complete && options.complete();
            };
        "#)?;
        
        Ok(())
    }
}
