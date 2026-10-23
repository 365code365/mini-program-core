            // ───────── 基础 / 系统 / 缓存 / 隐私：引擎里能真做的那批（对照官方 api/base、api/storage、api/open-api/privacy）─────────

            // 异步回调的统一写法：有回调走回调（下一轮事件循环），一个回调都没给时返回 Promise
            function __asyncOk(o, name, res) {
                o = o || {};
                res = res || {};
                res.errMsg = name + ':ok';
                if (!o.success && !o.fail && !o.complete) {
                    return new Promise(function(resolve) { setTimeout(function() { resolve(res); }, 0); });
                }
                setTimeout(function() { o.success && o.success(res); o.complete && o.complete(res); }, 0);
            }
            function __asyncFail(o, name, msg) {
                o = o || {};
                var bad = { errMsg: name + ':fail ' + msg };
                if (!o.success && !o.fail && !o.complete) {
                    return new Promise(function(resolve, reject) { setTimeout(function() { reject(bad); }, 0); });
                }
                setTimeout(function() { o.fail && o.fail(bad); o.complete && o.complete(bad); }, 0);
            }

            // ---------- 系统 ----------
            // 官方的 getSystemInfo 是异步的（complete 也带 res），新版基础库无回调时返回 Promise
            wx.getSystemInfo = function(o) { return __asyncOk(o, 'getSystemInfo', wx.getSystemInfoSync()); };
            wx.getSystemInfoAsync = function(o) { return __asyncOk(o, 'getSystemInfoAsync', wx.getSystemInfoSync()); };
            // 本引擎就是 Skyline 的对标实现：总是「支持」
            wx.getSkylineInfoSync = function() { return { isSupported: true, version: '1.4.0', reason: '' }; };
            wx.getSkylineInfo = function(o) { return __asyncOk(o, 'getSkylineInfo', wx.getSkylineInfoSync()); };
            wx.getRendererUserAgent = function(o) {
                var i = wx.getSystemInfoSync();
                var ua = 'Mozilla/5.0 (' + i.platform + '; ' + i.system + ') MiniRender/0.1 MicroMessenger/' + i.version +
                         ' miniProgram Skyline/1.4.0';
                return __asyncOk(o, 'getRendererUserAgent', { userAgent: ua });
            };
            wx.getDeviceBenchmarkInfo = function(o) {
                var lv = wx.getSystemInfoSync().benchmarkLevel;
                return __asyncOk(o, 'getDeviceBenchmarkInfo', { benchmarkLevel: lv, modelLevel: lv > 0 ? Math.min(4, lv) : 0 });
            };
            wx.getApiCategory = function() { return { apiCategory: 'default' }; };
            wx.setEnableDebug = function(o) { return __asyncOk(o, 'setEnableDebug'); };
            wx.checkIsPictureInPictureActive = function() { return false; };
            wx.getShowSplashAdStatus = function(o) { return __asyncOk(o, 'getShowSplashAdStatus', { status: 'invalid' }); };
            wx.getExptInfoSync = function() { return {}; };
            wx.isVKSupport = function() { return false; };
            wx.checkIsOpenAccessibility = function(o) {
                var v = (typeof __hostSystemInfo === 'object' && __hostSystemInfo) ? __hostSystemInfo.accessibility : undefined;
                return __asyncOk(o, 'checkIsOpenAccessibility', { open: !!v });
            };
            wx.getScreenRecordingState = function(o) { return __asyncOk(o, 'getScreenRecordingState', { state: __screenRecording }); };
            // 跳系统设置页：只有宿主能做
            ['openSystemBluetoothSetting', 'openAppAuthorizeSetting'].forEach(function(name) {
                wx[name] = function(o) { return __hostInvoke(name, o); };
                wx[name].__default = true;
            });

            // 预加载 / 分包：引擎一次读入整个代码包、没有独立的 webview 进程，这些都立即成功
            ['preloadWebview', 'preloadSkylineView', 'preloadAssets'].forEach(function(name) {
                wx[name] = function(o) { return __asyncOk(o, name); };
            });
            wx.preDownloadSubpackage = function(o) {
                var ls = [];
                var task = { onProgressUpdate: function(cb) { if (typeof cb === 'function') ls.push(cb); } };
                setTimeout(function() {
                    ls.forEach(function(cb) { cb({ progress: 100, totalBytesWritten: 0, totalBytesExpectedToWrite: 0 }); });
                    __asyncOk(o, 'preDownloadSubpackage', { packageName: (o && (o.packageName || o.packageType)) || '' });
                }, 0);
                return task;
            };
            wx.getPluginUpdateManager = function() {
                return { onCheckForUpdate: function(cb) { if (typeof cb === 'function') setTimeout(function() { cb({ hasUpdate: false }); }, 0); },
                         onUpdateReady: function() {}, onUpdateFailed: function() {}, applyUpdate: function() {} };
            };

            // 空闲回调：逻辑层空下来就跑（这里按「下一轮事件循环」近似，timeRemaining 给一帧的预算）
            var __idleSeq = 0, __idle = {};
            wx.requestIdleCallback = function(cb, opt) {
                var id = ++__idleSeq, t0 = Date.now();
                __idle[id] = setTimeout(function() {
                    delete __idle[id];
                    if (typeof cb === 'function') cb({ didTimeout: false, timeRemaining: function() { return Math.max(0, 16 - (Date.now() - t0)); } });
                }, 1);
                return id;
            };
            wx.cancelIdleCallback = function(id) { if (__idle[id]) { clearTimeout(__idle[id]); delete __idle[id]; } };

            // ---------- 随机数与加密 ----------
            function __randomBuffer(n) {
                var b64 = __native_random(String(Math.max(0, Math.min(65536, Number(n) || 0))));
                return __b64decode(b64);
            }
            wx.getRandomValues = function(o) {
                o = o || {};
                var n = Number(o.length);
                if (!(n > 0) || n > 65536) return __asyncFail(o, 'getRandomValues', 'parameter error: length should be 1~65536');
                return __asyncOk(o, 'getRandomValues', { randomValues: __randomBuffer(n) });
            };
            wx.getUserCryptoManager = function() {
                return {
                    getRandomValues: function(o) { return wx.getRandomValues(o); },
                    // 用户密钥由微信后台下发：交给宿主
                    getLatestUserKey: function(o) { return __hostInvoke('UserCryptoManager.getLatestUserKey', o); }
                };
            };

            // ---------- 界面 ----------
            wx.hideHomeButton = function(o) { __emitHostEvent({ type: 'navigationBar', homeButton: false }); return __asyncOk(o, 'hideHomeButton'); };
            // 只在 iOS 置顶聊天里有用，基础库 2.x 起废弃：照常成功
            wx.setTopBarText = function(o) { return __asyncOk(o, 'setTopBarText'); };
            // PC 端调窗口大小：交给宿主（事件 windowSize），移动端宿主可以忽略
            wx.setWindowSize = function(o) {
                o = o || {};
                __emitHostEvent({ type: 'windowSize', width: Number(o.width) || 0, height: Number(o.height) || 0 });
                return __asyncOk(o, 'setWindowSize');
            };
            // 内置字体（TencentSans 等）是微信客户端带的，引擎没有
            wx.loadBuiltInFontFace = function(o) { return __asyncFail(o, 'loadBuiltInFontFace', '当前环境没有内置字体'); };
            // 当前聚焦输入框的光标（引擎在聚焦 / 输入时写 __focusedInput，见 host 的 inputFocus）
            wx.getSelectedTextRange = function(o) {
                var f = typeof __focusedInput === 'object' ? __focusedInput : null;
                if (!f) return __asyncFail(o, 'getSelectedTextRange', 'no focused input');
                return __asyncOk(o, 'getSelectedTextRange', { start: f.start, end: f.end });
            };

            // ---------- 隐私（官方 api/open-api/privacy）：引擎自己不收集任何用户信息 ----------
            // 需要用户同意的那几类能力都在宿主那边，所以「是否需要授权」由宿主给（__hostSystemInfo.privacy），默认不需要
            wx.getPrivacySetting = function(o) {
                var p = (typeof __hostSystemInfo === 'object' && __hostSystemInfo && __hostSystemInfo.privacy) || {};
                return __asyncOk(o, 'getPrivacySetting', { needAuthorization: !!p.needAuthorization, privacyContractName: String(p.privacyContractName || '') });
            };
            wx.requirePrivacyAuthorize = function(o) { return __asyncOk(o, 'requirePrivacyAuthorize'); };

            // ---------- 广告：引擎没有广告组件，load / show 一律 1004（无合适的广告，官方错误码表） ----------
            function __makeAd(name) {
                var ls = { load: [], error: [], close: [] };
                var bad = { errMsg: name + ':fail no advertisement', errCode: 1004 };
                var ad = {
                    load: function() { setTimeout(function() { ls.error.forEach(function(cb) { cb(bad); }); }, 0); return Promise.reject(bad); },
                    show: function() { return Promise.reject(bad); },
                    destroy: function() { ls = { load: [], error: [], close: [] }; }
                };
                ['Load', 'Error', 'Close'].forEach(function(N) {
                    var k = N.toLowerCase();
                    ad['on' + N] = function(cb) { if (typeof cb === 'function') ls[k].push(cb); };
                    ad['off' + N] = function(cb) { ls[k] = cb ? ls[k].filter(function(f) { return f !== cb; }) : []; };
                });
                // 创建后自动拉一次（微信语义），失败走 onError
                setTimeout(function() { ls.error.forEach(function(cb) { cb(bad); }); }, 0);
                return ad;
            }
            wx.createRewardedVideoAd = function() { return __makeAd('RewardedVideoAd'); };
            wx.createInterstitialAd = function() { return __makeAd('InterstitialAd'); };

            // ---------- AI：没有推理引擎 / VisionKit，对象照给，用的时候报不支持 ----------
            wx.createInferenceSession = function() {
                var ls = { load: [], error: [] };
                var bad = { errMsg: 'createInferenceSession:fail 当前环境不支持 AI 推理' };
                var s = {
                    run: function() { return Promise.reject(bad); },
                    destroy: function() {},
                    onLoad: function(cb) { if (typeof cb === 'function') ls.load.push(cb); },
                    offLoad: function(cb) { ls.load = cb ? ls.load.filter(function(f) { return f !== cb; }) : []; },
                    onError: function(cb) { if (typeof cb === 'function') ls.error.push(cb); },
                    offError: function(cb) { ls.error = cb ? ls.error.filter(function(f) { return f !== cb; }) : []; }
                };
                setTimeout(function() { ls.error.forEach(function(cb) { cb(bad); }); }, 0);
                return s;
            };
            wx.getInferenceEnvInfo = function(o) { return __asyncFail(o, 'getInferenceEnvInfo', '当前环境不支持 AI 推理'); };
            wx.createVKSession = function() {
                var noop = function() {};
                return {
                    state: 0, config: {}, cameraSize: { width: 0, height: 0 },
                    start: function(cb) { if (typeof cb === 'function') setTimeout(function() { cb(2003); }, 0); },
                    stop: noop, destroy: noop, update3DMode: noop, updateMaskMode: noop, updateOSDThreshold: noop, setDepthOccRange: noop,
                    on: noop, off: noop, getVKFrame: function() { return null; }, hitTest: function() { return []; },
                    addMarker: function() { return -1; }, removeMarker: noop, getAllMarker: function() { return []; },
                    addOSDMarker: function() { return -1; }, removeOSDMarker: noop, getAllOSDMarker: function() { return []; },
                    detectBody: noop, detectDepth: noop, detectFace: noop, detectHand: noop, runOCR: noop,
                    requestAnimationFrame: function(cb) { return setTimeout(function() { cb(Date.now()); }, 16); },
                    cancelAnimationFrame: function(id) { clearTimeout(id); }
                };
            };
