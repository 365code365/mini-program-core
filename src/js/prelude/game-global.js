            // ==================== 小游戏：全局对象与错误通道（game-doc/02 §3.2、§3.3） ====================
            //
            // 只装进小游戏（js/api.rs 的 GAME_EXTRA），排在通用段之后、api-probe 之前。

            // `GameGlobal` 就是全局对象本身（适配层 weapp-adapter 往它上面挂 window / document）
            globalThis.GameGlobal = globalThis;

            // console.js 的每个方法都往 `__console_buffer` 里 push，而**没有任何代码读取或清空它**
            // （game-doc/11 B-11）：页面生命周期短时看不出来，游戏每帧打几行日志，一小时就是几十万条、
            // 内存只涨不降。小游戏把它换成丢弃式的容器；小程序侧要改是行为变化，单独征求同意（D-10）。
            // console.js 在调用时才按名字找这个全局，所以换掉变量就够了，不用改 console 的实现
            __console_buffer = { push: function() {} };

            // ---------- wx.onError / wx.onUnhandledRejection（小游戏版：没有 App() 对象） ----------
            // 回调参数：onError 收 { message, stack }，onUnhandledRejection 收 { reason, promise }（官方形状）。
            // 由原生在捕获到异常 / 拒绝时调 __gameDispatchError / __gameDispatchRejection
            var __gameErrorListeners = [];
            var __gameRejectionListeners = [];
            function __gameOnOff(list) {
                return [
                    function(cb) { if (typeof cb === 'function') list.push(cb); },
                    function(cb) {
                        if (typeof cb !== 'function') { list.length = 0; return; }
                        var i = list.indexOf(cb);
                        if (i >= 0) list.splice(i, 1);
                    }
                ];
            }
            (function() {
                var e = __gameOnOff(__gameErrorListeners), r = __gameOnOff(__gameRejectionListeners);
                wx.onError = e[0]; wx.offError = e[1];
                wx.onUnhandledRejection = r[0]; wx.offUnhandledRejection = r[1];
            })();
            // 返回收到它的监听者个数（宿主据此决定要不要另外弹错误提示）。监听者自己抛错只打日志，
            // 不再递归派发（否则一个写错的 onError 会死循环）
            globalThis.__gameDispatchError = function(message, stack) {
                var res = { message: String(message), stack: String(stack || '') };
                var list = __gameErrorListeners.slice();
                for (var i = 0; i < list.length; i++) {
                    try { list[i](res); } catch (e) { __native_print('[ERROR] wx.onError 监听者抛错：' + (e && e.message)); }
                }
                return list.length;
            };
            globalThis.__gameDispatchRejection = function(reason, promise) {
                var list = __gameRejectionListeners.slice();
                for (var i = 0; i < list.length; i++) {
                    try { list[i]({ reason: reason, promise: promise }); } catch (e) { __native_print('[ERROR] wx.onUnhandledRejection 监听者抛错：' + (e && e.message)); }
                }
                return list.length;
            };
            // 定时器回调里的异常：timer.js 只 console.error 一行就吞掉了。小游戏没有 App() 可以兜，
            // 微信里它们会进 wx.onError —— 所以这里换成经 __gameCall 的版本（只改小游戏，小程序那份不动）。
            // 原生按名字调 __trigger_timer(id)，换掉全局函数就生效；其余逻辑与 timer.js 逐行相同
            __trigger_timer = function(id) {
                var timer = __timers[id];
                if (timer && typeof timer.callback === 'function') {
                    __gameCall(timer.callback, null, timer.args || []);
                    if (timer.type === 'timeout') { delete __timers[id]; }
                }
            };

            // wx.triggerGC：只做标记，由宿主在这段 JS 跑完之后执行 GC（在 JS 调用栈里直接回收不安全）
            var __gameGcRequested = false;
            wx.triggerGC = function() { __gameGcRequested = true; };

            // 统一的「执行一段游戏回调」入口：异常不外抛（一个回调出错不能打断同批其余回调，GT-S3），
            // 转给 wx.onError 并打日志。返回 true = 正常结束
            globalThis.__gameCall = function(fn, self, args) {
                try { fn.apply(self, args || []); return true; }
                catch (e) {
                    var msg = e && e.message !== undefined ? e.message : String(e);
                    var stack = e && e.stack ? String(e.stack) : '';
                    __native_print('[ERROR] ' + (e && e.name ? e.name + ': ' : '') + msg + (stack ? '\n' + stack : ''));
                    __gameDispatchError(msg, stack);
                    return false;
                }
            };
