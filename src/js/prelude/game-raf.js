            // ==================== 小游戏：requestAnimationFrame（game-doc/02 §4.2） ====================
            //
            // 节拍在原生（game/raf_pacer.rs：默认 60fps、跟宿主 vsync、高刷屏上也是 60）。到了 rAF 帧，原生调
            // __gameRunRaf(ts)：取出**当前**全部回调（快照），以同一个时间戳依次调用；回调里新注册的进下一批；
            // 回调里取消同批里还没执行的那个，它就不执行（与浏览器一致）。ts 是帧时刻，与 performance.now() 同一时间轴。
            (function() {
                var queue = [];          // [id, cb]
                var cancelled = {};      // 本批执行期间被取消的 id
                var nextId = 1;
                globalThis.requestAnimationFrame = function(cb) {
                    if (typeof cb !== 'function') throw new TypeError('requestAnimationFrame: 参数必须是函数');
                    var id = nextId++;
                    queue.push([id, cb]);
                    return id;
                };
                globalThis.cancelAnimationFrame = function(id) {
                    for (var i = 0; i < queue.length; i++) if (queue[i][0] === id) { queue.splice(i, 1); return; }
                    cancelled[id] = true;
                };
                globalThis.__gameRafPending = function() { return queue.length; };
                globalThis.__gameRunRaf = function(ts) {
                    var batch = queue; queue = []; cancelled = {};
                    for (var i = 0; i < batch.length; i++) {
                        if (cancelled[batch[i][0]]) continue;
                        __gameCall(batch[i][1], null, [ts]);
                    }
                    cancelled = {};
                    return batch.length;
                };
                wx.setPreferredFramesPerSecond = function(fps) { __game_set_fps(Number(fps)); };
                // performance.now / wx.getPerformance().now：与 rAF 参数同一条时间轴（game/clock.rs）。
                // QuickJS 自带的 performance.now 按它自己的原点走，rAF 参数与它对不上，按 dt 积分的动画会错
                var perf = { now: function() { return __game_now(); } };
                try { performance.now = perf.now; } catch (e) { globalThis.performance = perf; }
                if (typeof performance !== 'object' || performance.now !== perf.now) globalThis.performance = perf;
                var oldPerf = wx.getPerformance;
                wx.getPerformance = function() {
                    var p = typeof oldPerf === 'function' ? oldPerf() : {};
                    p.now = perf.now;
                    return p;
                };
            })();
