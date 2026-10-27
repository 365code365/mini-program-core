            // ==================== 小游戏：键盘（game-doc/03、04 G7.2，GT-U2） ====================
            // 微信小游戏的键盘是宿主原生的：输入框 + 确认键在输入法上方（游戏画面不动），游戏只收事件。
            // - wx.showKeyboard({defaultValue, maxLength, multiple, confirmHold, confirmType}) → 宿主事件 {type:'showKeyboard', …}；
            // - wx.updateKeyboard({value}) → {type:'updateKeyboard', value}；wx.hideKeyboard() → {type:'hideKeyboard'}；
            // - 宿主推回（mr_game_dispatch_event）：keyboardInput / keyboardConfirm / keyboardComplete {value}、
            //   keyboardHeightChange {height}；
            // - confirmHold 为 false 时确认之后收起键盘（这里发 hideKeyboard，宿主收起后推 keyboardComplete）；
            // - 参数缺省：defaultValue ''、maxLength 100、multiple false、confirmHold false、confirmType 'done'（官方文档；待取证）。
            (function() {
                var ls = { keyboardInput: [], keyboardConfirm: [], keyboardComplete: [] };
                var state = null;
                function host(o) { __native_host_event(JSON.stringify(o)); }
                ['Input', 'Confirm', 'Complete'].forEach(function(k) {
                    var key = 'keyboard' + k, list = ls[key];
                    wx['onKeyboard' + k] = function(cb) { if (typeof cb === 'function' && list.indexOf(cb) < 0) list.push(cb); };
                    wx['offKeyboard' + k] = function(cb) {
                        var keep = typeof cb === 'function' ? list.filter(function(f) { return f !== cb; }) : [];
                        list.length = 0; Array.prototype.push.apply(list, keep);
                    };
                });
                var CONFIRM = ['done', 'next', 'search', 'go', 'send'];
                wx.showKeyboard = function(o) {
                    o = o || {};
                    var maxLength = o.maxLength === undefined ? 100 : Number(o.maxLength);
                    var cfg = {
                        type: 'showKeyboard',
                        defaultValue: o.defaultValue == null ? '' : String(o.defaultValue),
                        maxLength: isFinite(maxLength) ? maxLength : 100,
                        multiple: !!o.multiple,
                        confirmHold: !!o.confirmHold,
                        confirmType: CONFIRM.indexOf(o.confirmType) >= 0 ? o.confirmType : 'done'
                    };
                    state = { value: cfg.defaultValue, confirmHold: cfg.confirmHold };
                    host(cfg);
                    return __asyncOk(o, 'showKeyboard');
                };
                wx.updateKeyboard = function(o) {
                    o = o || {};
                    if (!state) return __asyncFail(o, 'updateKeyboard', 'keyboard is not shown');
                    state.value = o.value == null ? '' : String(o.value);
                    host({ type: 'updateKeyboard', value: state.value });
                    return __asyncOk(o, 'updateKeyboard');
                };
                wx.hideKeyboard = function(o) {
                    host({ type: 'hideKeyboard' });
                    return __asyncOk(o, 'hideKeyboard');
                };
                function fire(key, arg) {
                    var l = ls[key].slice();
                    for (var i = 0; i < l.length; i++) __gameCall(l[i], null, [arg]);
                }
                // 宿主事件的统一入口（原生 mr_game_dispatch_event 调）：键盘三件在这里，其余交给 host-capabilities 的 __hostDispatch
                globalThis.__gameHostEvent = function(name, json) {
                    var p = {};
                    try { p = JSON.parse(json || '{}') || {}; } catch (e) {}
                    if (name in ls) {
                        var value = p.value == null ? '' : String(p.value);
                        if (state) state.value = value;
                        fire(name, { value: value });
                        if (name === 'keyboardConfirm' && state && !state.confirmHold) host({ type: 'hideKeyboard' });
                        if (name === 'keyboardComplete') state = null;
                        return ls[name].length;
                    }
                    return typeof __hostDispatch === 'function' ? __hostDispatch(name, json) : 0;
                };
            })();
