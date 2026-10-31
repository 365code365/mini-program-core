// ==================== 高性能+：画布 / 图片 / WebGL 的原生入口录下来，交回 App 回放（src/game/remote） ====================
//
// 页面里装的是普通模式那几层（game-image / game-canvas / game-webgl*.js），它们只认 __game_gl / __game_2d 这些原生名字。
// 这里把这些名字换成「录进命令缓冲」：
// - 不要返回值的：只录（一帧几千条，零往返）；
// - createXxx：页面自己发号（CLIENT_BASE 起），号跟着命令走，App 那边记「号 → GL 名字」；
// - 要返回值的：连同之前录下的一起同步发过去（iOS prompt，桌面同步 XHR），等结果；常用的几样在这里缓存，
//   稳态一帧零往返：getParameter 的常量、扩展列表、精度、程序链接后的全部信息（uniform / attribute 的位置、日志）、
//   measureText（按 字体 + 对齐 + 基线 + 文字）、font 的规范化。
// 帧尾（wg-frame.js 的 rAF 跑完）发一包；App 回放完上屏后回 __wgrAck()，在飞的超过 2 包就先不跑下一批 rAF（流控）。
// 线格式见 src/game/remote/mod.rs 文件头。
(function() {
    var G = globalThis, B = __wgBoot;
    if (!B.plus) return;
    var BASE = 0x6A5A0000;
    var FNAMES = ['__game_gl', '__game_2d', '__game_canvas_new', '__game_canvas_ctx', '__game_canvas_resize', '__game_canvas_free',
        '__game_gl_create', '__game_gl_resize', '__game_gl_sync', '__game_gl_free', '__game_gl_lose', '__game_gl_restore',
        '__game_img_new', '__game_img_src', '__game_img_free', '__game_load_font', '__game_text_line_height',
        '__game_canvas_encode', '__game_canvas_save', '__game_canvas_size', '__wgrProgramInfo', '__wgrBatch'];
    var F = {}; FNAMES.forEach(function(n, i) { F[n] = i; });
    var F_PRESENT = 0xF0;

    // ---------------- 编码器 ----------------
    var buf = new ArrayBuffer(1 << 16), dv = new DataView(buf), u8 = new Uint8Array(buf), len = 0;
    function ensure(n) {
        if (len + n <= buf.byteLength) return;
        var cap = buf.byteLength * 2; while (cap < len + n) cap *= 2;
        var nb = new ArrayBuffer(cap); new Uint8Array(nb).set(u8.subarray(0, len));
        buf = nb; dv = new DataView(nb); u8 = new Uint8Array(nb);
    }
    var te = new TextEncoder();
    function blob(tag, b) { ensure(5 + b.length); u8[len++] = tag; dv.setUint32(len, b.length, true); len += 4; u8.set(b, len); len += b.length; }
    function str(s) {
        // ASCII 快路（颜色、字体、大多数文字）：不经 TextEncoder 分配
        var n = s.length, ascii = n < 256;
        for (var i = 0; ascii && i < n; i++) if (s.charCodeAt(i) > 127) ascii = false;
        if (!ascii) return blob(6, te.encode(s));
        ensure(5 + n); u8[len++] = 6; dv.setUint32(len, n, true); len += 4;
        for (var j = 0; j < n; j++) u8[len++] = s.charCodeAt(j);
    }
    var VK = { Int8Array: 1, Uint8Array: 2, Uint8ClampedArray: 3, Int16Array: 4, Uint16Array: 5, Int32Array: 6, Uint32Array: 7, Float32Array: 8, Float64Array: 9 };
    function val(v) {
        if (v === undefined) { ensure(1); u8[len++] = 0; return; }
        if (v === null) { ensure(1); u8[len++] = 1; return; }
        var t = typeof v;
        if (t === 'number') {
            if ((v | 0) === v && !(v === 0 && 1 / v < 0)) { ensure(5); u8[len++] = 5; dv.setInt32(len, v, true); len += 4; }
            else { ensure(9); u8[len++] = 4; dv.setFloat64(len, v, true); len += 8; }
            return;
        }
        if (t === 'boolean') { ensure(1); u8[len++] = v ? 3 : 2; return; }
        if (t === 'string') { str(v); return; }
        if (v instanceof ArrayBuffer) { blobView(0, new Uint8Array(v)); return; }
        if (ArrayBuffer.isView(v)) { blobView(VK[v.constructor.name] || 2, new Uint8Array(v.buffer, v.byteOffset, v.byteLength)); return; }
        if (Array.isArray(v)) { ensure(5); u8[len++] = 8; dv.setUint32(len, v.length, true); len += 4; for (var i = 0; i < v.length; i++) val(v[i]); return; }
        blob(9, te.encode(JSON.stringify(v)));
    }
    function blobView(kind, b) {
        ensure(6 + b.length); u8[len++] = 7; u8[len++] = kind; dv.setUint32(len, b.length, true); len += 4; u8.set(b, len); len += b.length;
    }
    function rec(fn, args, sync) {
        ensure(4); u8[len++] = fn; u8[len++] = sync ? 1 : 0; dv.setUint16(len, args.length, true); len += 2;
        for (var i = 0; i < args.length; i++) val(args[i]);
        schedulePresent();
    }

    // ---------------- 传输 ----------------
    var ios = __wgIOS;
    function b64(bytes) {
        var s = '', CH = 0x8000;
        for (var p = 0; p < bytes.length; p += CH) s += String.fromCharCode.apply(null, bytes.subarray(p, p + CH));
        return btoa(s);
    }
    function take() { var out = u8.slice(0, len); len = 0; return out; }
    var inflight = 0, lastAck = 0;
    function sendAsync(bytes) {
        if (ios) { G.webkit.messageHandlers.wgr.postMessage(b64(bytes)); return; }
        var x = new __wgNative.XHR();
        x.open('POST', '/__wg/rcall', false);
        x.send(bytes);
    }
    function reviver(k, v) {
        if (v && typeof v === 'object' && !Array.isArray(v)) {
            if (v.$u) return undefined;
            if (v.$err) throw new Error(v.$err);
            if (v.$t) {
                var T = G[v.$t];
                if (v.$b !== undefined) { var ab = __wgB64Decode(v.$b); return new T(ab); }
                return new T(v.v);
            }
            if (v.$b !== undefined) return __wgB64Decode(v.$b);
        }
        return v;
    }
    function decode(text) {
        if (text === '' || text === undefined || text === null) return undefined;
        var err = null, r = JSON.parse(text, function(k, v) { try { return reviver(k, v); } catch (e) { err = e; return null; } });
        if (err) throw err;
        return r;
    }
    function sync(fn, args) {
        rec(fn, args, true);
        var bytes = take(), text;
        if (ios) text = G.prompt('__wgr', b64(bytes));
        else {
            var x = new __wgNative.XHR();
            x.open('POST', '/__wg/rcall', false);
            x.send(bytes);
            text = x.responseText;
        }
        return decode(text);
    }
    // 帧尾：发出这一帧（有画东西的话）。flow：在飞的包数
    G.__wgrFlush = function(present) {
        if (present) { ensure(4); u8[len++] = F_PRESENT; u8[len++] = 0; u8[len++] = 0; u8[len++] = 0; }
        if (!len) return;
        var bytes = take();
        if (present && ios) { inflight++; }
        sendAsync(bytes);
    };
    G.__wgrAck = function() { inflight = Math.max(0, inflight - 1); lastAck = performance.now(); };
    // App 还没回放完前两帧：这一个 vsync 不跑 rAF（不然命令越积越多、延迟越来越大）。ack 丢了 500ms 之后放开
    G.__wgrBusy = function() {
        if (inflight >= 2 && performance.now() - lastAck > 500) inflight = 0;
        return inflight >= 2;
    };
    // 不在 rAF 里画的（触摸回调、定时器里画一下就停）：下一个 vsync 自动收尾
    var inTick = false, autoPending = false;
    G.__wgrInTick = function(v) { inTick = v; };
    function schedulePresent() {
        if (inTick || autoPending) return;
        autoPending = true;
        __wgNativeRaf(function() { autoPending = false; if (!inTick) G.__wgrFlush(true); });
    }

    // ---------------- WebGL ----------------
    var OPS = B.glOps || [], OPI = {};
    OPS.forEach(function(n, i) { OPI[n] = i; });
    G.__game_gl_ops = function() { return OPS.slice(); };
    var nextId = BASE + 1;
    var KIND = OPS.map(function(n) {
        if (n.indexOf('create') === 0 || n === 'fenceSync') return 1;
        if (n === '_bufferDataSize' || n === '_synthError') return 0;
        if (/^(get|is|check)/.test(n) || n === '_readPixels' || n === 'clientWaitSync' || n === '_getExtension' || n === '_info') return 2;
        return 0;
    });
    var ctxs = {};   // 画布 id → { k: 常量缓存, ext, prec, prog: {页面号: 信息}, sh: {程序: [着色器]}, slog: {着色器: 日志} }
    function cx(cid) { return ctxs[cid] || (ctxs[cid] = { k: {}, ext: null, prec: {}, prog: {}, sh: {}, slog: {} }); }
    var K1 = [0x0D33, 0x851C, 0x84E8, 0x8869, 0x8DFB, 0x8DFC, 0x8DFD, 0x8872, 0x8B4C, 0x8B4D, 0x0D3A, 0x846E, 0x846D, 0x0D50, 0x80A8, 0x80A9];
    var K2 = [0x8D57, 0x8073, 0x88FF, 0x8824, 0x8CDF, 0x8A2F, 0x8A2B, 0x8A2D, 0x8A2E, 0x8A30, 0x8A34, 0x8B4A, 0x8B49, 0x80E8, 0x80E9, 0x84FD,
        0x8B4B, 0x8905, 0x8904, 0x9111, 0x8D6B, 0x9125, 0x9122, 0x8A31, 0x8A33];
    function prefetch(cid, ver) {
        var c = cx(cid), op = OPI.getParameter, list = (ver === 2 ? K1.concat(K2) : K1);
        var calls = list.map(function(p) { return [cid, op, p]; });
        var prec = [];
        [0x8B31, 0x8B30].forEach(function(s) { [0x8DF0, 0x8DF1, 0x8DF2, 0x8DF3, 0x8DF4, 0x8DF5].forEach(function(t) { prec.push([s, t]); calls.push([cid, OPI.getShaderPrecisionFormat, s, t]); }); });
        calls.push([cid, OPI.getSupportedExtensions]);
        var r = sync(F.__wgrBatch, ['__game_gl', calls]) || [];
        list.forEach(function(p, i) { if (r[i] !== null && r[i] !== undefined) c.k[p] = r[i]; });
        prec.forEach(function(p, i) { c.prec[p[0] + ':' + p[1]] = r[list.length + i]; });
        var ex = r[r.length - 1];
        // 并行编译的扩展不给：three.js 有了它会每帧问一次 COMPLETION_STATUS（每帧一次同步往返）
        c.ext = Array.isArray(ex) ? ex.filter(function(n) { return n !== 'KHR_parallel_shader_compile'; }) : [];
    }
    function clone(v) { return ArrayBuffer.isView(v) ? new v.constructor(v) : Array.isArray(v) ? v.slice() : v; }
    function progInfo(cid, p) {
        var c = cx(cid);
        if (c.prog[p]) return c.prog[p];
        var shaders = (c.sh[p] || []).slice();
        var info = JSON.parse(sync(F.__wgrProgramInfo, [cid, p, shaders]));
        info.uMap = {}; info.aMap = {};
        info.u.forEach(function(u) {
            var name = u[0], base = name.replace(/\[0\]$/, ''), locs = u[3];
            info.uMap[name] = locs[0];
            if (u[1] > 1) { info.uMap[base] = locs[0]; for (var k = 0; k < locs.length; k++) info.uMap[base + '[' + k + ']'] = locs[k]; }
        });
        info.a.forEach(function(a) { info.aMap[a[0]] = a[3]; });
        shaders.forEach(function(s, i) { c.slog[s] = info.s[i]; });
        c.prog[p] = info;
        return info;
    }
    var slice = Array.prototype.slice;
    G.__game_gl = function(cid, op) {
        var k = KIND[op];
        if (k === 0) {
            var name0 = OPS[op];
            if (name0 === 'linkProgram') delete cx(cid).prog[arguments[2]];
            else if (name0 === 'compileShader') delete cx(cid).slog[arguments[2]];
            else if (name0 === 'attachShader') { var s0 = cx(cid).sh; (s0[arguments[2]] = s0[arguments[2]] || []).push(arguments[3]); }
            else if (name0 === 'detachShader') { var l0 = cx(cid).sh[arguments[2]] || [], i0 = l0.indexOf(arguments[3]); if (i0 >= 0) l0.splice(i0, 1); }
            rec(0, arguments, false);
            return;
        }
        if (k === 1) {
            var id = nextId++, a1 = slice.call(arguments);
            a1.push(id);
            rec(0, a1, false);
            return id;
        }
        var c = cx(cid), name = OPS[op], a = arguments;
        switch (name) {
            case 'getParameter':
                if (a[2] in c.k) return clone(c.k[a[2]]);
                break;
            case 'getSupportedExtensions':
                if (c.ext) return c.ext.slice();
                break;
            case '_getExtension':
                if (a[2] === 'KHR_parallel_shader_compile') return false;
                break;
            case 'getShaderPrecisionFormat':
                var pk = a[2] + ':' + a[3];
                if (pk in c.prec) return clone(c.prec[pk]);
                break;
            case 'getProgramParameter':
                if (a[3] === 0x8B82 || a[3] === 0x8B86 || a[3] === 0x8B89) {
                    var pi = progInfo(cid, a[2]);
                    return a[3] === 0x8B82 ? pi.link : a[3] === 0x8B86 ? pi.u.length : pi.a.length;
                }
                break;
            case 'getProgramInfoLog':
                if (c.prog[a[2]]) return c.prog[a[2]].log;
                break;
            case 'getShaderInfoLog':
                if (a[2] in c.slog) return c.slog[a[2]];
                break;
            case 'getActiveUniform':
            case 'getActiveAttrib':
                var pa = c.prog[a[2]], list = pa && (name === 'getActiveUniform' ? pa.u : pa.a);
                if (list && a[3] >= 0 && a[3] < list.length) { var e = list[a[3]]; return { size: e[1], type: e[2], name: e[0] }; }
                break;
            case 'getUniformLocation':
                var pu = c.prog[a[2]];
                if (pu && a[3] in pu.uMap) return pu.uMap[a[3]];
                break;
            case 'getAttribLocation':
                var pq = c.prog[a[2]];
                if (pq && a[3] in pq.aMap) return pq.aMap[a[3]];
                break;
        }
        var r = sync(0, a);
        if (name === 'getSupportedExtensions' && Array.isArray(r)) c.ext = r.filter(function(n) { return n !== 'KHR_parallel_shader_compile'; });
        if (name === 'getShaderPrecisionFormat') c.prec[a[2] + ':' + a[3]] = r;
        return r;
    };
    G.__game_gl_create = function(cid, ver, attrs) {
        var r = sync(F.__game_gl_create, [cid, ver, attrs]);
        if (r) { delete ctxs[cid]; prefetch(cid, ver); }
        return r;
    };
    ['__game_gl_resize', '__game_gl_sync', '__game_gl_free', '__game_gl_lose'].forEach(function(n) {
        G[n] = function() { if (n === '__game_gl_free') delete ctxs[arguments[0]]; rec(F[n], arguments, false); };
    });
    G.__game_gl_restore = function(id) { delete ctxs[id]; return sync(F.__game_gl_restore, [id]); };

    // ---------------- 画布 ----------------
    var nextCanvas = 0, sizes = {}, txt = {};   // txt：画布 id → [font, align, baseline] 与 save 栈
    function tstate(id) { return txt[id] || (txt[id] = { f: '', a: 'start', b: 'alphabetic', st: [] }); }
    G.__game_canvas_new = function() {
        var id = nextCanvas++, main = id === 0, info = __hostSystemInfo;
        var w = main ? Math.max(1, Math.round(info.windowWidth)) : 300, h = main ? Math.max(1, Math.round(info.windowHeight)) : 150;
        sizes[id] = [w, h];
        rec(F.__game_canvas_new, [], false);
        return [id, w, h, main ? 1 : 0];
    };
    G.__game_canvas_size = function(id) { return sizes[id] ? sizes[id].slice() : undefined; };
    G.__game_canvas_resize = function(id, w, h) {
        function clamp(v) { v = Number(v); return isFinite(v) && v > 0 ? Math.min(Math.floor(v), 4096) : 0; }
        if (!sizes[id]) return undefined;
        sizes[id] = [clamp(w), clamp(h)];
        delete txt[id];
        rec(F.__game_canvas_resize, [id, w, h], false);
        return sizes[id].slice();
    };
    G.__game_canvas_ctx = function(id, type) { rec(F.__game_canvas_ctx, [id, type], false); if (type === '2d') tstate(id).b = 'alphabetic'; return type === '2d'; };
    G.__game_canvas_free = function(id) { delete sizes[id]; delete txt[id]; rec(F.__game_canvas_free, [id], false); };
    G.__game_canvas_encode = function() { return sync(F.__game_canvas_encode, arguments); };
    G.__game_canvas_save = function() { return sync(F.__game_canvas_save, arguments); };
    G.__game_canvas_shared = function() { return [nextCanvas++, 300, 150]; };

    var mcache = new Map(), fcache = new Map(), lhcache = new Map();
    G.__game_2d = function(id, op) {
        var a = arguments;
        switch (op) {
            case 17: { var t17 = tstate(id); t17.st.push([t17.f, t17.a, t17.b]); rec(1, a, false); return true; }
            case 18: { var t18 = tstate(id), s18 = t18.st.pop(); if (s18) { t18.f = s18[0]; t18.a = s18[1]; t18.b = s18[2]; } rec(1, a, false); return; }
            case 30: case 31: rec(1, a, false); return true;
            case 35: tstate(id).a = String(a[2]); break;
            case 36: tstate(id).b = String(a[2]); break;
            case 34: {
                var fk = String(a[2]);
                tstate(id).f = fk;
                if (fcache.has(fk)) { rec(1, a, false); return fcache.get(fk); }
                var fr = sync(1, a);
                fcache.set(fk, fr);
                return fr;
            }
            case 27: {
                var t = tstate(id), key = t.f + '\u0001' + t.a + '\u0001' + t.b + '\u0001' + a[2];
                var m = mcache.get(key);
                if (m) return Object.assign({}, m);
                m = sync(1, a);
                if (mcache.size > 8192) mcache.clear();
                mcache.set(key, m);
                return Object.assign({}, m);
            }
            case 8: if (a[4] < 0) throw new RangeError('arc: 半径不能为负（IndexSizeError）'); break;
            case 13: if (a[4] < 0 || a[5] < 0) throw new RangeError('ellipse: 半径不能为负（IndexSizeError）'); break;
            case 45: case 48: return sync(1, a);
        }
        rec(1, a, false);
    };
    G.__game_load_font = function(p) { return sync(F.__game_load_font, [p]); };
    G.__game_text_line_height = function(j) {
        if (lhcache.has(j)) return lhcache.get(j);
        var r = sync(F.__game_text_line_height, [j]);
        lhcache.set(j, r);
        return r;
    };

    // ---------------- 图片：原生解码（包内 / wxfile / http / data 都认），blob: 先在页面里读成 data URL ----------------
    var nextImg = 1;
    G.__game_img_new = function() { rec(F.__game_img_new, [], false); return nextImg++; };
    G.__game_img_free = function(id) { rec(F.__game_img_free, [id], false); };
    G.__game_img_src = function(id, src) {
        src = String(src);
        if (/^blob:/i.test(src)) {
            __wgNative.fetch(src).then(function(r) { return r.arrayBuffer(); }).then(function(ab) {
                rec(F.__game_img_src, [id, 'data:application/octet-stream;base64,' + __wgB64Encode(ab)], false);
            }, function() { rec(F.__game_img_src, [id, ''], false); });
            return;
        }
        rec(F.__game_img_src, [id, src], false);
    };

    // 引擎直接用真 document 建画布 / 图片（Pixi 的 Texture.WHITE、Phaser 的 CanvasPool …）：给游戏画布 / 游戏图片。
    // 它们不是 DOM 节点，挂进 / 移出 DOM 的调用原样返回（普通模式里适配层的假 document 也是这么做的）
    var ce = Document.prototype.createElement;
    Document.prototype.createElement = function(tag) {
        var t = String(tag).toLowerCase();
        if (t === 'canvas') return wx.createCanvas();
        if (t === 'img') return wx.createImage();
        return ce.apply(this, arguments);
    };
    ['appendChild', 'removeChild', 'insertBefore', 'replaceChild'].forEach(function(k) {
        var f = Node.prototype[k];
        Node.prototype[k] = function(n) { return n instanceof Node ? f.apply(this, arguments) : n; };
    });

    // 开放数据域：高性能+ 不支持（宿主看到 openDataContext 回落普通模式）
    wx.getOpenDataContext = function() { return { canvas: wx.createCanvas(), postMessage: function() {} }; };
})();
