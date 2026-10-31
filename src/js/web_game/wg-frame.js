// ==================== 高性能模式：帧（rAF）、帧率、前后台、窗口尺寸 ====================
//
// 语义对齐 QuickJS 模式（game-raf.js + game/raf_pacer.rs + frame.rs 的 set_background）：
// - requestAnimationFrame：同一批回调用同一个时间戳（vsync 时刻，与 performance.now 同一时间轴），回调里新注册的进下一批，
//   回调里取消同批还没执行的那个就不执行；一个回调抛错不打断同批其余的（进 wx.onError）；
// - wx.setPreferredFramesPerSecond(1~60)：按显示刷新跳帧（WebKit 的 rAF 跟屏幕刷新走，高刷屏上也限到 60 —— 与微信一致）；
// - 进后台：不跑 rAF、按着的手指 touchcancel、WebAudio 挂起，再派发 onHide；回前台先恢复音频再派发 onShow。
//
// 帧统计（宿主注入 __wgStats = true，iOS 的 -gameStats 1）：每 2 秒一行，与 GameFrameStats.swift 同一格式，
// tools/ios-game-sweep.sh 两种模式一起比。「pump」在这里是这一批 rAF 回调的 JS 耗时（WebKit 的提交与合成不在里面）。
(function() {
    var G = globalThis, nativeRaf = G.requestAnimationFrame.bind(G), nativeCancel = G.cancelAnimationFrame.bind(G);
    var queue = [], cancelled = {}, nextId = 1, scheduled = 0, fps = 60, lastRun = -1, hidden = false, firstSent = false;
    function schedule() { if (!scheduled && !hidden && queue.length) scheduled = nativeRaf(tick); }
    function tick(ts) {
        scheduled = 0;
        // 跳帧：离上一次跑过不到一个目标周期（留 2ms 余量吸收 vsync 抖动）就等下一个 vsync
        if (fps < 60 && lastRun >= 0 && ts - lastRun < 1000 / fps - 2) { schedule(); return; }
        lastRun = ts;
        var batch = queue, t0 = performance.now(); queue = []; cancelled = {};
        for (var i = 0; i < batch.length; i++) {
            if (cancelled[batch[i][0]]) continue;
            __gameCall(batch[i][1], null, [ts]);
        }
        cancelled = {};
        // 第一帧：宿主据此撤 loading（与 QuickJS 模式「第一帧上屏」同一时机）
        if (!firstSent && batch.length) { firstSent = true; __native_host_event(JSON.stringify({ type: 'firstFrame' })); }
        if (G.__wgStats) stat(ts, performance.now() - t0);
        __wgFlushLogs();
        schedule();
    }
    G.requestAnimationFrame = function(cb) {
        if (typeof cb !== 'function') throw new TypeError('requestAnimationFrame: 参数必须是函数');
        var id = nextId++;
        queue.push([id, cb]);
        schedule();
        return id;
    };
    G.cancelAnimationFrame = function(id) {
        for (var i = 0; i < queue.length; i++) if (queue[i][0] === id) { queue.splice(i, 1); return; }
        cancelled[id] = true;
    };
    G.__gameRafPending = function() { return queue.length; };
    wx.setPreferredFramesPerSecond = function(v) {
        v = Number(v);
        if (isFinite(v)) fps = Math.max(1, Math.min(60, Math.round(v)));
    };
    var oldPerf = wx.getPerformance;
    wx.getPerformance = function() {
        var p = typeof oldPerf === 'function' ? oldPerf() : {};
        p.now = function() { return performance.now(); };
        return p;
    };

    // ---------------- 前后台（宿主调 __wgLifecycle；页面自己的 visibilitychange 兜底） ----------------
    var hooks = [];
    G.__wgOnLifecycle = function(f) { hooks.push(f); };
    G.__wgLifecycle = function(kind) {
        var bg = kind === 'hide';
        if (bg === hidden) return;
        hidden = bg;
        if (bg) {
            if (scheduled) { nativeCancel(scheduled); scheduled = 0; }
            if (G.__wgCancelTouches) G.__wgCancelTouches();
        }
        hooks.forEach(function(f) { try { f(bg); } catch (e) { console.error(e); } });
        __gameLifecycle(kind);
        if (!bg) { lastRun = -1; schedule(); }
        __wgFlushLogs();
    };
    document.addEventListener('visibilitychange', function() { G.__wgLifecycle(document.hidden ? 'hide' : 'show'); });

    // ---------------- 窗口尺寸：宿主调（与 QuickJS 模式同一入口）；上屏画布的 CSS 尺寸本来就铺满视口 ----------------
    G.__wgResize = function(w, h) { __gameWindowResize({ windowWidth: w, windowHeight: h }); };

    // ---------------- 帧统计（与 GameFrameStats.swift 同一格式） ----------------
    var gaps = [], pumps = [], lastTs = 0, winStart = 0, frames = 0;
    function q(a, p) { if (!a.length) return 0; var s = a.slice().sort(function(x, y) { return x - y; }); return s[Math.min(s.length - 1, Math.floor((s.length - 1) * p))]; }
    function stat(ts, work) {
        if (!winStart) winStart = ts;
        frames++;
        pumps.push(work);
        // 停过帧（没有挂着的 rAF）之后的第一帧不算间隔，否则静止画面被记成掉帧（与 GameFrameStats.idle 同理）
        if (lastTs && ts - lastTs < 1000) gaps.push(ts - lastTs);
        lastTs = ts;
        if (ts - winStart < 2000) return;
        var span = (ts - winStart) / 1000, budget = 1000 / fps;
        var slow = gaps.filter(function(g) { return g > budget * 1.5; }).length;
        console.log('⏱ [GAME] ' + span.toFixed(0) + 's 上屏 ' + frames + ' 帧（' + (frames / span).toFixed(1) + ' fps，目标 ' + fps + '）间隔中位 ' +
            q(gaps, 0.5).toFixed(1) + ' 最大 ' + Math.max.apply(null, gaps.concat([0])).toFixed(1) + 'ms 掉帧 ' + slow + '；pump 中位 ' +
            q(pumps, 0.5).toFixed(1) + ' p95 ' + q(pumps, 0.95).toFixed(1) + ' 最大 ' + Math.max.apply(null, pumps).toFixed(1) +
            'ms；present 中位 0.0 最大 0.0ms；发热 ' + (G.__wgThermal || '正常') + '（高性能模式）');
        gaps = []; pumps = []; frames = 0; winStart = ts;
    }
})();
