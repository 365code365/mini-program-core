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
