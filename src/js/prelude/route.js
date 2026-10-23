            // 相对路径（`../detail/detail?id=1`、`detail?id=1`）按**当前页面**所在目录解析（微信语义）；
            // 以 `/` 开头的是从小程序根算起。宿主只认根路径，所以在这里先解析好
            function __resolveRoute(url) {
                url = String(url == null ? '' : url);
                if (!url || url.charAt(0) === '/') return url;
                var page = (typeof __currentPage === 'object' && __currentPage) ? __currentPage : null;
                var base = page && (page.route || page.__route__) ? String(page.route || page.__route__) : '';
                var q = url.indexOf('?'), path = q >= 0 ? url.slice(0, q) : url, query = q >= 0 ? url.slice(q) : '';
                var parts = base.split('/'); parts.pop();
                path.split('/').forEach(function(seg) {
                    if (seg === '..') parts.pop();
                    else if (seg !== '.' && seg !== '') parts.push(seg);
                });
                return '/' + parts.join('/') + query;
            }
            // 导航 API
            wx.navigateTo = function(options) {
                __pendingNavigation = {
                    type: 'navigateTo',
                    url: __resolveRoute(options.url)
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
                // 逻辑层页面栈出栈，并给关掉的每一页派发 onUnload；宿主收到请求后按记下的层数
                // 退自己的栈（host::navigation::take_logic_popped），不再出栈、也不再派发。
                // 它复用上一页的实例，不重新加载页面 JS
                __popPage(delta, true);
                __native_print('[Navigate] navigateBack');
                options.success && options.success();
            };
            
            wx.switchTab = function(options) {
                __pendingNavigation = {
                    type: 'switchTab',
                    url: __resolveRoute(options.url)
                };
                __native_print('[Navigate] switchTab: ' + options.url);
                options.success && options.success();
            };
            
            // `redirectTo` 是**替换**当前页（栈深不变、退不回来），
            // `reLaunch` 是**清空整个栈**再开。宿主两条分支都实现了
            // （window.rs::process_navigation），这里以前却分别发成
            // 'navigateTo' / 'switchTab'，结果 redirectTo 变成入栈（还能退回本该关掉的页），
            // reLaunch 走 switchTab（目标不是 tab 页时直接失败、返回栈也没清）。
            wx.redirectTo = function(options) {
                __pendingNavigation = {
                    type: 'redirectTo',
                    url: __resolveRoute(options.url)
                };
                __native_print('[Navigate] redirectTo: ' + options.url);
                options.success && options.success();
            };
            
            wx.reLaunch = function(options) {
                __pendingNavigation = {
                    type: 'reLaunch',
                    url: __resolveRoute(options.url)
                };
                __native_print('[Navigate] reLaunch: ' + options.url);
                options.success && options.success();
            };

            // <navigator> 被点了（host::builtin_tap）：按 open-type 走对应的路由 API
            globalThis.__navigatorTap = function(openType, url, delta, mpJson) {
                switch (openType) {
                    case 'redirect': return wx.redirectTo({ url: url });
                    case 'switchTab': return wx.switchTab({ url: url });
                    case 'reLaunch': return wx.reLaunch({ url: url });
                    case 'navigateBack': return wx.navigateBack({ delta: delta || 1 });
                    case 'exit': return __navigatorExit(); // 退出小程序：发宿主事件 exitMiniProgram（route-ext.js）
                    case 'miniProgram': {
                        var mp = {};
                        try { mp = JSON.parse(mpJson || '{}'); } catch (e) {}
                        var extra = mp['extra-data'];
                        try { extra = extra ? JSON.parse(extra) : undefined; } catch (e) {}
                        return wx.navigateToMiniProgram({ appId: mp['app-id'], path: mp.path, extraData: extra,
                                                          envVersion: mp.version || 'release', shortLink: mp['short-link'] });
                    }
                    default: return wx.navigateTo({ url: url });
                }
            };
