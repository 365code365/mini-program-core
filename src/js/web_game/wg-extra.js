// ==================== 高性能模式：WebSocket、WXWebAssembly、Worker、杂项 ====================
//
// - WebSocket：外壳是基础库 socket.js（SocketTask、旧式全局 API 的语义），原生入口换成浏览器的 WebSocket
//   （WebSocket 不受跨源限制，连接与收发都在 WebKit 的网络进程里）；事件照旧经 __resolveSocket(id, type, json)。
// - WXWebAssembly：WebKit 的 WebAssembly（带 JIT，比 QuickJS 模式的 wasmi 解释器快一个量级）。官方只认包内 .wasm 路径。
// - wx.createWorker：真 Web Worker，跑 `/__wg/worker.js?entry=…`（boot.rs：module.js + worker 全局 + 包里 workers 目录的模块）。
(function() {
    var G = globalThis;

    // ---------------- WebSocket ----------------
    var socks = {}, bins = {};
    function sfire(id, type, res) { setTimeout(function() { __resolveSocket(id, type, JSON.stringify(res || {})); }, 0); }
    G.__native_socket_connect = function(id, url, headerJson, protoJson, timeout) {
        var protos = [];
        try { protos = JSON.parse(protoJson || '[]'); } catch (e) {}
        var ws;
        var WS = __wgNative.WebSocket;
        try { ws = protos.length ? new WS(url, protos) : new WS(url); }
        catch (e) { return 'connectSocket:fail ' + (e && e.message || e); }
        ws.binaryType = 'arraybuffer';
        socks[id] = ws;
        var t = Number(timeout) > 0 ? setTimeout(function() { if (ws.readyState === 0) { ws.close(); sfire(id, 'error', { errMsg: 'connectSocket:fail timeout' }); } }, Number(timeout)) : 0;
        ws.onopen = function() { clearTimeout(t); sfire(id, 'open', { header: {}, profile: {} }); };
        ws.onmessage = function(e) {
            if (typeof e.data === 'string') sfire(id, 'message', { data: e.data });
            else { (bins[id] = bins[id] || []).push(e.data); sfire(id, 'message', { bin: true }); }
        };
        ws.onerror = function() { sfire(id, 'error', { errMsg: 'connectSocket:fail' }); };
        ws.onclose = function(e) { clearTimeout(t); delete socks[id]; sfire(id, 'close', { code: e.code, reason: e.reason }); };
        return '';
    };
    G.__native_socket_send = function(id, kind, data) {
        var ws = socks[id];
        if (!ws || ws.readyState !== 1) return 'sendSocketMessage:fail WebSocket is not connected';
        ws.send(kind === 'b' ? __wgB64Decode(data) : data);
        return '';
    };
    G.__native_socket_send_bin = function(id, bytes) {
        var ws = socks[id];
        if (!ws || ws.readyState !== 1) return 'sendSocketMessage:fail WebSocket is not connected';
        ws.send(bytes);
        return '';
    };
    G.__native_socket_close = function(id, code, reason) {
        var ws = socks[id];
        if (!ws) return 'closeSocket:fail WebSocket is not connected';
        try { ws.close(Number(code) || 1000, reason || ''); } catch (e) { ws.close(); }
        return '';
    };
    G.__native_bin_take = function(kind, id) { var q = bins[id]; return q && q.length ? q.shift() : null; };
    G.__toBytes = G.__toBytes || function(d) { return d instanceof ArrayBuffer ? d : d && d.buffer ? d.buffer.slice(d.byteOffset, d.byteOffset + d.byteLength) : new ArrayBuffer(0); };

    // ---------------- WXWebAssembly ----------------
    // 用启动时留下的 WebAssembly：Cocos 这类引擎的适配层会写 `GameGlobal.WebAssembly = WXWebAssembly`，
    // 这里再用全局名字就自己调自己（farm001 真机上栈溢出过）
    var WA = __wgNative.WebAssembly;
    if (typeof WA === 'object') {
        var W = Object.create(WA);
        W.instantiate = function(src, imports) {
            if (typeof src !== 'string') return WA.instantiate(src, imports);
            if (!/\.wasm(\.br)?$/.test(src)) return Promise.reject(new Error('WXWebAssembly.instantiate: 只支持包内 .wasm 路径（' + src + '）'));
            return __wgNative.fetch(__wgUrl(src) + (/\.br$/.test(src) ? '?br=1' : '')).then(function(r) {
                if (!r.ok) throw new Error('WXWebAssembly.instantiate: 读不出 ' + src);
                return r.arrayBuffer();
            }).then(function(b) { return WA.instantiate(b, imports); });
        };
        G.WXWebAssembly = W;
    }

    // ---------------- Worker ----------------
    // 与 QuickJS 模式 game-worker.js 同一套规矩：脚本必须在 game.json 的 workers 目录里、同一时刻只有一个 Worker
    // （新建的顶掉旧的）、onMessage 的参数是消息本身、Worker 里没接住的异常进 onError({message, stack})
    var active = null;
    wx.createWorker = function(path, opts) {
        var dir = (__wgBoot.workers || '').replace(/^\/+|\/+$/g, '');
        var key = String(path || '').replace(/^\/+/, '').replace(/\.js$/, '');
        if (!dir) throw new Error('createWorker:fail game.json 没有配置 workers');
        if (key.indexOf(dir + '/') !== 0) throw new Error('createWorker:fail 脚本 ' + path + ' 不在 workers 目录 ' + dir + ' 里');
        if (__wgBoot.modules.indexOf(key) < 0) throw new Error('createWorker:fail 脚本不存在：' + path);
        var w = new __wgNative.Worker('/__wg/worker.js?entry=' + encodeURIComponent(key));
        var ls = { message: [], error: [], processKilled: [] }, alive = true;
        function fire(k, arg) { ls[k].slice().forEach(function(f) { __gameCall(f, null, [arg]); }); }
        w.onmessage = function(e) {
            var d = e.data;
            if (d && typeof d === 'object' && typeof d.__wgLog === 'string') { console.log(d.__wgLog); return; }
            if (d && typeof d === 'object' && d.__wgError) { console.error('[worker][error] ' + d.__wgError.message + '\n' + d.__wgError.stack); fire('error', d.__wgError); return; }
            if (alive) fire('message', d);
        };
        w.onerror = function(e) { e.preventDefault && e.preventDefault(); fire('error', { message: e.message || String(e), stack: '' }); };
        var api = {
            postMessage: function(m, transfer) { if (alive) w.postMessage(m, transfer || []); },
            onMessage: function(f) { if (typeof f === 'function') ls.message.push(f); },
            onError: function(f) { if (typeof f === 'function') ls.error.push(f); },
            onProcessKilled: function(f) { if (typeof f === 'function') ls.processKilled.push(f); },
            terminate: function() { if (!alive) return; alive = false; w.terminate(); if (active === api) active = null; },
            getCameraFrameData: function() { return undefined; },
            testOnProcessKilled: function() { if (!alive) return; alive = false; w.terminate(); active = null; fire('processKilled', {}); }
        };
        api.__dead = function() { alive = false; w.terminate(); };
        if (active) active.__dead();
        active = api;
        return api;
    };

    // ---------------- 杂项 ----------------
    // wx.getGPU / 胶囊 / 性能：与 QuickJS 模式同名的几个只读查询
    wx.getPerformance = wx.getPerformance || function() { return { now: function() { return performance.now(); } }; };
    // 高性能模式的标记（官方：vconsole 里 game start 的 render 字段是 "h5"）
    G.__wgMode = 'h5';
})();
