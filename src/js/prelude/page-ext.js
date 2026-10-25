            // ───────── 页面 / 组件实例上补齐的方法与字段 ─────────
            // app-page.js 的 Page() 建好实例后调 __decoratePage，component.js 建实例时调 __decorateComponent。
            //
            // 页面：options（onLoad 的参数）、getPageId、getOpenerEventChannel、router / pageRouter、getTabBar、
            //       setUpdatePerformanceListener，以及 onLoad / onShow / onUnload 外面的那层（路由事件、提交路由）。
            // 组件：getPageId、router / pageRouter（相对**所在页面**解析）。
            var __pageIdSeq = 0;
            var __launched = false;
            // 返回拦截（page-container 弹着时返回先关它）：函数返回 true = 接住了
            var __backInterceptors = [];

            function __routerFor(owner) {
                function rel(o) {
                    o = o || {};
                    var copy = {};
                    for (var k in o) copy[k] = o[k];
                    var url = String(o.url || '');
                    if (url && url.charAt(0) !== '/') {
                        // 相对 owner 所在页面解析（不是当前栈顶）
                        var base = String(owner.route || '').split('/');
                        base.pop();
                        url.split('?')[0].split('/').forEach(function(seg) {
                            if (seg === '..') base.pop(); else if (seg !== '.' && seg !== '') base.push(seg);
                        });
                        copy.url = '/' + base.join('/') + (url.indexOf('?') >= 0 ? url.slice(url.indexOf('?')) : '');
                    }
                    return copy;
                }
                return {
                    navigateTo: function(o) { return wx.navigateTo(rel(o)); },
                    redirectTo: function(o) { return wx.redirectTo(rel(o)); },
                    switchTab: function(o) { return wx.switchTab(rel(o)); },
                    reLaunch: function(o) { return wx.reLaunch(rel(o)); },
                    navigateBack: function(o) { return wx.navigateBack(o); }
                };
            }

            function __decoratePage(page) {
                page.__pageId = ++__pageIdSeq;
                page.__route__ = page.route;
                page.options = {};
                page.getPageId = function() { return 'pageId:' + page.__pageId; };
                // navigateTo({events}) 打开的这一页：拿到同一个 EventChannel；别的方式打开的给一个空的（官方也是）
                var ch = (__routePending && __routePending.channel) || null;
                page.getOpenerEventChannel = function() { return ch || (ch = __makeEventChannel({})); };
                page.router = page.pageRouter = __routerFor(page);
                // 自定义 tabBar 的实例（宿主在 app.js 之后用 __mountCustomTabBar 建好；没有自定义 tabBar 时是 null，
                // 官方写法 `typeof this.getTabBar === 'function' && this.getTabBar()` 判空能接住）
                page.getTabBar = function() { return __customTabBarInstance || null; };
                page.setUpdatePerformanceListener = function(opt, cb) { page.__perfListener = typeof opt === 'function' ? opt : cb; };
                if (typeof __attachMediaQuery === 'function') __attachMediaQuery(page);
                if (typeof __attachAnimate === 'function') __attachAnimate(page);

                var onLoad = page.onLoad, onShow = page.onShow, onUnload = page.onUnload;
                // 宿主只在页面「有」onLoad / onShow / onUnload 时才调：这里总给，路由事件才发得出来
                page.onLoad = function(q) {
                    q = q || {};
                    page.options = q;
                    if (!__routePending) {
                        // 没有逻辑层发起的路由：启动后的第一页是 appLaunch，宿主自己开的页（深链）算 navigateTo
                        __routeBegin(__launched ? 'navigateTo' : 'appLaunch', '/' + page.route, {});
                    }
                    __launched = true;
                    var info = { path: page.route, query: q, openType: __routePending ? __routePending.openType : 'navigateTo' };
                    __fireRoute('BeforePageLoad', info);
                    var r;
                    if (typeof onLoad === 'function') r = onLoad.call(page, q);
                    __fireRoute('AfterPageLoad', info);
                    return r;
                };
                page.onShow = function() {
                    var r;
                    if (typeof onShow === 'function') r = onShow.apply(page, arguments);
                    // 新页面 / 返回到的这一页露出来了：挂着的路由在这里提交（success、onAppRoute）
                    if (__currentPage === page) __routeCommit(page);
                    return r;
                };
                page.onUnload = function() {
                    var info = { path: page.route, query: page.options };
                    __fireRoute('BeforePageUnload', info);
                    var r;
                    if (typeof onUnload === 'function') r = onUnload.apply(page, arguments);
                    __fireRoute('AfterPageUnload', info);
                    return r;
                };
            }

            function __decorateComponent(inst) {
                var page = function() { return __currentPage || {}; };
                inst.getPageId = function() { return 'pageId:' + (page().__pageId || 0); };
                var r = null;
                Object.defineProperty(inst, 'router', { configurable: true, get: function() { return r || (r = __routerFor({ route: page().route })); } });
                Object.defineProperty(inst, 'pageRouter', { configurable: true, get: function() { return __routerFor({ route: page().route }); } });
                if (typeof __attachMediaQuery === 'function') __attachMediaQuery(inst);
                if (typeof __attachAnimate === 'function') __attachAnimate(inst);
            }

            // 宿主发起的返回（返回键、侧滑）：逻辑层没挂路由时补一个 navigateBack，回到的页面 onShow 时提交
            (function() {
                var raw = __popPage;
                __popPage = function(delta, byApi) {
                    if (!byApi && !__routePending && __pageStack.length > 1) {
                        var to = __pageStack[__pageStack.length - 2];
                        __routeBegin('navigateBack', '/' + (to ? to.route : ''), {});
                    }
                    return raw(delta, byApi);
                };
            })();

            // ---------- 自定义 tabBar 的活实例 ----------
            // 从前 tabBar 只用另一个 JS 上下文里取的 data 快照画：`getTabBar()` 是 null，组件自己 setData
            // （角标、选中态）也改不到画面（轻聊消息角标一直停在 20）。现在宿主在 app.js 执行之后建一个实例：
            // 页面 `getTabBar().setData(...)` 与组件内部的 setData 都会让 `__tabBarVersion` 加一，
            // 宿主在取页面数据时比对版本号，变了就取 `__customTabBarData()` 重画 tabBar。
            var __customTabBarInstance = null;
            var __tabBarVersion = 0;
            function __mountCustomTabBar() {
                if (__customTabBarInstance) { return true; }
                var inst = __createComponentInstance('custom-tab-bar', {});
                if (!inst) { return false; }
                inst.__appLevel = true;
                var raw = inst.setData;
                inst.setData = function(d, cb) { raw.call(inst, d, cb); __tabBarVersion++; };
                __customTabBarInstance = inst;
                __tabBarVersion++;
                return true;
            }
            function __customTabBarData() {
                return JSON.stringify(__customTabBarInstance ? __customTabBarInstance.data : {});
            }
