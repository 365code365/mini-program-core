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
            // setData 序号：每次 setData 记下「这个实例的哪些根字段在第几次被设过」。
            // 宿主用它判断受控属性有没有被**设置**（哪怕值没变）—— scroll-view 的
            // `refresher-triggered` 一直是 false、刷新完再 setData 一次 false，也要把指示器收起来
            var __setDataSeq = 0;
            function __applySetData(instance, newData, callback) {
                if (newData) {
                    __setDataSeq++;
                    var seen = instance.__setKeys || (instance.__setKeys = {});
                    for (var key in newData) {
                        if (!newData.hasOwnProperty(key)) { continue; }
                        seen[String(key).split(/[.\[]/)[0]] = __setDataSeq;
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

            /// 序号 `seq` 之后，`ownerKey`（空串 = 当前页面，否则组件实例键）有没有 setData 过 `keys` 里任一字段
            function __setSince(ownerKey, keys, seq) {
                var inst = ownerKey ? (typeof __compState === 'function' ? __compState().map[ownerKey] : null) : __currentPage;
                var seen = inst && inst.__setKeys;
                if (!seen) { return 0; }
                for (var i = 0; i < keys.length; i++) {
                    if ((seen[keys[i]] || 0) > seq) { return 1; }
                }
                return 0;
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
                    
                    // 官方 API：在页面自己模板里的组件实例中按 `#id` / `.cls` / 标签找（component-tree.js）
                    selectComponent: function(sel) { return __selectComponentsFrom(__compStateOf(this), '', sel, false); },
                    selectAllComponents: function(sel) { return __selectComponentsFrom(__compStateOf(this), '', sel, true); },
                    // 官方实例方法：等价于 wx.createSelectorQuery().in(this) / wx.createIntersectionObserver(this, opt)（query.js）
                    createSelectorQuery: function() { return wx.createSelectorQuery().in(this); },
                    createIntersectionObserver: function(opt) { return wx.createIntersectionObserver(this, opt); }
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
                
                // 路由事件、EventChannel、options、pageRouter、页面级的新方法（route-ext.js / page-ext.js）
                if (typeof __decoratePage === 'function') __decoratePage(page);

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
                __logicPopped = 0;
                return true;
            }
            // wx.navigateBack 已经替宿主出过、宿主还没认领的层数
            var __logicPopped = 0;
            // 宿主处理「返回」时调：取走上面那个层数并清零（0 = 这次返回是宿主自己发起的）
            function __takeLogicPopped() {
                var n = __logicPopped;
                __logicPopped = 0;
                return n;
            }
            // 页面出栈（供 navigateBack 使用）。`byApi` 为真表示是 wx.navigateBack 发起的：
            // 给关掉的每一页派发 onUnload，并记下替宿主出了几层（__takeLogicPopped 取走）。
            // 宿主据此只退自己的栈、不再出逻辑层的栈、不再派发 onUnload —— 从前两边各出一次，
            // 回到的页面拿着上上一页的 data 渲染；onUnload 也派给了出栈之后的 __currentPage，
            // 那是要回去的那一页。宿主发起的返回（返回键、侧滑）由宿主先派发、再 __popPage(1)。
            function __popPage(delta, byApi) {
                delta = delta || 1;
                for (var i = 0; i < delta && __pageStack.length > 1; i++) {
                    var gone = __pageStack.pop();
                    if (!byApi) continue;
                    __logicPopped++;
                    if (gone && typeof gone.onUnload === 'function') {
                        try { gone.onUnload(); } catch (e) { __native_print('[Page.onUnload] ' + e.message); }
                    }
                }
                __currentPage = __pageStack[__pageStack.length - 1] || null;
                return __currentPage;
            }
            
            // 切走的 tab 页实例（路由 → 实例）。微信里 tab 页建一次就一直在：切回来不再 onLoad，
            // 只有 onHide / onShow，页面里的数据原样（host::page_keep 留着它的视图）
            var __tabPages = {};
            // 宿主的 switchTab：从栈顶往下，非 tab 页 onUnload（含组件 detached），tab 页留进
            // __tabPages（它在栈顶时先 onHide；在栈底被盖着的，盖住它时已经 onHide 过）。
            // 目标 tab 页留过实例：放回栈底、onShow，返回 true —— 宿主不再加载它的 js；
            // 没留过返回 false，宿主照常加载（Page() 会入栈）
            function __switchTabTo(route, tabRoutes) {
                var top = __pageStack[__pageStack.length - 1];
                for (var i = __pageStack.length - 1; i >= 0; i--) {
                    var p = __pageStack[i];
                    // __dispatchPage 按 __currentPage 派发（组件的 pageLifetimes 也按它筛）
                    __currentPage = p;
                    if (tabRoutes.indexOf(p.route) >= 0) {
                        if (p === top) __dispatchPage('onHide');
                        __tabPages[p.route] = p;
                    } else {
                        __dispatchPage('onUnload');
                    }
                }
                __pageStack = [];
                __currentPage = null;
                __logicPopped = 0;
                var kept = __tabPages[route];
                if (!kept) return false;
                delete __tabPages[route];
                __pageStack.push(kept);
                __currentPage = kept;
                __dispatchPage('onShow');
                return true;
            }
            // 宿主没有这个 tab 页的视图（只能重新加载）：逻辑层也别留它的实例
            function __dropTab(route) {
                if (__tabPages[route]) {
                    var p = __tabPages[route];
                    delete __tabPages[route];
                    var cur = __currentPage;
                    __currentPage = p;
                    __dispatchPage('onUnload');
                    __currentPage = cur;
                }
                __pageStack = [];
                __currentPage = null;
            }
            // 宿主的 reLaunch：栈上每一页（从顶往下）与留着的 tab 页全部 onUnload
            function __reLaunchAll() {
                for (var i = __pageStack.length - 1; i >= 0; i--) {
                    __currentPage = __pageStack[i];
                    __dispatchPage('onUnload');
                }
                for (var r in __tabPages) {
                    if (!__tabPages.hasOwnProperty(r)) continue;
                    __currentPage = __tabPages[r];
                    __dispatchPage('onUnload');
                }
                __tabPages = {};
                __pageStack = [];
                __currentPage = null;
                __logicPopped = 0;
                return true;
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
