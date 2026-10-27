            // 开放数据域里的 wx（G9.7）：sharedCanvas、onMessage、关系链与托管数据（交宿主）、系统信息。
            // 关系链 API 由宿主在 hostApis 里点名 `openData.<名字>` 接管（mr_game_host_reply 回结果）；没点名的一律 fail。
            (function() {
                var shared = null, listeners = [];
                wx.getSharedCanvas = function() {
                    if (!shared) { var i = __od_shared(); shared = new __GameCanvas([i[0], i[1], i[2], 0]); Object.defineProperty(shared, '__shared', { value: true }); }
                    return shared;
                };
                wx.onMessage = function(f) { if (typeof f === 'function' && listeners.indexOf(f) < 0) listeners.push(f); };
                wx.offMessage = function(f) { listeners = f ? listeners.filter(function(x) { return x !== f; }) : []; };
                globalThis.__odMessage = function(json) {
                    var m = JSON.parse(json);
                    listeners.slice().forEach(function(f) { __gameCall(f, null, [m]); });
                };
                var sys = JSON.parse(__od_sysinfo());
                wx.getSystemInfoSync = function() { var o = {}; for (var k in sys) o[k] = sys[k]; return o; };
                wx.getSystemInfo = function(o) { return __asyncOk(o, 'getSystemInfo', wx.getSystemInfoSync()); };
                var pend = {};
                globalThis.__odReply = function(id, ok, json) {
                    var p = pend[id]; if (!p) return; delete pend[id];
                    var res = {}; try { res = JSON.parse(json || '{}') || {}; } catch (e) {}
                    if (ok) __asyncOk(p.o, p.name, res); else __asyncFail(p.o, p.name, res.errMsg || 'host error');
                };
                ['getFriendCloudStorage', 'getGroupCloudStorage', 'getUserCloudStorage', 'getUserCloudStorageKeys', 'setUserCloudStorage',
                    'removeUserCloudStorage', 'getUserInfo', 'getPotentialFriendList', 'getUserInteractiveStorage', 'getGroupInfo',
                    'getGroupMembersGameData', 'modifyFriendInteractiveStorage', 'shareMessageToFriend', 'setMessageToFriendQuery'].forEach(function(name) {
                    wx[name] = function(o) {
                        o = o || {};
                        var args = {}; for (var k in o) if (typeof o[k] !== 'function') args[k] = o[k];
                        var id = __od_host(name, JSON.stringify(args));
                        if (id < 0) return __asyncFail(o, name, '宿主没有提供开放数据域的 ' + name);
                        var p = { o: o, name: name };
                        pend[id] = p;
                        if (!o.success && !o.fail && !o.complete) return new Promise(function(res, rej) { p.o = { success: res, fail: rej }; });
                    };
                });
            })();
