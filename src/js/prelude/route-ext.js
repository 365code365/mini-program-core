            // ───────── 路由补齐：校验、回调时机、EventChannel、路由事件、返回询问、rewriteRoute、exit / restart、router ─────────
            //
            // 五个路由 API（route.js）只把请求写进 __pendingNavigation 交给宿主。这里在它们外面包一层，补上微信的语义：
            // - 先按 app.json 校验：页面不存在、navigateTo 到 tab 页、switchTab 到非 tab 页、页面栈满 10 层，立刻 fail；
            // - success **在新页面 onShow 之后**才回（从前同步回：opener 在 success 里 eventChannel.emit，新页面还没注册 on，消息就丢了）；
            // - navigateTo 的 `events` + 新页面 `this.getOpenerEventChannel()`：同一个 EventChannel，两边互发；
            // - wx.onBeforeAppRoute / onAppRoute / onAppRouteDone / onBeforePageLoad / onAfterPageLoad / onBeforePageUnload / onAfterPageUnload；
            // - wx.enableAlertBeforeUnload：wx.navigateBack、导航栏返回键、Android 返回键先弹询问（iOS 侧滑不问：页面已经滑出去了）。
            //   宿主发起的返回先调 __interceptBack()（page-container 弹着时返回也是先关它，见 page-ext.js 的 __backInterceptors）。

            // app.json 的页面表（包内文件，FileSystemManager 读得到）。读不到时一律放行（无头单测没有 app.json）
            var __appJsonCache = null;
            function __appJson() {
                if (__appJsonCache) return __appJsonCache;
                var j = null;
                try {
                    var r = JSON.parse(__native_fs('readFile', JSON.stringify({ filePath: 'app.json', encoding: 'utf8' })));
                    if (r && r.ok) j = JSON.parse(r.data);
                } catch (e) {}
                var pages = {}, tabs = {};
                if (j) {
                    (j.pages || []).forEach(function(p) { pages[p] = true; });
                    (j.subpackages || j.subPackages || []).forEach(function(s) {
                        (s.pages || []).forEach(function(p) { pages[(s.root || '').replace(/\/$/, '') + '/' + p] = true; });
                    });
                    ((j.tabBar && j.tabBar.list) || []).forEach(function(t) { tabs[t.pagePath] = true; });
                }
                __appJsonCache = { known: !!j, pages: pages, tabs: tabs, entry: (j && (j.entryPagePath || (j.pages || [])[0])) || '' };
                return __appJsonCache;
            }
            function __routePath(url) {
                var u = String(url || ''), q = u.indexOf('?');
                return { path: (q >= 0 ? u.slice(0, q) : u).replace(/^\/+/, ''), query: __parseQuery(q >= 0 ? u.slice(q + 1) : '') };
            }
            function __parseQuery(s) {
                var o = {};
                String(s || '').split('&').forEach(function(kv) {
                    if (!kv) return;
                    var i = kv.indexOf('=');
                    var k = i >= 0 ? kv.slice(0, i) : kv, v = i >= 0 ? kv.slice(i + 1) : '';
                    try { o[decodeURIComponent(k)] = decodeURIComponent(v); } catch (e) { o[k] = v; }
                });
                return o;
            }

            // ---------- 路由事件 ----------
            var __routeHooks = {};
            ['BeforeAppRoute', 'AppRoute', 'AppRouteDone', 'BeforePageLoad', 'AfterPageLoad', 'BeforePageUnload', 'AfterPageUnload']
                .forEach(function(N) {
                    __routeHooks[N] = [];
                    wx['on' + N] = function(cb) { if (typeof cb === 'function') __routeHooks[N].push(cb); };
                    wx['off' + N] = function(cb) { __routeHooks[N] = cb ? __routeHooks[N].filter(function(f) { return f !== cb; }) : []; };
                });
            var __routeEventSeq = 0;
            function __fireRoute(N, res) {
                __routeHooks[N].slice().forEach(function(cb) { try { cb(res); } catch (e) { console.error(e); } });
            }

            // 一次路由：api 发起时建，新页面 onShow 时提交（回 success、发 onAppRoute / onAppRouteDone）
            var __routePending = null;
            function __routeBegin(openType, url, o, extra) {
                var r = __routePath(url);
                __routePending = { openType: openType, path: r.path, query: r.query, o: o || {}, routeEventId: 'route_' + (++__routeEventSeq) };
                for (var k in (extra || {})) __routePending[k] = extra[k];
                __fireRoute('BeforeAppRoute', { path: r.path, query: r.query, openType: openType, routeEventId: __routePending.routeEventId });
                return __routePending;
            }
            // 新页面 / 回到的页面 onShow：提交挂着的路由
            function __routeCommit(page) {
                var p = __routePending;
                if (!p) return;
                __routePending = null;
                var res = { path: page.route, query: page.options || {}, openType: p.openType, routeEventId: p.routeEventId,
                            webviewId: page.__pageId };
                __fireRoute('AppRoute', res);
                var ok = { errMsg: p.openType + ':ok' };
                if (p.channel) ok.eventChannel = p.channel;
                var o = p.o;
                o.success && o.success(ok);
                o.resolve && o.resolve(ok);
                o.complete && o.complete(ok);
                setTimeout(function() { __fireRoute('AppRouteDone', res); }, 0);
            }
            function __routeFail(o, name, msg) {
                var bad = { errMsg: name + ':fail ' + msg };
                setTimeout(function() { o.fail && o.fail(bad); o.reject && o.reject(bad); o.complete && o.complete(bad); }, 0);
            }
            function __promiseOf(o) {
                if (o.success || o.fail || o.complete) return undefined;
                return new Promise(function(resolve, reject) { o.resolve = resolve; o.reject = reject; });
            }

            // ---------- EventChannel ----------
            function __makeEventChannel(events) {
                var ls = {};
                var ch = {
                    on: function(name, cb) { if (typeof cb === 'function') (ls[name] = ls[name] || []).push({ cb: cb, once: false }); },
                    once: function(name, cb) { if (typeof cb === 'function') (ls[name] = ls[name] || []).push({ cb: cb, once: true }); },
                    off: function(name, cb) { if (ls[name]) ls[name] = cb ? ls[name].filter(function(l) { return l.cb !== cb; }) : []; },
                    emit: function(name) {
                        var args = [].slice.call(arguments, 1);
                        (ls[name] || []).slice().forEach(function(l) {
                            if (l.once) ch.off(name, l.cb);
                            try { l.cb.apply(null, args); } catch (e) { console.error(e); }
                        });
                    }
                };
                for (var k in (events || {})) if (typeof events[k] === 'function') ch.on(k, events[k]);
                return ch;
            }

            // ---------- 五个路由 API（包住 route.js 那份，请求格式不变） ----------
            var __rawNav = { navigateTo: wx.navigateTo, redirectTo: wx.redirectTo, switchTab: wx.switchTab, reLaunch: wx.reLaunch, navigateBack: wx.navigateBack };
            function __checkTarget(name, path) {
                var a = __appJson();
                if (!a.known) return '';
                if (!a.pages[path]) return 'page "' + path + '" is not found';
                if ((name === 'navigateTo' || name === 'redirectTo') && a.tabs[path]) return 'can not ' + name + ' a tabbar page';
                if (name === 'switchTab' && !a.tabs[path]) return 'can not switch to no-tabBar page';
                if (name === 'navigateTo' && __pageStack.length >= 10) return 'webview count limit exceed';
                return '';
            }
            ['navigateTo', 'redirectTo', 'switchTab', 'reLaunch'].forEach(function(name) {
                wx[name] = function(o) {
                    o = o || {};
                    var promise = __promiseOf(o);
                    var url = __resolveRoute(o.url);
                    var target = __routePath(url).path;
                    var err = __checkTarget(name, target);
                    if (err) { __routeFail(o, name, err); return promise; }
                    var r = __routeBegin(name, url, o, name === 'navigateTo' ? { channel: __makeEventChannel(o.events) } : null);
                    // onBeforeAppRoute 里调了 wx.rewriteRoute：换成改写后的地址
                    if (r.rewrite) url = r.rewrite;
                    __rawNav[name]({ url: url });
                    return promise;
                };
            });
            wx.navigateBack = function(o) {
                o = o || {};
                var promise = __promiseOf(o);
                if (__pageStack.length <= 1) { __routeFail(o, 'navigateBack', 'cannot navigate back at first page.'); return promise; }
                var go = function() {
                    var delta = Math.max(1, Number(o.delta) || 1);
                    var to = __pageStack[Math.max(0, __pageStack.length - 1 - delta)];
                    __routeBegin('navigateBack', '/' + (to ? to.route : ''), o);
                    __rawNav.navigateBack({ delta: delta });
                };
                // page-container 弹着：navigateBack 也只是关掉它（官方：返回操作包括调用 navigateBack）
                if (!o.__skipAlert && typeof __backInterceptors !== 'undefined') {
                    for (var i = __backInterceptors.length - 1; i >= 0; i--) {
                        if (__backInterceptors[i]()) { __routeFail(o, 'navigateBack', 'intercepted by page-container'); return promise; }
                    }
                }
                var msg = __currentPage && __currentPage.__alertBeforeUnload;
                if (msg && !o.__skipAlert) { __askBeforeUnload(msg, go, function() { __routeFail(o, 'navigateBack', 'cancel'); }); return promise; }
                go();
                return promise;
            };

            // rewriteRoute：只能在 onBeforeAppRoute 里调（官方限制），改写正要打开的那个地址
            wx.rewriteRoute = function(o) {
                o = o || {};
                if (!__routePending) return __asyncFail(o, 'rewriteRoute', 'can only be called in onBeforeAppRoute');
                var url = __resolveRoute(o.url);
                var r = __routePath(url);
                if (o.preserveQuery) {
                    for (var k in __routePending.query) if (!(k in r.query)) r.query[k] = __routePending.query[k];
                    var qs = Object.keys(r.query).map(function(k) { return encodeURIComponent(k) + '=' + encodeURIComponent(r.query[k]); }).join('&');
                    url = '/' + r.path + (qs ? '?' + qs : '');
                }
                __routePending.rewrite = url;
                __routePending.path = r.path;
                return __asyncOk(o, 'rewriteRoute');
            };

            // ---------- 返回询问 ----------
            function __askBeforeUnload(msg, go, stay) {
                wx.showModal({ title: '', content: String(msg), success: function(r) { if (r.confirm) go(); else stay && stay(); } });
            }
            wx.enableAlertBeforeUnload = function(o) {
                o = o || {};
                if (!__currentPage) return __asyncFail(o, 'enableAlertBeforeUnload', 'no page');
                __currentPage.__alertBeforeUnload = String(o.message || '');
                return __asyncOk(o, 'enableAlertBeforeUnload');
            };
            wx.disableAlertBeforeUnload = function(o) {
                if (__currentPage) __currentPage.__alertBeforeUnload = '';
                return __asyncOk(o, 'disableAlertBeforeUnload');
            };
            // 宿主发起的返回（导航栏返回键、Android 返回键、桌面 Esc）先问这里：返回 true = 这次返回被接住了，宿主别退栈
            globalThis.__interceptBack = function() {
                if (typeof __backInterceptors !== 'undefined') {
                    for (var i = __backInterceptors.length - 1; i >= 0; i--) {
                        try { if (__backInterceptors[i]()) return true; } catch (e) { console.error(e); }
                    }
                }
                var msg = __currentPage && __currentPage.__alertBeforeUnload;
                if (msg && __pageStack.length > 1) {
                    __askBeforeUnload(msg, function() { wx.navigateBack({ __skipAlert: true }); });
                    return true;
                }
                return false;
            };

            // ---------- 退出 / 重启 ----------
            wx.exitMiniProgram = function(o) {
                __emitHostEvent({ type: 'exitMiniProgram' });
                return __asyncOk(o, 'exitMiniProgram');
            };
            // 引擎不重跑 app.js（App 实例与 globalData 保留）：清栈回入口页（或给定页面），App 再走一次 onShow
            wx.restartMiniProgram = function(o) {
                o = o || {};
                var path = o.path || ('/' + __appJson().entry);
                wx.reLaunch({ url: path, success: function() { __dispatchApp('onShow'); __asyncOk(o, 'restartMiniProgram'); },
                              fail: function(e) { __asyncFail(o, 'restartMiniProgram', String(e.errMsg || '').replace(/^[^:]*:fail\s*/, '')); } });
            };
            globalThis.__navigatorExit = function() { wx.exitMiniProgram({}); };
