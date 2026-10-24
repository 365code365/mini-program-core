            // 系统信息 API
            //
            // 字段要给全：`safeArea` / `safeAreaInsets` / `statusBarHeight` 这些是
            // 布局算式的输入（刘海屏留白、吸底栏高度），应用普遍直接点进去取值。
            // 少一个字段就是一个 `cannot read property 'bottom' of undefined`。
            //
            // 宿主（移动端 SDK / Flutter 插件）会在 app.js 之前写入 `__hostSystemInfo`：
            // 真实的屏幕尺寸、像素比、安全区、平台。没写时（桌面窗体、无头快照）
            // 就是下面这份 375×667 的默认值，与从前一致。
            wx.getSystemInfoSync = function() {
                var info = __defaultSystemInfo();
                if (typeof __hostSystemInfo === 'object' && __hostSystemInfo) {
                    // 深拷贝：页面拿到对象后改字段（常见于框架做适配），不能污染下一次调用
                    var host = JSON.parse(JSON.stringify(__hostSystemInfo));
                    for (var k in host) { info[k] = host[k]; }
                }
                return info;
            };
            function __defaultSystemInfo() {
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
            }
            // 窗口尺寸变化（转屏、分屏、折叠屏）：宿主调 mr_app_resize 后由引擎派发
            var __windowResizeListeners = [];
            wx.onWindowResize = function(cb) {
                if (typeof cb === 'function') __windowResizeListeners.push(cb);
            };
            wx.offWindowResize = function(cb) {
                __windowResizeListeners = cb
                    ? __windowResizeListeners.filter(function(f) { return f !== cb; })
                    : [];
            };
            function __dispatchWindowResize() {
                var i = wx.getSystemInfoSync();
                var res = { size: { windowWidth: i.windowWidth, windowHeight: i.windowHeight,
                                    screenWidth: i.screenWidth, screenHeight: i.screenHeight },
                            deviceOrientation: i.deviceOrientation };
                var page = (typeof __currentPage === 'object' && __currentPage) ? __currentPage : null;
                if (page && typeof page.onResize === 'function') {
                    try { page.onResize(res); } catch (e) { console.error(e); }
                }
                __windowResizeListeners.slice().forEach(function(cb) {
                    try { cb(res); } catch (e) { console.error(e); }
                });
            }
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
            // 宿主在 __hostSystemInfo 里给了 systemSetting / appAuthorizeSetting 就以它为准
            // （真实的蓝牙 / 定位开关与授权状态只有宿主知道），没给时是下面这份默认值
            function __hostOverride(key, base) {
                var h = (typeof __hostSystemInfo === 'object' && __hostSystemInfo) ? __hostSystemInfo[key] : null;
                if (h && typeof h === 'object') { for (var k in h) base[k] = h[k]; }
                return base;
            }
            wx.getSystemSetting = function() {
                return __hostOverride('systemSetting', { bluetoothEnabled: false, locationEnabled: false, wifiEnabled: true,
                         deviceOrientation: wx.getSystemInfoSync().deviceOrientation });
            };
            wx.getAppAuthorizeSetting = function() {
                return __hostOverride('appAuthorizeSetting', { albumAuthorized: 'authorized', cameraAuthorized: 'authorized',
                         locationAuthorized: 'authorized', microphoneAuthorized: 'authorized',
                         notificationAuthorized: 'authorized' });
            };
            
            wx.getSystemInfo = function(options) {
                options = options || {};
                var info = wx.getSystemInfoSync();
                options.success && options.success(info);
                options.complete && options.complete();
            };

            // 启动参数：框架（uni-app / Taro）在 onLaunch 里必读，缺了会整个
            // onLaunch 抛异常 —— 于是 Vue 的响应式层根本没起来，页面只剩静态骨架。
            // 外部 App / H5 / 别的小程序打开时，宿主在创建选项里给 launchOptions（engine_options.rs）：
            // path / query / scene（1069 App 打开、1065 URL Scheme、1037 小程序打开小程序）/ referrerInfo
            wx.getLaunchOptionsSync = function() {
                var h = (typeof __hostLaunchOptions === 'object' && __hostLaunchOptions) ? __hostLaunchOptions : {};
                var q = (h.query && typeof h.query === 'object') ? JSON.parse(JSON.stringify(h.query)) : {};
                var ref = (h.referrerInfo && typeof h.referrerInfo === 'object') ? JSON.parse(JSON.stringify(h.referrerInfo)) : {};
                return {
                    path: typeof h.path === 'string' ? h.path : (typeof __launchPath === 'string' ? __launchPath : ''),
                    scene: Number(h.scene) || 1001,
                    query: q,
                    shareTicket: '',
                    referrerInfo: ref,
                    forwardMaterials: [],
                    chatType: undefined,
                    apiCategory: 'default'
                };
            };
            // 「本次进入」的参数，字段与启动参数同构
            wx.getEnterOptionsSync = function() { return wx.getLaunchOptionsSync(); };
