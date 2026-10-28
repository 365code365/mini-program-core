            // PC 端的指针锁定、手柄、光标（G3 延后项，官方：仅 PC 平台）。手机上照官方形状给，状态由宿主推：
            // - requestPointerLock / exitPointerLock → 宿主事件 {type:'pointerLock', lock}；宿主确认后 mr_game_dispatch_event('pointerLockChange', {locked})；
            // - getGamepads()：宿主经 mr_game_dispatch_event('gamepad', {index, id, connected, buttons:[{pressed,value}], axes:[…], timestamp}) 推的最新状态；
            //   connected 变化时派发 onGamepadConnected / onGamepadDisconnected({gamepad})；没有宿主推过就是空数组（手机上的常态）；
            // - setCursor(path, x, y) → 宿主事件 {type:'cursor', path, x, y}，返回 true（只有宿主能决定是否生效）。
            (function() {
                var locked = false, pads = [], on = { gamepadConnected: [], gamepadDisconnected: [] };
                function host(e) { __native_host_event(JSON.stringify(e)); }
                wx.requestPointerLock = function() { host({ type: 'pointerLock', lock: true }); };
                wx.exitPointerLock = function() { host({ type: 'pointerLock', lock: false }); };
                wx.isPointerLocked = function() { return locked; };
                wx.setCursor = function(path, x, y) { host({ type: 'cursor', path: String(path === undefined ? 'default' : path), x: Number(x) || 0, y: Number(y) || 0 }); return true; };
                wx.getGamepads = function() { return pads.filter(function(p) { return p && p.connected; }).map(function(p) { return JSON.parse(JSON.stringify(p)); }); };
                ['Connected', 'Disconnected'].forEach(function(k) {
                    var n = 'gamepad' + k;
                    wx['onGamepad' + k] = function(f) { if (typeof f === 'function') on[n].push(f); };
                    wx['offGamepad' + k] = function(f) { on[n] = f ? on[n].filter(function(x) { return x !== f; }) : []; };
                });
                var prev = globalThis.__gameHostEvent;
                globalThis.__gameHostEvent = function(name, json) {
                    var p = {}; try { p = JSON.parse(json || '{}') || {}; } catch (e) {}
                    if (name === 'pointerLockChange') { locked = !!p.locked; return 1; }
                    if (name === 'gamepad') {
                        var i = Number(p.index) || 0, was = !!(pads[i] && pads[i].connected);
                        pads[i] = { index: i, id: String(p.id || ''), connected: p.connected !== false, mapping: p.mapping || 'standard',
                            buttons: (p.buttons || []).map(function(b) { return { pressed: !!b.pressed, touched: !!(b.touched || b.pressed), value: Number(b.value) || 0 }; }),
                            axes: (p.axes || []).map(Number), timestamp: Number(p.timestamp) || 0 };
                        var now = pads[i].connected;
                        if (now !== was) { var ev = now ? 'gamepadConnected' : 'gamepadDisconnected'; on[ev].slice().forEach(function(f) { __gameCall(f, null, [{ gamepad: JSON.parse(JSON.stringify(pads[i])) }]); }); }
                        return 1;
                    }
                    return prev ? prev(name, json) : 0;
                };
            })();
