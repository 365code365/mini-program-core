// Worker 线程上的运行环境（game/worker/thread.rs 在新的 QuickJS 运行时里先执行 module.js、game-worker-codec.js，再执行这里）。
// 官方 Worker 里能用的：全局 `worker`（onMessage / postMessage / getCameraFrameData / testOnProcessKilled）、console、
// 定时器、require（按 Worker 目录解析）。不装 wx 的画布 / 音频 / UI / 网络 / 文件（02 §9；可用集合待取证 GT-W5）。
// 定时器按本线程的单调时钟走（__w_now，毫秒）；原生事件循环在没有消息也没有到点定时器时阻塞等待，不轮询。
(function() {
    var g = globalThis;
    g.GameGlobal = g;
    function fmt(args) {
        return Array.prototype.map.call(args, function(a) {
            if (typeof a === 'string') return a;
            if (a instanceof Error) return (a.name || 'Error') + ': ' + a.message + (a.stack ? '\n' + a.stack : '');
            try { return JSON.stringify(a); } catch (e) { return String(a); }
        }).join(' ');
    }
    g.console = {};
    ['log', 'info', 'warn', 'error', 'debug'].forEach(function(l) { g.console[l] = function() { __w_print(l, fmt(arguments)); }; });

    var timers = {}, seq = 0;
    function add(cb, ms, args, repeat) {
        if (typeof cb !== 'function') return 0;
        var id = ++seq, d = Math.max(0, Number(ms) || 0);
        timers[id] = { cb: cb, args: args, at: __w_now() + d, every: repeat ? Math.max(1, d) : 0 };
        return id;
    }
    g.setTimeout = function(cb, ms) { return add(cb, ms, Array.prototype.slice.call(arguments, 2), false); };
    g.setInterval = function(cb, ms) { return add(cb, ms, Array.prototype.slice.call(arguments, 2), true); };
    g.clearTimeout = g.clearInterval = function(id) { delete timers[id]; };
    function report(e) {
        __w_error(e && e.message !== undefined ? String(e.message) : String(e), e && e.stack ? String(e.stack) : '');
    }
    // 下一个定时器还有多久（毫秒）；没有返回 -1
    g.__wNextDue = function() {
        var now = __w_now(), best = -1;
        for (var k in timers) { var d = Math.max(0, timers[k].at - now); if (best < 0 || d < best) best = d; }
        return best;
    };
    g.__wRunTimers = function() {
        var now = __w_now(), due = [];
        for (var k in timers) if (timers[k].at <= now) due.push(+k);
        due.sort(function(a, b) { return timers[a].at - timers[b].at || a - b; });
        due.forEach(function(id) {
            var t = timers[id];
            if (!t) return;
            if (t.every) t.at = now + t.every; else delete timers[id];
            try { t.cb.apply(null, t.args); } catch (e) { report(e); }
        });
        return 0;
    };

    var listeners = [];
    g.worker = {
        onMessage: function(cb) { if (typeof cb === 'function') listeners.push(cb); },
        postMessage: function(msg, transfer) {
            var p = __wclone.pack(msg, transfer);
            __w_post(p[0], p[1], p[2]);
        },
        getCameraFrameData: function() { return undefined; },
        testOnProcessKilled: function() { __w_kill(); }
    };
    // 原生收到主线程的消息后调：取出来、还原、交给 onMessage（参数就是消息本身，与小程序那份一致；形状待取证）
    g.__wDeliver = function() {
        var r = __w_take();
        if (!r) return 0;
        var msg = __wclone.decode(r[0], r[1]);
        listeners.slice().forEach(function(cb) { try { cb(msg); } catch (e) { report(e); } });
        return listeners.length;
    };
    g.__wReport = report;
})();
