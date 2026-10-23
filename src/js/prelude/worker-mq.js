            // ───────── Worker 与 MediaQueryObserver ─────────

            // ---------- wx.createWorker（官方 api/worker）----------
            // 微信的 worker 是另一条线程上的 JS 环境。本引擎的逻辑层只有一个 QuickJS，所以 worker 脚本在**同一个运行时**里、
            // 以独立的模块作用域执行（它 require 的模块也按 worker 目录解析），消息双向都走 JSON 深拷贝 + 下一轮事件循环 ——
            // 对页面来说语义一样（异步、不共享对象），只是不能把真正的长计算挪出逻辑线程。
            // worker 脚本里的全局 `worker`（onMessage / postMessage）在 worker 活着期间指向它自己的端点（官方同时只允许一个 worker）。
            var __activeWorker = null;
            function __cloneMsg(m) { return m === undefined ? undefined : JSON.parse(JSON.stringify(m)); }
            wx.createWorker = function(scriptPath, opt) {
                if (__activeWorker) __activeWorker.terminate();
                var key = __stripJs(__normalizePath(String(scriptPath || '').replace(/^\/+/, '')));
                var mainLs = { message: [], error: [], processKilled: [] }, workerLs = [];
                var alive = true;
                var mainSide = {
                    postMessage: function(msg) {
                        var m = __cloneMsg(msg);
                        setTimeout(function() { if (alive) workerLs.slice().forEach(function(cb) { try { cb(m); } catch (e) { fireErr(e); } }); }, 0);
                    },
                    onMessage: function(cb) { if (typeof cb === 'function') mainLs.message.push(cb); },
                    onError: function(cb) { if (typeof cb === 'function') mainLs.error.push(cb); },
                    onProcessKilled: function(cb) { if (typeof cb === 'function') mainLs.processKilled.push(cb); },
                    terminate: function() {
                        alive = false;
                        if (__activeWorker === mainSide) __activeWorker = null;
                        if (globalThis.worker === workerSide) globalThis.worker = undefined;
                    },
                    testOnProcessKilled: function() {
                        setTimeout(function() { mainLs.processKilled.forEach(function(cb) { cb({}); }); mainSide.terminate(); }, 0);
                    },
                    getCameraFrameData: function() { return null; }
                };
                var workerSide = {
                    onMessage: function(cb) { if (typeof cb === 'function') workerLs.push(cb); },
                    postMessage: function(msg) {
                        var m = __cloneMsg(msg);
                        setTimeout(function() { if (alive) mainLs.message.slice().forEach(function(cb) { try { cb(m); } catch (e) { console.error(e); } }); }, 0);
                    },
                    getCameraFrameData: function() { return null; },
                    testOnProcessKilled: function() { mainSide.testOnProcessKilled(); }
                };
                function fireErr(e) {
                    var res = { message: e && e.message ? e.message : String(e), filename: key, lineno: 0 };
                    setTimeout(function() { mainLs.error.forEach(function(cb) { cb(res); }); }, 0);
                }
                __activeWorker = mainSide;
                var factory = __modules[key] || __modules[key + '/index'];
                if (!factory) {
                    fireErr(new Error('worker 脚本不存在：' + scriptPath + '（app.json 的 workers 目录里的 js 会被预先登记）'));
                    return mainSide;
                }
                globalThis.worker = workerSide;
                try {
                    var module = { exports: {} }, dir = __dirname(key);
                    factory.call(module.exports, module, module.exports, function(req) { return __require(dir, req); });
                } catch (e) { fireErr(e); }
                return mainSide;
            };

            // ---------- MediaQueryObserver（官方 api/wxml/MediaQueryObserver）：页面 / 组件的 this.createMediaQueryObserver() ----------
            // 判据取窗口尺寸（getWindowInfo），窗口变了（mr_app_resize → onWindowResize）重新判，结果变了才回调；
            // observe 之后马上按当前状态回一次（与微信一致：页面拿到初始的 matches）
            var __mqObservers = [];
            function __mqMatch(d) {
                var w = wx.getWindowInfo(), W = w.windowWidth, H = w.windowHeight;
                var n = function(k) { return typeof d[k] === 'number' ? d[k] : null; };
                if (n('width') !== null && W !== d.width) return false;
                if (n('minWidth') !== null && W < d.minWidth) return false;
                if (n('maxWidth') !== null && W > d.maxWidth) return false;
                if (n('height') !== null && H !== d.height) return false;
                if (n('minHeight') !== null && H < d.minHeight) return false;
                if (n('maxHeight') !== null && H > d.maxHeight) return false;
                if (d.orientation === 'landscape' && !(W > H)) return false;
                if (d.orientation === 'portrait' && !(H >= W)) return false;
                return true;
            }
            function __attachMediaQuery(inst) {
                inst.createMediaQueryObserver = function() {
                    var entry = { d: null, cb: null, last: null };
                    return {
                        observe: function(descriptor, cb) {
                            entry.d = descriptor || {};
                            entry.cb = cb;
                            entry.last = __mqMatch(entry.d);
                            __mqObservers.push(entry);
                            var m = entry.last;
                            setTimeout(function() { if (typeof cb === 'function') cb({ matches: m }); }, 0);
                        },
                        disconnect: function() { __mqObservers = __mqObservers.filter(function(e) { return e !== entry; }); }
                    };
                };
            }
            // 挂在宿主派发窗口变化的入口上（不走 wx.onWindowResize：页面一句 wx.offWindowResize() 会把它也摘掉）
            (function() {
                var raw = __dispatchWindowResize;
                __dispatchWindowResize = function() {
                    raw.apply(null, arguments);
                    __mqObservers.slice().forEach(function(e) {
                        var m = __mqMatch(e.d);
                        if (m !== e.last) { e.last = m; if (typeof e.cb === 'function') e.cb({ matches: m }); }
                    });
                };
            })();
