            // ───────── WebSocket（官方 api/network/websocket）─────────
            // wx.connectSocket 返回 SocketTask；连接、收发在原生线程里（src/websocket.rs），
            // 事件由宿主每帧取回后调 __resolveSocket(id, type, json)。
            // 旧式全局 API（wx.onSocketOpen / sendSocketMessage / closeSocket …）作用于**最近一次**创建的连接。
            // 二进制帧不经 base64：发送直接把 ArrayBuffer 交给原生（__native_socket_send_bin），收到的帧在原生排队，
            // 事件里只有 bin:true，这里按 id 取 ArrayBuffer（src/bin_stash.rs）。
            var __sockSeq = 0, __sockets = {}, __sockLatest = 0;
            var __sockGlobal = { open: [], message: [], error: [], close: [] };
            function __sockCall(o, name, res) {
                o = o || {};
                res.errMsg = res.errMsg || (name + ':ok');
                var ok = /:ok/.test(res.errMsg);
                if (ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
                o.complete && o.complete(res);
            }
            function __makeSocketTask(id) {
                var ls = { open: [], message: [], error: [], close: [] };
                var task = {
                    readyState: 0, // CONNECTING
                    CONNECTING: 0, OPEN: 1, CLOSING: 2, CLOSED: 3,
                    send: function(o) {
                        o = o || {};
                        var d = o.data, err;
                        if (typeof d === 'string') err = __native_socket_send(String(id), 't', d);
                        else if (typeof __native_socket_send_bin === 'function') err = __native_socket_send_bin(id, __toBytes(d));
                        else err = __native_socket_send(String(id), 'b', __b64encode(d));
                        __sockCall(o, 'sendSocketMessage', err ? { errMsg: err } : {});
                    },
                    close: function(o) {
                        o = o || {};
                        task.readyState = 2;
                        var err = __native_socket_close(String(id), String(o.code === undefined ? 1000 : o.code), String(o.reason || ''));
                        __sockCall(o, 'closeSocket', err ? { errMsg: err } : {});
                    }
                };
                ['Open', 'Message', 'Error', 'Close'].forEach(function(N) {
                    var k = N.toLowerCase();
                    task['on' + N] = function(cb) { if (typeof cb === 'function') ls[k].push(cb); };
                    task['off' + N] = function(cb) { ls[k] = cb ? ls[k].filter(function(f) { return f !== cb; }) : []; };
                });
                task.__ls = ls;
                return task;
            }
            wx.connectSocket = function(o) {
                o = o || {};
                var id = ++__sockSeq;
                var task = __makeSocketTask(id);
                __sockets[id] = task;
                __sockLatest = id;
                var err = __native_socket_connect(String(id), String(o.url || ''), JSON.stringify(o.header || {}),
                                                  JSON.stringify(o.protocols || []), String(o.timeout || 0));
                if (err) {
                    delete __sockets[id];
                    task.readyState = 3;
                    setTimeout(function() { __sockCall(o, 'connectSocket', { errMsg: err }); }, 0);
                } else {
                    // success 只表示「接口调用成功」，连上与否看 onOpen / onError（微信语义）
                    setTimeout(function() { __sockCall(o, 'connectSocket', {}); }, 0);
                }
                return task;
            };
            globalThis.__resolveSocket = function(id, type, json) {
                var task = __sockets[id];
                if (!task) return;
                var res = {};
                try { res = JSON.parse(json || '{}'); } catch (e) {}
                if (type === 'open') task.readyState = 1;
                if (type === 'message' && res.bin) { res = { data: __native_bin_take(1, id) || new ArrayBuffer(0) }; }
                if (type === 'close') { task.readyState = 3; delete __sockets[id]; }
                var fire = function(list) { list.slice().forEach(function(cb) { try { cb(res); } catch (e) { console.error(e); } }); };
                fire(task.__ls[type] || []);
                if (id === __sockLatest) fire(__sockGlobal[type] || []);
            };
            ['Open', 'Message', 'Error', 'Close'].forEach(function(N) {
                var k = N.toLowerCase();
                wx['onSocket' + N] = function(cb) { if (typeof cb === 'function') __sockGlobal[k].push(cb); };
                wx['offSocket' + N] = function(cb) { __sockGlobal[k] = cb ? __sockGlobal[k].filter(function(f) { return f !== cb; }) : []; };
            });
            wx.sendSocketMessage = function(o) {
                var t = __sockets[__sockLatest];
                if (!t) return __sockCall(o, 'sendSocketMessage', { errMsg: 'sendSocketMessage:fail WebSocket is not connected' });
                t.send(o);
            };
            wx.closeSocket = function(o) {
                var t = __sockets[__sockLatest];
                if (!t) return __sockCall(o, 'closeSocket', { errMsg: 'closeSocket:fail WebSocket is not connected' });
                t.close(o);
            };
