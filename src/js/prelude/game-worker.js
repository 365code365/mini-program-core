            // ==================== 小游戏：wx.createWorker 真线程（game-doc/02 §9、04 G6.5；原生在 game/worker） ====================
            // 换掉 worker-mq.js 里同一运行时的那份：脚本在另一条线程、另一个 QuickJS 运行时里跑，长计算不卡主线程（GT-W1）。
            // 消息经 __wclone（game-worker-codec.js）编码，ArrayBuffer 按字节跨线程；postMessage 的第二个参数是 transfer 列表。
            // onMessage 的参数是消息本身（与小程序那份一致；官方文档写的是 res.message，真机形状待取证）。
            (function() {
                if (typeof __game_worker !== 'function') return;
                var active = null;
                function fail(msg) { var e = new Error(msg); return e; }
                wx.createWorker = function(scriptPath, opt) {
                    var err = __game_worker('create', String(scriptPath || ''));
                    if (err) throw fail(err);
                    var ls = { message: [], error: [], processKilled: [] }, alive = true;
                    var w = {
                        postMessage: function(msg, transfer) {
                            if (!alive) return;
                            var p = __wclone.pack(msg, transfer);
                            __game_worker_post(p[0], p[1], p[2]);
                        },
                        onMessage: function(cb) { if (typeof cb === 'function') ls.message.push(cb); },
                        onError: function(cb) { if (typeof cb === 'function') ls.error.push(cb); },
                        onProcessKilled: function(cb) { if (typeof cb === 'function') ls.processKilled.push(cb); },
                        terminate: function() {
                            if (!alive) return;
                            alive = false;
                            if (active === w) { active = null; __game_worker('terminate'); }
                        },
                        getCameraFrameData: function() { return undefined; },
                        testOnProcessKilled: function() { if (alive && active === w) __game_worker('kill'); }
                    };
                    w.__ls = ls;
                    w.__dead = function() { alive = false; };
                    if (active) active.__dead();
                    active = w;
                    return w;
                };
                globalThis.__gameWorkerDeliver = function() {
                    var r = __game_worker_take();
                    if (!r || !active) return 0;
                    var msg = __wclone.decode(r[0], r[1]);
                    var list = active.__ls.message.slice();
                    for (var i = 0; i < list.length; i++) __gameCall(list[i], null, [msg]);
                    return list.length;
                };
                globalThis.__gameWorkerEvent = function(kind, res) {
                    if (kind === 'deliver') return __gameWorkerDeliver();
                    var w = active;
                    if (!w) return 0;
                    if (kind === 'processKilled') { w.__dead(); active = null; }
                    var list = w.__ls[kind].slice();
                    for (var i = 0; i < list.length; i++) __gameCall(list[i], null, [res]);
                    return list.length;
                };
            })();
