            // ───────── 小程序信息、网络状态、胶囊按钮、能力探测、更新、分享、屏幕与杂项 ─────────
            // 宿主能给真值的（网络类型、胶囊位置、电量、appId…）都从 __hostSystemInfo 取，
            // 没给时是一份与微信 iOS 默认外观一致的值；纯意图类（分享菜单、常亮、背景色）发宿主事件。

            function __hostField(k) {
                return (typeof __hostSystemInfo === 'object' && __hostSystemInfo) ? __hostSystemInfo[k] : undefined;
            }

            // ---------- 网络状态（api/device/network）----------
            var __netType = null, __netListeners = [], __netWeakListeners = [];
            wx.getNetworkType = function(o) {
                var t = __netType || __hostField('networkType') || 'wifi';
                return __syncOk(o, 'getNetworkType', { networkType: t, signalStrength: 0, hasSystemProxy: false });
            };
            wx.onNetworkStatusChange = function(cb) { if (typeof cb === 'function') __netListeners.push(cb); };
            wx.offNetworkStatusChange = function(cb) { __netListeners = cb ? __netListeners.filter(function(f) { return f !== cb; }) : []; };
            wx.onNetworkWeakChange = function(cb) { if (typeof cb === 'function') __netWeakListeners.push(cb); };
            wx.offNetworkWeakChange = function(cb) { __netWeakListeners = cb ? __netWeakListeners.filter(function(f) { return f !== cb; }) : []; };
            // 宿主网络变了：eval("__dispatchNetworkStatus('4g')")；'none' 表示断网
            globalThis.__dispatchNetworkStatus = function(type) {
                __netType = String(type || 'none');
                var res = { isConnected: __netType !== 'none', networkType: __netType };
                __netListeners.slice().forEach(function(cb) { try { cb(res); } catch (e) { console.error(e); } });
            };

            // ---------- 胶囊按钮（api/ui/menu/wx.getMenuButtonBoundingClientRect）----------
            // 微信的胶囊：87×32，距右 7；距状态栏底 iOS 4、Android 8（48 高的导航栏里上下居中）。
            // 与引擎画胶囊的 `host::capsule::rect` 是同一条公式。宿主有真值时给 __hostSystemInfo.menuButton
            wx.getMenuButtonBoundingClientRect = function() {
                var mb = __hostField('menuButton');
                if (mb && typeof mb === 'object') return JSON.parse(JSON.stringify(mb));
                var i = wx.getSystemInfoSync();
                var gap = String(i.platform).toLowerCase() === 'android' ? 8 : 4;
                var top = (i.statusBarHeight || 20) + gap, right = i.windowWidth - 7;
                return { width: 87, height: 32, top: top, bottom: top + 32, left: right - 87, right: right };
            };

            // ---------- 能力探测（api/base/wx.canIUse）----------
            // `${API}.${method}.${param}.${option}` 或 `${component}.${attribute}.${option}`。
            // API 看 wx 上有没有**真的实现**（system.js 里「立刻 fail」的默认实现算没有）；
            // 组件看引擎自己画的那张表。更细的段（参数、返回值、属性取值）不逐项核对，按支持算。
            var __CANIUSE_COMPONENTS = ['view', 'block', 'scroll-view', 'swiper', 'swiper-item', 'text', 'rich-text', 'icon', 'image',
                'video', 'canvas', 'button', 'input', 'textarea', 'switch', 'slider', 'progress', 'checkbox', 'checkbox-group',
                'radio', 'radio-group', 'picker', 'picker-view', 'picker-view-column', 'form', 'navigator', 'label', 'map',
                // 渲染器补的（components::registry / grid_view / native_placeholder）与 movable-area（带默认样式的容器）
                'grid-view', 'list-view', 'cover-view', 'cover-image', 'span', 'movable-area',
                // 引擎内置组件（builtin_components，页面用到才注入；单测核对这里一个不漏）
                'page-container', 'match-media', 'root-portal', 'page-meta', 'navigation-bar', 'draggable-sheet',
                'audio', 'editor', 'movable-view',
                'tap-gesture-handler', 'double-tap-gesture-handler', 'long-press-gesture-handler', 'pan-gesture-handler',
                'horizontal-drag-gesture-handler', 'vertical-drag-gesture-handler', 'scale-gesture-handler', 'force-press-gesture-handler'];
            wx.canIUse = function(schema) {
                var parts = String(schema || '').split('.');
                var head = parts[0];
                if (!head) return false;
                if (head === 'wx' && parts.length > 1) head = parts[1];
                if (__CANIUSE_COMPONENTS.indexOf(head) >= 0) return true;
                var f = (typeof __wxImpl === 'object' ? __wxImpl : wx)[head];
                if (typeof f === 'function') return !f.__default;
                return f !== undefined && f !== null;
            };

            // ---------- 更新管理（api/base/update）：引擎按源码目录跑，没有「新版本」----------
            var __updater = null;
            wx.getUpdateManager = function() {
                if (__updater) return __updater;
                __updater = {
                    applyUpdate: function() {},
                    onCheckForUpdate: function(cb) { if (typeof cb === 'function') setTimeout(function() { cb({ hasUpdate: false }); }, 0); },
                    onUpdateReady: function() {},
                    onUpdateFailed: function() {}
                };
                return __updater;
            };

            // ---------- 账号信息（api/open-api/account-info）----------
            wx.getAccountInfoSync = function() {
                var acc = __hostField('accountInfo');
                if (acc && typeof acc === 'object') return JSON.parse(JSON.stringify(acc));
                return {
                    miniProgram: { appId: String(__hostField('appId') || (typeof __projectAppId === 'string' ? __projectAppId : '') || ''),
                                   envVersion: 'develop', version: '' },
                    plugin: {}
                };
            };

            // ---------- 授权设置（api/open-api/setting）：由 getAppAuthorizeSetting 推出来 ----------
            wx.getSetting = function(o) {
                var host = __hostField('authSetting');
                var auth = {};
                if (host && typeof host === 'object') { auth = JSON.parse(JSON.stringify(host)); }
                else {
                    var a = wx.getAppAuthorizeSetting();
                    auth['scope.userLocation'] = a.locationAuthorized === 'authorized';
                    auth['scope.camera'] = a.cameraAuthorized === 'authorized';
                    auth['scope.record'] = a.microphoneAuthorized === 'authorized';
                    auth['scope.writePhotosAlbum'] = a.albumAuthorized === 'authorized';
                }
                return __syncOk(o, 'getSetting', { authSetting: auth, subscriptionsSetting: { mainSwitch: true } });
            };

            // ---------- 分享菜单（api/share）：意图交给宿主（事件 shareMenu）----------
            ['showShareMenu', 'hideShareMenu', 'updateShareMenu', 'showShareImageMenu'].forEach(function(name) {
                wx[name] = function(o) {
                    o = o || {};
                    var ev = { type: 'shareMenu', action: name };
                    for (var k in o) { if (o.hasOwnProperty(k) && typeof o[k] !== 'function') ev[k] = o[k]; }
                    __emitHostEvent(ev);
                    return __syncOk(o, name);
                };
            });

            // ---------- 屏幕（api/device/screen）----------
            var __brightness = 0.5;
            wx.setKeepScreenOn = function(o) {
                o = o || {};
                __emitHostEvent({ type: 'keepScreenOn', keepScreenOn: !!o.keepScreenOn });
                return __syncOk(o, 'setKeepScreenOn');
            };
            wx.setScreenBrightness = function(o) {
                o = o || {};
                __brightness = Math.max(0, Math.min(1, Number(o.value)));
                __emitHostEvent({ type: 'screenBrightness', value: __brightness });
                return __syncOk(o, 'setScreenBrightness');
            };
            wx.getScreenBrightness = function(o) {
                var v = __hostField('screenBrightness');
                return __syncOk(o, 'getScreenBrightness', { value: typeof v === 'number' ? v : __brightness });
            };
            var __captureListeners = [];
            wx.onUserCaptureScreen = function(cb) { if (typeof cb === 'function') __captureListeners.push(cb); };
            wx.offUserCaptureScreen = function(cb) { __captureListeners = cb ? __captureListeners.filter(function(f) { return f !== cb; }) : []; };
            globalThis.__dispatchUserCaptureScreen = function() { __captureListeners.slice().forEach(function(cb) { cb({}); }); };

            // ---------- 窗口背景（api/ui/background）：下拉回弹露出来的那块颜色，由引擎画 ----------
            wx.setBackgroundColor = function(o) {
                o = o || {};
                var req = {};
                ['backgroundColor', 'backgroundColorTop', 'backgroundColorBottom'].forEach(function(k) { if (o[k]) req[k] = String(o[k]); });
                __native_host_request('backgroundColor', JSON.stringify(req));
                return __syncOk(o, 'setBackgroundColor');
            };
            wx.setBackgroundTextStyle = function(o) {
                o = o || {};
                __emitHostEvent({ type: 'backgroundTextStyle', textStyle: String(o.textStyle || 'dark') });
                return __syncOk(o, 'setBackgroundTextStyle');
            };

            // ---------- 电量（api/device/battery）----------
            wx.getBatteryInfoSync = function() {
                var b = __hostField('battery');
                return (b && typeof b === 'object') ? { level: b.level, isCharging: !!b.isCharging } : { level: 100, isCharging: true };
            };
            wx.getBatteryInfo = function(o) { return __syncOk(o, 'getBatteryInfo', wx.getBatteryInfoSync()); };

            // ---------- 键盘（api/ui/keyboard）----------
            var __kbListeners = [];
            wx.hideKeyboard = function(o) {
                __native_host_request('hideKeyboard', '{}');
                return __syncOk(o, 'hideKeyboard');
            };
            wx.onKeyboardHeightChange = function(cb) { if (typeof cb === 'function') __kbListeners.push(cb); };
            wx.offKeyboardHeightChange = function(cb) { __kbListeners = cb ? __kbListeners.filter(function(f) { return f !== cb; }) : []; };
            // 宿主键盘弹起 / 收起：eval("__dispatchKeyboardHeight(300)")
            globalThis.__dispatchKeyboardHeight = function(h) {
                var res = { height: Number(h) || 0, duration: 0.25 };
                __kbListeners.slice().forEach(function(cb) { try { cb(res); } catch (e) { console.error(e); } });
            };

            // ---------- 日志与上报（api/base/debug、data-analysis）：落到 console ----------
            wx.getLogManager = function() {
                var w = function(level) { return function() { console[level === 'warn' ? 'warn' : 'log'].apply(console, ['[LogManager]'].concat([].slice.call(arguments))); }; };
                return { log: w('log'), info: w('log'), warn: w('warn'), debug: w('log') };
            };
            wx.getRealtimeLogManager = function() {
                var m = {
                    info: function() { console.log.apply(console, ['[RealtimeLog]'].concat([].slice.call(arguments))); },
                    warn: function() { console.warn.apply(console, ['[RealtimeLog]'].concat([].slice.call(arguments))); },
                    error: function() { console.error.apply(console, ['[RealtimeLog]'].concat([].slice.call(arguments))); },
                    setFilterMsg: function() {}, addFilterMsg: function() {}
                };
                m.tag = function() { return m; };
                return m;
            };
            wx.reportAnalytics = function() {};
            wx.reportEvent = function() {};
            wx.reportMonitor = function() {};
            wx.reportPerformance = function() {};
            wx.getPerformance = function() {
                var t0 = Date.now();
                return {
                    now: function() { return Date.now() - t0; },
                    getEntries: function() { return []; }, getEntriesByName: function() { return []; }, getEntriesByType: function() { return []; },
                    createObserver: function() { return { observe: function() {}, disconnect: function() {} }; },
                    setBufferSize: function() {}
                };
            };
            var __memListeners = [];
            wx.onMemoryWarning = function(cb) { if (typeof cb === 'function') __memListeners.push(cb); };
            wx.offMemoryWarning = function(cb) { __memListeners = cb ? __memListeners.filter(function(f) { return f !== cb; }) : []; };
            globalThis.__dispatchMemoryWarning = function(level) { __memListeners.slice().forEach(function(cb) { cb({ level: level || 10 }); }); };
            wx.getExtConfigSync = function() { var e = __hostField('extConfig'); return (e && typeof e === 'object') ? e : {}; };
            wx.getExtConfig = function(o) { return __syncOk(o, 'getExtConfig', { extConfig: wx.getExtConfigSync() }); };

            // ---------- 图片信息与压缩（api/media/image）：原生解码 / 重新编码 ----------
            function __imageInfoLocal(o, src, name) {
                var r = {};
                try { r = JSON.parse(__native_image_info(String(src))); } catch (e) { r = { err: String(e) }; }
                var res = r.err ? { errMsg: name + ':fail ' + r.err } :
                    { width: r.width, height: r.height, type: r.type, orientation: 'up', path: String(src), errMsg: name + ':ok' };
                setTimeout(function() {
                    if (r.err) { o.fail && o.fail(res); o.reject && o.reject(res); }
                    else { o.success && o.success(res); o.resolve && o.resolve(res); }
                    o.complete && o.complete(res);
                }, 0);
            }
            wx.getImageInfo = function(o) {
                o = o || {};
                var promise;
                if (!o.success && !o.fail && !o.complete) {
                    promise = new Promise(function(resolve, reject) { o = { src: o.src, resolve: resolve, reject: reject }; });
                }
                var src = String(o.src || '');
                if (/^https?:\/\//.test(src)) {
                    wx.downloadFile({ url: src, success: function(d) { __imageInfoLocal(o, d.tempFilePath, 'getImageInfo'); },
                                      fail: function(e) { __imageInfoLocal(o, '', 'getImageInfo'); } });
                } else {
                    __imageInfoLocal(o, src, 'getImageInfo');
                }
                return promise;
            };
            wx.compressImage = function(o) {
                o = o || {};
                var r = {};
                try {
                    r = JSON.parse(__native_compress_image(String(o.src || ''), String(o.quality === undefined ? 80 : o.quality),
                                                           String(o.compressedWidth || 0), String(o.compressedHeight || 0)));
                } catch (e) { r = { err: String(e) }; }
                var res = r.err ? { errMsg: 'compressImage:fail ' + r.err } : { tempFilePath: r.tempFilePath, errMsg: 'compressImage:ok' };
                var promise;
                if (!o.success && !o.fail && !o.complete) {
                    return new Promise(function(resolve, reject) { setTimeout(function() { (r.err ? reject : resolve)(res); }, 0); });
                }
                setTimeout(function() {
                    if (r.err) { o.fail && o.fail(res); } else { o.success && o.success(res); }
                    o.complete && o.complete(res);
                }, 0);
                return promise;
            };
