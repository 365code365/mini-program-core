            // ==================== 小游戏：窗口尺寸与方向（game-doc/04 G6.3，GT-O；原生在 game/window.rs） ====================
            // - wx.onWindowResize 换成小游戏的形状：回调参数 {windowWidth, windowHeight}（小程序那份是 {size:{…}, deviceOrientation}）；
            // - wx.onDeviceOrientationChange({value})：value ∈ portrait / landscape / landscapeReverse；
            // - wx.setDeviceOrientation({value: 'portrait' | 'landscape'})：交给宿主转（宿主事件 deviceOrientation），
            //   转完宿主调 mr_game_resize，上面两个事件随之派发。非法值 fail（文案待取证）。
            (function() {
                function listeners() {
                    var list = [];
                    return {
                        on: function(cb) { if (typeof cb === 'function' && list.indexOf(cb) < 0) list.push(cb); },
                        off: function(cb) {
                            var keep = typeof cb === 'function' ? list.filter(function(f) { return f !== cb; }) : [];
                            list.length = 0; Array.prototype.push.apply(list, keep);
                        },
                        fire: function(arg) {
                            var l = list.slice();
                            for (var i = 0; i < l.length; i++) __gameCall(l[i], null, [JSON.parse(JSON.stringify(arg))]);
                        }
                    };
                }
                var resize = listeners(), orient = listeners();
                wx.onWindowResize = resize.on; wx.offWindowResize = resize.off;
                wx.onDeviceOrientationChange = orient.on; wx.offDeviceOrientationChange = orient.off;
                globalThis.__gameWindowResize = function(res) { resize.fire(res); return true; };
                globalThis.__gameOrientationChange = function(res) { orient.fire(res); return true; };
                // getDeviceBenchmarkInfo（G6.6）：benchmarkLevel / modelLevel 由宿主按机器估算后放进 systemInfo（Android 示例按
                // 核数与内存，MiniGameView）；宿主没给 modelLevel 时按 benchmarkLevel 推：≥30 高档 1、≥15 中档 2、≥1 低档 3、其余未知 0。
                // 官方的跑分口径是微信自己的，对不上（记在验收文档）
                wx.getDeviceBenchmarkInfo = function(o) {
                    var i = wx.getSystemInfoSync(), lv = Number(i.benchmarkLevel);
                    if (!(lv === lv)) lv = -1;
                    var ml = typeof i.modelLevel === 'number' ? i.modelLevel : lv >= 30 ? 1 : lv >= 15 ? 2 : lv >= 1 ? 3 : 0;
                    return __asyncOk(o, 'getDeviceBenchmarkInfo', { benchmarkLevel: lv, modelLevel: ml });
                };
                wx.setDeviceOrientation = function(o) {
                    o = o || {};
                    if (o.value !== 'portrait' && o.value !== 'landscape') {
                        var bad = { errMsg: 'setDeviceOrientation:fail invalid value ' + o.value };
                        setTimeout(function() {
                            if (typeof o.fail === 'function') __gameCall(o.fail, null, [bad]);
                            if (typeof o.complete === 'function') __gameCall(o.complete, null, [bad]);
                        }, 0);
                        return;
                    }
                    __native_host_event(JSON.stringify({ type: 'deviceOrientation', value: o.value }));
                    return __asyncOk(o, 'setDeviceOrientation');
                };
            })();
