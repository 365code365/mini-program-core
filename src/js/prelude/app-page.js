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
