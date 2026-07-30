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
