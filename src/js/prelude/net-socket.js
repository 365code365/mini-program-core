            // ───────── TCP / UDP（官方 api/network/tcp、api/network/udp）与本机 IP ─────────
            // 连接与收发在原生线程里（src/net_socket.rs），事件由宿主每帧取回后调
            // __resolveNetSocket(id, type, json)。write / send 收 string（按 UTF-8）或 ArrayBuffer / TypedArray，
            // 字节直接过桥（几个 _bin 结尾的原生入口，src/bin_stash.rs）；onMessage 的 message 是 ArrayBuffer（与微信一致），同样不经 base64。
            var __netSeq = 0, __netSockets = {};
            function __netBytes(data, offset, length) {
                var b = typeof data === 'string' ? __utf8encode(data) : __toBytes(data);
                if (offset !== undefined || length !== undefined) {
                    var o = Number(offset) || 0;
                    b = b.subarray(o, length === undefined ? b.length : o + Number(length));
                }
                return b;
            }
            function __netEmitter(kinds) {
                var ls = {}, obj = { __ls: ls };
                kinds.forEach(function(N) {
                    var k = N.charAt(0).toLowerCase() + N.slice(1);
                    ls[k] = [];
                    obj['on' + N] = function(cb) { if (typeof cb === 'function') ls[k].push(cb); };
                    obj['off' + N] = function(cb) { ls[k] = cb ? ls[k].filter(function(f) { return f !== cb; }) : []; };
                });
                return obj;
            }
            function __netFire(sock, type, res) {
                (sock.__ls[type] || []).slice().forEach(function(cb) { try { cb(res); } catch (e) { console.error(e); } });
            }
            // 同步失败（参数错、没连上）：走 onError，与微信「接口不抛异常」一致
            function __netError(sock, err) {
                if (!err) return;
                setTimeout(function() { __netFire(sock, 'error', { errMsg: err }); }, 0);
            }

            wx.createTCPSocket = function(opt) {
                var id = ++__netSeq;
                var s = __netEmitter(['Connect', 'Message', 'Error', 'Close', 'BindWifi']);
                s.connect = function(o) {
                    o = o || {};
                    var timeout = (o.timeout === undefined ? 2 : Number(o.timeout)) * 1000;
                    __netError(s, __native_tcp_connect(String(id), String(o.address || ''), String(o.port || 0), String(Math.max(1, timeout))));
                };
                s.write = function(data) { __netError(s, __native_tcp_write_bin(id, __netBytes(data))); };
                s.close = function() { __native_net_close(String(id)); };
                // 只有 Android 微信有：本引擎不管多网卡，直接报成功
                s.bindWifi = function() { setTimeout(function() { __netFire(s, 'bindWifi', {}); }, 0); };
                __netSockets[id] = s;
                return s;
            };

            wx.createUDPSocket = function(opt) {
                var id = ++__netSeq, ipv6 = !!(opt && opt.type === 'ipv6');
                var s = __netEmitter(['Listening', 'Message', 'Error', 'Close']);
                s.bind = function(port) {
                    var r = __native_udp_bind(String(id), String(port || 0), ipv6 ? '1' : '0');
                    if (r.indexOf('err:') === 0) { __netError(s, r.slice(4)); return 0; }
                    return Number(r);
                };
                s.send = function(o) {
                    o = o || {};
                    __netError(s, __native_udp_send_bin(id, String(o.address || ''), Number(o.port || 0),
                        __netBytes(o.message === undefined ? '' : o.message, o.offset, o.length), !!o.setBroadcast));
                };
                s.connect = function(o) { o = o || {}; __netError(s, __native_udp_connect(String(id), String(o.address || ''), String(o.port || 0))); };
                s.write = function(o) {
                    o = o || {};
                    __netError(s, __native_udp_write_bin(id, __netBytes(o.message === undefined ? '' : o.message, o.offset, o.length)));
                };
                s.setTTL = function(ttl) { __netError(s, __native_udp_ttl(String(id), String(ttl))); };
                s.close = function() { __native_net_close(String(id)); };
                __netSockets[id] = s;
                return s;
            };

            globalThis.__resolveNetSocket = function(id, type, json) {
                var s = __netSockets[id];
                if (!s) return;
                var res = {};
                try { res = JSON.parse(json || '{}'); } catch (e) {}
                if (type === 'message') res.message = res.bin ? (__native_bin_take(2, id) || new ArrayBuffer(0)) : __b64decode(res.message || '');
                delete res.bin;
                if (type === 'close') delete __netSockets[id];
                __netFire(s, type, res);
            };

            wx.getLocalIPAddress = function(o) {
                var ip = __native_local_ip();
                if (!ip) {
                    var bad = { errMsg: 'getLocalIPAddress:fail 没有可用的网络' };
                    o = o || {};
                    if (!o.success && !o.fail && !o.complete) return Promise.reject(bad);
                    setTimeout(function() { o.fail && o.fail(bad); o.complete && o.complete(bad); }, 0);
                    return;
                }
                // 子网掩码 std 拿不到，给最常见的 /24（微信在部分机型上也只给这个）
                return __syncOk(o, 'getLocalIPAddress', { localip: ip, netmask: '255.255.255.0' });
            };
