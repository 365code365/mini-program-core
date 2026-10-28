            // 开放数据域的运行环境（game-doc/04 G9.7，原生在 game/opendata.rs）：独立的 QuickJS 运行时，只有这里给的东西。
            // 没有网络、存储、文件（与微信一致：开放数据域拿不到这些）；有 2D 画布、图片、定时器、rAF、console。
            // 定时器与 rAF 是 JS 里的队列，由主域每帧 __odTick(毫秒) 推进（与主域同一个帧时钟）。
            (function() {
                var g = globalThis;
                g.GameGlobal = g; g.window = undefined;
                g.wx = {};
                function fmt(a) { return Array.prototype.map.call(a, function(x) { try { return typeof x === 'string' ? x : x instanceof Error ? x.stack || String(x) : JSON.stringify(x); } catch (e) { return String(x); } }).join(' '); }
                g.console = {
                    log: function() { __od_log('log', fmt(arguments)); }, info: function() { __od_log('info', fmt(arguments)); },
                    warn: function() { __od_log('warn', fmt(arguments)); }, error: function() { __od_log('error', fmt(arguments)); },
                    debug: function() { __od_log('debug', fmt(arguments)); }
                };
                g.__gameCall = function(fn, self, args) {
                    try { return fn.apply(self, args || []); } catch (e) { console.error('[开放数据域] ' + (e && e.stack || e)); }
                };
                var now = 0, seq = 0, timers = {}, rafs = [];
                function add(fn, ms, rep, args) {
                    var id = ++seq; timers[id] = { fn: fn, at: now + Math.max(0, Number(ms) || 0), ms: Math.max(1, Number(ms) || 0), rep: rep, args: args }; return id;
                }
                g.setTimeout = function(fn, ms) { return add(fn, ms, false, [].slice.call(arguments, 2)); };
                g.setInterval = function(fn, ms) { return add(fn, ms, true, [].slice.call(arguments, 2)); };
                g.clearTimeout = g.clearInterval = function(id) { delete timers[id]; };
                g.requestAnimationFrame = function(fn) { rafs.push([++seq, fn]); return seq; };
                g.cancelAnimationFrame = function(id) { rafs = rafs.filter(function(r) { return r[0] !== id; }); };
                g.performance = { now: function() { return now; } };
                g.__odTick = function(ms, raf) {
                    now = ms;
                    Object.keys(timers).map(Number).sort(function(a, b) { return timers[a].at - timers[b].at || a - b; }).forEach(function(id) {
                        var t = timers[id]; if (!t || t.at > now) return;
                        if (t.rep) t.at = now + t.ms; else delete timers[id];
                        __gameCall(t.fn, null, t.args);
                    });
                    if (raf) { var list = rafs; rafs = []; list.forEach(function(r) { __gameCall(r[1], null, [now]); }); }
                    return rafs.length + Object.keys(timers).length;
                };
                function cb(o, name, ok, res) {
                    o = o || {}; res = res || {}; res.errMsg = name + (ok ? ':ok' : ':fail ' + (res.errMsg || ''));
                    setTimeout(function() { if (ok && o.success) __gameCall(o.success, null, [res]); if (!ok && o.fail) __gameCall(o.fail, null, [res]); if (o.complete) __gameCall(o.complete, null, [res]); }, 0);
                    if (!o.success && !o.fail && !o.complete) return ok ? Promise.resolve(res) : Promise.reject(res);
                }
                g.__asyncOk = function(o, name, res) { return cb(o, name, true, res); };
                g.__asyncFail = function(o, name, msg) { return cb(o, name, false, { errMsg: msg }); };
            })();
