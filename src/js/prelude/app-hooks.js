            // ==================== App 级监听型 API ====================
            //
            // `wx.onError` / `wx.onUnhandledRejection` / `wx.onPageNotFound` /
            // `wx.onThemeChange` 是真实的微信 API，与 `App({onError})` **并存**
            // （微信里两边都会收到）。必须真的实现，不能靠「未实现 API 探针」返回
            // undefined 兜着：框架的包装层自己就是个函数、能通过
            // `typeof uni.onError === 'function'` 的能力探测，进去之后执行的是
            // `wx[name].apply(wx, args)` —— 底下缺了就是
            // `TypeError: cannot read property 'apply' of undefined`，
            // **整个 app 脚本中断、一个页面都出不来**（tea-app 换新版编译产物后就是这样）。
            var __appHooks = {};
            function __makeAppHook(name) {
                wx['on' + name] = function (cb) {
                    if (typeof cb !== 'function') { return; }
                    (__appHooks['on' + name] = __appHooks['on' + name] || []).push(cb);
                };
                wx['off' + name] = function (cb) {
                    var list = __appHooks['on' + name];
                    if (!list) { return; }
                    if (cb === undefined) { __appHooks['on' + name] = []; return; }
                    var i = list.indexOf(cb);
                    if (i >= 0) { list.splice(i, 1); }
                };
            }
            __makeAppHook('Error');
            __makeAppHook('UnhandledRejection');
            __makeAppHook('PageNotFound');
            __makeAppHook('ThemeChange');
            __makeAppHook('AppShow');
            __makeAppHook('AppHide');
