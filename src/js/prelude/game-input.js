            // ==================== 小游戏：触摸与 PC 键鼠（game-doc/02 §7、03） ====================
            //
            // 原生（game/touch.rs、game/pc_input.rs）在每帧 rAF 之前把宿主事件整理好，逐个调 __gameInput(kind, json)。
            // 这里只管监听表与派发：同一事件按注册顺序调用，一个回调抛错不打断其余（__gameCall，GT-S3）；
            // off 不带参数清空该事件的全部回调（与官方 off* 的约定一致）。
            // 键盘走已有的 wx.onKeyDown / onKeyUp（host-capabilities.js，__hostDispatch），这里不重复定义。
            (function() {
                var names = { TouchStart: 'touchstart', TouchMove: 'touchmove', TouchEnd: 'touchend', TouchCancel: 'touchcancel',
                    MouseDown: 'mousedown', MouseUp: 'mouseup', MouseMove: 'mousemove', Wheel: 'wheel' };
                var listeners = {};
                Object.keys(names).forEach(function(api) {
                    var k = names[api];
                    listeners[k] = [];
                    wx['on' + api] = function(cb) {
                        if (typeof cb !== 'function') return;
                        if (listeners[k].indexOf(cb) < 0) listeners[k].push(cb);
                    };
                    wx['off' + api] = function(cb) {
                        listeners[k] = typeof cb === 'function' ? listeners[k].filter(function(f) { return f !== cb; }) : [];
                    };
                });
                globalThis.__gameInput = function(kind, json) {
                    var list = listeners[kind];
                    if (!list || !list.length) return 0;
                    var ev = JSON.parse(json);
                    // 快照：回调里 off 自己不影响这一轮
                    list = list.slice();
                    for (var i = 0; i < list.length; i++) __gameCall(list[i], null, [ev]);
                    return list.length;
                };
            })();
