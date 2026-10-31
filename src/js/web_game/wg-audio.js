// ==================== 高性能模式：音频（WebKit 的 WebAudio） ====================
//
// - InnerAudioContext：外壳仍是基础库 audio.js（属性默认值、事件表、背景音频 / 旧接口全部沿用），这里只实现它调的
//   `__native_audio(op, id, json)`，事件照旧经 `__resolveAudio(id, type, json)` 回去。解码用 WebAudio（同源取字节再
//   decodeAudioData：自定义协议上 <audio> 元素不一定放得出来，WebAudio 只要字节）；同一个 src 解一次，按 src 缓存。
// - wx.createWebAudioContext：就是浏览器的 AudioContext（官方 WebAudioContext 的方法与它同名同义）。
// - 前后台：进后台全部挂起（InnerAudio 记下播放位置），回前台恢复 —— 与 QuickJS 模式 audio.set_background 一致。
// - 第一次触摸时把 AudioContext 唤醒：WebKit 在没有用户手势时可能把它建成 suspended（宿主已放开自动播放，这里兜底）。
(function() {
    var G = globalThis, AC = __wgNative.AudioContext;
    var contexts = [];
    var shared = null;
    function ctx() {
        if (!shared) { shared = new AC(); contexts.push(shared); }
        return shared;
    }
    var buffers = {}, loading = {};   // src → AudioBuffer / [回调…]
    function load(src, done) {
        if (buffers[src]) return done(null, buffers[src]);
        if (loading[src]) return loading[src].push(done);
        loading[src] = [done];
        var finish = function(err, buf) {
            var l = loading[src]; delete loading[src];
            if (buf) buffers[src] = buf;
            l.forEach(function(f) { f(err, buf); });
        };
        __wgNative.fetch(__wgUrl(src)).then(function(r) {
            if (!r.ok) throw { code: 10003, msg: 'file not found ' + src };
            return r.arrayBuffer();
        }).then(function(ab) {
            return new Promise(function(res, rej) { ctx().decodeAudioData(ab, res, function() { rej({ code: 10004, msg: 'decode failed ' + src }); }); });
        }).then(function(buf) { finish(null, buf); }, function(e) { finish(e && e.code ? e : { code: 10002, msg: String(e && e.message || e) }); });
    }

    var A = {};   // id → 状态
    function fire(id, type, res) { setTimeout(function() { if (A[id] || type === 'stop') __resolveAudio(id, type, JSON.stringify(res || {})); }, 0); }
    function pos(a) {
        if (!a.node) return a.offset;
        var t = a.offset + (ctx().currentTime - a.startedAt) * a.playbackRate;
        var d = a.buffer ? a.buffer.duration : 0;
        return a.loop && d > 0 ? t % d : Math.min(t, d);
    }
    function stopNode(a) {
        if (!a.node) return;
        a.node.onended = null;
        try { a.node.stop(); } catch (e) {}
        a.node.disconnect();
        a.node = null;
    }
    function startNode(id, a) {
        var c = ctx();
        var n = c.createBufferSource();
        n.buffer = a.buffer;
        n.loop = !!a.loop;
        n.playbackRate.value = a.playbackRate;
        n.connect(a.gain);
        n.onended = function() {
            if (a.node !== n) return;
            a.node = null; a.offset = 0; a.paused = true;
            fire(id, 'ended');
        };
        a.startedAt = c.currentTime;
        n.start(0, Math.min(a.offset, Math.max(0, a.buffer.duration - 0.001)));
        a.node = n;
    }
    function play(id, a) {
        a.wantPlay = true;
        if (!a.buffer) { if (a.src) { if (!a.loading) begin(id, a); } else fire(id, 'error', { errMsg: 'src is empty', errCode: 10003 }); return; }
        if (a.node) return;
        if (ctx().state === 'suspended' && !hidden) ctx().resume();
        a.paused = false;
        startNode(id, a);
        fire(id, 'play');
        tick(id, a);
    }
    function tick(id, a) {
        clearInterval(a.timer);
        a.timer = setInterval(function() { if (a.node) fire(id, 'timeUpdate'); else clearInterval(a.timer); }, 250);
    }
    function begin(id, a) {
        var src = a.src, gen = ++a.gen;
        a.loading = true;
        fire(id, 'waiting');
        load(src, function(err, buf) {
            if (!A[id] || a.gen !== gen) return;
            a.loading = false;
            if (err) { a.wantPlay = false; fire(id, 'error', { errMsg: err.msg, errCode: err.code }); return; }
            a.buffer = buf;
            if (!a.started) { a.offset = Math.max(0, Number(a.startTime) || 0); a.started = true; }
            fire(id, 'canplay');
            if (a.wantPlay || a.autoplay) play(id, a);
        });
    }
    var hidden = false;
    G.__native_audio = function(op, id, json) {
        var p = {};
        try { p = JSON.parse(json || '{}'); } catch (e) {}
        var a = A[id];
        switch (op) {
            case 'create':
                var g = ctx().createGain(); g.connect(ctx().destination);
                A[id] = { src: '', startTime: 0, autoplay: false, loop: false, volume: 1, playbackRate: 1, buffer: null, node: null,
                    gain: g, offset: 0, startedAt: 0, paused: true, wantPlay: false, gen: 0, started: false, timer: 0 };
                return '';
            case 'state':
                if (!a) return '{}';
                return JSON.stringify({ currentTime: pos(a), duration: a.buffer ? a.buffer.duration : 0, paused: !a.node, buffered: a.buffer ? a.buffer.duration : 0 });
        }
        if (!a) return '';
        switch (op) {
            case 'set':
                Object.keys(p).forEach(function(k) {
                    var v = p[k];
                    if (k === 'src') {
                        // 换 src：停掉旧的、立刻开始加载新的（微信设了 src 就预加载，canplay 之后 autoplay / 之前的 play() 才出声）
                        var was = a.wantPlay && !a.node;
                        stopNode(a); a.src = String(v || ''); a.buffer = null; a.offset = 0; a.started = false; a.wantPlay = was;
                        if (a.src) begin(id, a);
                    } else if (k === 'volume') { a.volume = Math.max(0, Math.min(1, Number(v))); a.gain.gain.value = a.volume; }
                    else if (k === 'loop') { a.loop = !!v; if (a.node) a.node.loop = a.loop; }
                    else if (k === 'playbackRate') { a.playbackRate = Math.max(0.5, Math.min(2, Number(v) || 1)); if (a.node) { a.offset = pos(a); a.startedAt = ctx().currentTime; a.node.playbackRate.value = a.playbackRate; } }
                    else if (k === 'autoplay') { a.autoplay = !!v; if (a.autoplay && a.buffer && !a.node) play(id, a); }
                    else if (k === 'startTime') a.startTime = Number(v) || 0;
                });
                return '';
            case 'play': play(id, a); return '';
            case 'pause':
                a.wantPlay = false;
                if (a.node) { a.offset = pos(a); stopNode(a); a.paused = true; fire(id, 'pause'); }
                return '';
            case 'stop':
                a.wantPlay = false;
                stopNode(a); a.offset = 0; a.paused = true; fire(id, 'stop');
                return '';
            case 'seek':
                var to = Math.max(0, Number(p.position) || 0);
                fire(id, 'seeking');
                if (a.node) { stopNode(a); a.offset = to; startNode(id, a); } else a.offset = to;
                fire(id, 'seeked');
                return '';
            case 'destroy':
                stopNode(a); clearInterval(a.timer); a.gain.disconnect(); delete A[id];
                return '';
        }
        return '';
    };
    wx.setInnerAudioOption = function(o) { return __syncOk(o, 'setInnerAudioOption'); };

    wx.createWebAudioContext = function() {
        var c = new AC();
        contexts.push(c);
        return c;
    };

    // 前后台：挂起 / 恢复全部 AudioContext（InnerAudio 的位置按时钟算，挂起期间不走）
    __wgOnLifecycle(function(bg) {
        hidden = bg;
        contexts.forEach(function(c) { try { bg ? c.suspend() : c.resume(); } catch (e) {} });
    });
    // 被系统打断（来电、闹钟）：WebKit 把 AudioContext 变成 interrupted —— 与 QuickJS 模式同样派发 onAudioInterruptionBegin / End
    function watch(c) {
        var was = c.state;
        c.addEventListener('statechange', function() {
            var s = c.state;
            if (s === 'interrupted' && was !== 'interrupted') __hostDispatch('audioInterruptionBegin', '{}');
            if (was === 'interrupted' && s !== 'interrupted') __hostDispatch('audioInterruptionEnd', '{}');
            was = s;
        });
    }
    var origPush = contexts.push;
    contexts.push = function(c) { watch(c); return origPush.call(contexts, c); };
    function unlock() {
        contexts.forEach(function(c) { if (c.state === 'suspended' && !hidden) c.resume(); });
    }
    document.addEventListener('touchend', unlock, true);
    document.addEventListener('mousedown', unlock, true);
})();
