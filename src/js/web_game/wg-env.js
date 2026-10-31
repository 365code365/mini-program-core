// ==================== 小游戏高性能模式：过桥层（game/web/mod.rs，最先执行） ====================
//
// 页面 ↔ 原生只有两种消息，消息体都是 {"n": 名字, "a": [参数…]}（参数与 QuickJS 模式的 __native_* 原生签名一一对应）：
// - 同步：iOS 用 prompt()（WKUIDelegate 同步回），桌面调试服务用同步 XHR（POST /__wg/sync）；
// - 异步：iOS 用 webkit.messageHandlers.wg.postMessage，桌面攒一批同步 XHR 发（POST /__wg/post，保序）。
// 原生要页面做的事（网络回包、宿主回包、前后台）以一段 JS 交回来：iOS 由宿主 evaluateJavaScript，桌面轮询 GET /__wg/poll。
//
// 这里还装三样东西：
// 1. `__wgG`：GameGlobal / window / self 的代理（模块包装把这三个名字换成它，boot.rs）。浏览器里 `window.location = …`
//    会跳走整个页面、`window.document` / `navigator` 是只读的，而 weapp-adapter 这类适配层正要往全局上挂这些名字 ——
//    赋值落到代理自己的影子表里，读回来是游戏设的那个；其余属性原样读写真全局（`GameGlobal.foo = 1` 之后裸写 `foo` 照样读得到）。
// 2. console：浏览器自己的照常（Safari 的 Web 检查器里看得到），同时按 QuickJS 模式的格式与限流（game/logs.rs）交给原生。
// 3. 基础库用到的 __native_*：storage 在页面里留一份整表（启动时一次给齐，读不过桥），写异步落盘；其余按名字过桥。
(function() {
    var G = globalThis, B = __wgBoot;
    // 浏览器原生的构造器与函数，在游戏代码跑之前留一份：适配层常把 window.Image / XMLHttpRequest / WebSocket 换成
    // 自己基于 wx.* 的实现（经代理写到了真全局上），这里再用全局名字就绕回游戏自己的实现、无限递归（萌宠跑酷就这么栈溢出过）
    var N = G.__wgNative = {
        XHR: G.XMLHttpRequest, fetch: G.fetch.bind(G), WebSocket: G.WebSocket, Image: G.Image, Worker: G.Worker,
        AudioContext: G.AudioContext || G.webkitAudioContext, FontFace: G.FontFace, WebAssembly: G.WebAssembly,
        createElement: (function(ce) { return function(tag) { return ce.call(document, tag); }; })(Document.prototype.createElement)
    };
    // 浏览器自己的 rAF（wg-frame.js 会把全局的换成小游戏语义的那份）
    G.__wgNativeRaf = G.requestAnimationFrame.bind(G);
    var ios = !!(G.webkit && G.webkit.messageHandlers && G.webkit.messageHandlers.wg);

    // ---------------- 消息通道 ----------------
    // 桌面调试服务上：异步消息按批编号（seq）用 fetch 发、不阻塞；同步调用带上「我之前发过的最后一批」，
    // 服务端等那批落地之后才处理它 —— 顺序与 iOS 上一样（异步的先到），主线程又不被每帧的同步 XHR 拖住
    var seq = 0;
    function sync(n, a) {
        var msg = JSON.stringify({ n: n, a: a || [] });
        if (ios) { var r = G.prompt('__wg', msg); return r == null ? '' : String(r); }
        flushPosts();
        var x = new N.XHR();
        x.open('POST', '/__wg/sync?after=' + seq, false);
        x.send(msg);
        return x.responseText;
    }
    var outbox = [], posting = false;
    function flushPosts() {
        posting = false;
        if (!outbox.length) return;
        var batch = outbox.join('\n'); outbox = [];
        N.fetch('/__wg/post?seq=' + (++seq), { method: 'POST', body: batch, keepalive: batch.length < 60000 }).catch(function() {});
    }
    function post(n, a) {
        var msg = JSON.stringify({ n: n, a: a || [] });
        if (ios) { G.webkit.messageHandlers.wg.postMessage(msg); return; }
        outbox.push(msg);
        if (!posting) { posting = true; Promise.resolve().then(flushPosts); }
    }
    G.__wgSync = sync;
    G.__wgPost = post;
    G.__wgIOS = ios;
    // 桌面：轮询原生交回来的 JS（iOS 由宿主直接 evaluateJavaScript）。异步取、一次一个，不压主线程
    if (!ios) {
        var pollOnce = function() {
            N.fetch('/__wg/poll').then(function(r) { return r.text(); }).then(function(js) {
                if (js) (0, eval)(js);
            }, function() {}).then(function() { setTimeout(pollOnce, 8); });
        };
        setTimeout(pollOnce, 0);
    }

    // ---------------- 日志（格式与限流同 QuickJS 模式：game/logs.rs） ----------------
    var MAX_LINES = 500, MAX_BYTES = 4096;
    var lines = [], dropped = 0, logTimer = 0;
    function cut(s) { return s.length > MAX_BYTES ? s.slice(0, MAX_BYTES) + '…（截断 ' + (s.length - MAX_BYTES) + ' 字符）' : s; }
    function flushLogs() {
        logTimer = 0;
        if (!lines.length && !dropped) return;
        var out = lines; lines = [];
        if (dropped) { out.unshift('…丢弃 ' + dropped + ' 行日志（每帧最多 ' + MAX_LINES + ' 行）'); dropped = 0; }
        post('log', [out.join('\n')]);
    }
    function emit(s) {
        if (lines.length >= MAX_LINES) { dropped++; } else { lines.push(cut(String(s))); }
        if (!logTimer) logTimer = setTimeout(flushLogs, 0);
    }
    G.__wgFlushLogs = flushLogs;
    function fmt(a) {
        if (a instanceof Error) return (a.name || 'Error') + ': ' + (a.message || '') + (a.stack ? '\n' + a.stack : '');
        if (a && typeof a === 'object') { try { return JSON.stringify(a); } catch (e) { return String(a); } }
        return String(a);
    }
    function fmtArgs(args) { return Array.prototype.map.call(args, fmt).join(' '); }
    var real = G.console || {};
    function wrap(k, prefix) {
        var orig = typeof real[k] === 'function' ? real[k].bind(real) : function() {};
        return function() { orig.apply(null, arguments); emit(prefix + fmtArgs(arguments)); };
    }
    var con = {};
    ['log', 'info', 'debug', 'dir', 'trace', 'table', 'group', 'groupCollapsed'].forEach(function(k) { con[k] = wrap(k, ''); });
    con.warn = wrap('warn', '[WARN] ');
    con.error = wrap('error', '[ERROR] ');
    con.groupEnd = function() {};
    con.assert = function(c) { if (!c) con.error('Assertion failed: ' + fmtArgs(Array.prototype.slice.call(arguments, 1))); };
    ['count', 'countReset', 'time', 'timeEnd', 'timeLog', 'profile', 'profileEnd', 'clear'].forEach(function(k) {
        con[k] = typeof real[k] === 'function' ? real[k].bind(real) : function() {};
    });
    try { G.console = con; } catch (e) {}
    G.__native_print = function(s) { emit(String(s)); };
    // game-global.js 会换掉它（丢弃式），这里先给一个，免得它之前的段 push 进来没人收
    G.__console_buffer = { push: function() {} };
    G.__print_buffer = [];

    // ---------------- 启动期错误：一段坏了报出来，不让整页悄悄空白 ----------------
    G.__wgFatal = function(where, e) {
        var msg = (e && e.message) || String(e);
        emit('[ERROR] 高性能模式 ' + where + '：' + msg + (e && e.stack ? '\n' + e.stack : ''));
        post('error', ['script', where + '：' + msg, String(e && e.stack || '')]);
    };
    // 异常（顶层脚本、定时器、事件回调）与未处理的拒绝：交给 wx.onError / onUnhandledRejection，并发 error 事件
    G.addEventListener('error', function(ev) {
        var e = ev.error, msg = (e && e.message) || ev.message || 'Script error';
        var stack = (e && e.stack) || ((ev.filename || '') + ':' + (ev.lineno || 0) + ':' + (ev.colno || 0));
        emit('[ERROR] ' + (e && e.name ? e.name + ': ' : '') + msg + '\n' + stack);
        post('error', ['script', msg, stack]);
        if (typeof G.__gameDispatchError === 'function') G.__gameDispatchError(msg, stack);
    });
    G.addEventListener('unhandledrejection', function(ev) {
        var r = ev.reason;
        if (typeof G.__gameDispatchRejection === 'function' && G.__gameDispatchRejection(r, ev.promise) > 0) { ev.preventDefault(); return; }
        emit('[ERROR] Uncaught (in promise) ' + fmt(r));
    });

    // ---------------- GameGlobal / window / self 的代理 ----------------
    // 写进影子表的：location（真赋值会跳页）与写不进真全局的（只读的 document、navigator、top …：Reflect.set 返回 false）。
    // 可替换的（frames、screen、parent …）照常写到真全局上：游戏 `globalThis.frames = fn` 之后宿主 eval 也看得见
    var UNSAFE = Object.assign(Object.create(null), { location: 1 });
    var shadow = Object.create(null), boundCache = new WeakMap(), nativeRe = /\{\s*\[native code\]\s*\}\s*$/;
    function isNative(f) {
        var r = boundCache.get(f);
        if (r !== undefined) return r;
        // 只给「宿主对象上的原生方法」绑 this（setTimeout、addEventListener、requestAnimationFrame …）：
        // 构造器（有 prototype）不绑，否则 Float32Array.BYTES_PER_ELEMENT 之类就没了
        var yes = !('prototype' in f) && nativeRe.test(Function.prototype.toString.call(f));
        r = yes ? f.bind(G) : false;
        boundCache.set(f, r);
        return r;
    }
    // 代理的目标是一个空对象而不是 window 本身：Proxy 的不变式按「目标」检查 —— 目标若是 window，`document` 这种
    // 不可配置、没有 setter 的属性，set 陷阱返回 true 就抛 TypeError（Chrome / WebKit 都这样），影子表就写不进去
    // 微信的 GameGlobal 上没有 DOM 的事件方法：适配层按「有没有 addEventListener」决定自己装一套（weapp-adapter 的 listen），
    // 让它看见 window 的原生 dispatchEvent 就会把普通对象当事件派发 → TypeError（Pixi 收不到点击）
    var HIDE = Object.assign(Object.create(null), { addEventListener: 1, removeEventListener: 1, dispatchEvent: 1 });
    var P = new Proxy(Object.create(null), {
        get: function(t, k) {
            if (typeof k === 'string' && k in shadow) return shadow[k];
            if (HIDE[k]) return undefined;
            if (k === 'window' || k === 'self' || k === 'GameGlobal' || k === 'globalThis') return P;
            var v = G[k];
            if (typeof v === 'function') { var b = isNative(v); if (b) return b; }
            return v;
        },
        set: function(t, k, v) {
            if (typeof k === 'string' && (UNSAFE[k] || HIDE[k] || k === 'window' || k === 'self' || k === 'GameGlobal' || k === 'globalThis')) { shadow[k] = v; return true; }
            var ok = false;
            try { ok = Reflect.set(G, k, v); } catch (e) { ok = false; }
            if (!ok) shadow[k] = v;
            else if (typeof k === 'string') delete shadow[k];
            return true;
        },
        has: function(t, k) { return k in shadow || (!HIDE[k] && k in G); },
        deleteProperty: function(t, k) { delete shadow[k]; try { Reflect.deleteProperty(G, k); } catch (e) {} return true; },
        defineProperty: function(t, k, d) {
            if (typeof k === 'string' && !UNSAFE[k]) { try { if (Reflect.defineProperty(G, k, d)) return true; } catch (e) {} }
            shadow[k] = 'value' in d ? d.value : (d.get ? d.get.call(P) : undefined);
            return true;
        },
        getOwnPropertyDescriptor: function(t, k) {
            if (typeof k === 'string' && k in shadow) return { value: shadow[k], writable: true, enumerable: true, configurable: true };
            var d = Reflect.getOwnPropertyDescriptor(G, k);
            if (d) d.configurable = true;   // 目标上没有它：不能报「不可配置」（不变式）
            return d;
        },
        ownKeys: function() { return Reflect.ownKeys(G).concat(Object.keys(shadow).filter(function(k) { return !(k in G); })); },
        getPrototypeOf: function() { return Object.getPrototypeOf(G); }
    });
    G.__wgG = P;

    // ---------------- 启动数据 → 基础库要的全局（与 QuickJS 模式 GameEngine::new 装的同名） ----------------
    G.__hostSystemInfo = B.info;
    G.__projectAppId = B.appId;
    G.__hostLaunchOptions = B.launch;
    G.__networkTimeout = B.networkTimeout;
    G.__gameSubpackages = B.subpackages;
    G.__gameEntrySub = B.entrySub ? B.entrySub.name : null;
    G.__gameMainBytes = B.mainBytes;
    G.__gamePlugins = {};

    // ---------------- 基础库用到的 __native_* ----------------
    var S = Object.create(null), keys = Object.keys(B.storage || {});
    for (var i = 0; i < keys.length; i++) S[keys[i]] = B.storage[keys[i]];
    function utf8(s) { var n = 0; for (var j = 0; j < s.length; j++) { var c = s.charCodeAt(j); n += c < 0x80 ? 1 : c < 0x800 ? 2 : (c >= 0xd800 && c <= 0xdbff) ? (j++, 4) : 3; } return n; }
    G.__native_storage_get = function(k) { var v = S[k]; return v === undefined ? '' : v; };
    G.__native_storage_set = function(k, v) { S[k] = String(v); post('storage_set', [String(k), String(v)]); };
    G.__native_storage_remove = function(k) { delete S[k]; post('storage_remove', [String(k)]); };
    G.__native_storage_clear = function() { S = Object.create(null); post('storage_clear', []); };
    G.__native_storage_keys = function() { return JSON.stringify(Object.keys(S)); };
    G.__native_storage_size = function() { var n = 0; for (var k in S) n += utf8(k) + utf8(S[k]); return String(n); };
    function syncN(name) { return function() { return sync(name, Array.prototype.map.call(arguments, String)); }; }
    function postN(name) { return function() { post(name, Array.prototype.map.call(arguments, String)); return 'undefined'; }; }
    G.__native_fs = syncN('fs');
    G.__native_temp_write = syncN('temp_write');
    G.__native_temp_remove = postN('temp_remove');
    G.__native_image_info = syncN('image_info');
    G.__native_compress_image = syncN('compress_image');
    G.__native_video_info = syncN('video_info');
    G.__native_local_ip = syncN('local_ip');
    G.__native_clipboard_get = syncN('clipboard_get');
    G.__native_clipboard_set = postN('clipboard_set');
    G.__native_request = postN('request');
    G.__native_download = postN('download');
    G.__native_upload = postN('upload');
    G.__native_host_event = function(json) { post('host_event', [String(json)]); };
    G.__native_host_request = function(kind, json) {
        // 页面自己画得了的（操作菜单、收键盘）先给 wg-overlay.js / wg-input.js 一个机会
        if (typeof G.__wgHostRequest === 'function' && G.__wgHostRequest(String(kind), String(json))) return 'undefined';
        post('host_request', [String(kind), String(json)]);
        return 'undefined';
    };
    // 二进制回包：原生给 base64
    G.__net_take_bytes = function(id) { var b = sync('net_take_bytes', [String(id)]); return b ? G.__wgB64Decode(b) : null; };
    G.__net_take_chunk = function(id) { var b = sync('net_take_chunk', [String(id)]); return b ? G.__wgB64Decode(b) : null; };
    G.__native_random = function(n) {
        var a = new Uint8Array(Math.max(0, Math.min(65536, Number(n) || 0)));
        if (a.length) G.crypto.getRandomValues(a);
        return G.__wgB64Encode(a.buffer);
    };
    // wx.encode / wx.decode / arrayBufferToBase64（game-compat.js）：浏览器自带的就够
    G.__wgB64Encode = function(buf) {
        var u = new Uint8Array(buf), s = '', CH = 0x8000;
        for (var p = 0; p < u.length; p += CH) s += String.fromCharCode.apply(null, u.subarray(p, p + CH));
        return btoa(s);
    };
    G.__wgB64Decode = function(str) {
        var s = atob(String(str).replace(/\s+/g, '')), u = new Uint8Array(s.length);
        for (var p = 0; p < s.length; p++) u[p] = s.charCodeAt(p);
        return u.buffer;
    };
    G.__native_b64_encode = function(b) { return G.__wgB64Encode(b instanceof ArrayBuffer ? b : (b && b.buffer) || new ArrayBuffer(0)); };
    G.__native_b64_decode = function(s) { return G.__wgB64Decode(s); };
    G.__native_text_encode = function(s, enc) { return new TextEncoder().encode(String(s)).buffer; };
    G.__native_text_decode = function(b, enc) {
        try { return new TextDecoder(enc || 'utf-8').decode(b instanceof ArrayBuffer ? new Uint8Array(b) : b); }
        catch (e) { return new TextDecoder('utf-8').decode(b instanceof ArrayBuffer ? new Uint8Array(b) : b); }
    };

    // 原生交回来的回调统一经这里（回调抛错进 wx.onError，不打断同批其余的）
    G.__wgCall = function(fn, args) {
        try { return fn.apply(null, args || []); }
        catch (e) { emit('[ERROR] ' + fmt(e)); if (typeof G.__gameDispatchError === 'function') G.__gameDispatchError(e && e.message || String(e), e && e.stack || ''); }
    };

    // URL：包内路径 → 同源路径；wxfile:// → /__wg/file/…；远程 http(s) → 经原生取（同源，WebGL 不报跨域，resource.rs）
    G.__wgUrl = function(src) {
        src = String(src == null ? '' : src);
        if (!src) return src;
        if (/^(data|blob):/i.test(src)) return src;
        if (src.indexOf('wxfile://') === 0) return '/__wg/file/' + src.slice(9).split('/').map(encodeURIComponent).join('/');
        if (/^https?:\/\//i.test(src)) return '/__wg/proxy?u=' + encodeURIComponent(src);
        if (src.indexOf('//') === 0) return '/__wg/proxy?u=' + encodeURIComponent('https:' + src);
        var p = src.replace(/^\.\//, '').replace(/^\/+/, '');
        return '/' + p.split('/').map(encodeURIComponent).join('/');
    };

    // ---------------- 启动：基础库与 DOM 层都装完之后（boot.html 的最后一个 <script>） ----------------
    G.__wgStart = function() {
        if (B.hostApis && B.hostApis.length && typeof G.__installHostApis === 'function') G.__installHostApis(B.hostApis);
        var mods = B.modules || [], i = 0;
        // 按顺序插入：async=false 的动态脚本按插入顺序执行（与 loader.rs 排序后的注册顺序一致）
        mods.forEach(function(k) {
            var s = N.createElement('script');
            s.async = false;
            s.src = '/__wg/m/' + k.split('/').map(encodeURIComponent).join('/') + '.js';
            s.onerror = function() { G.__wgFatal('module ' + k, new Error('加载失败')); };
            document.head.appendChild(s);
        });
        var e = N.createElement('script');
        e.async = false;
        e.src = '/__wg/entry.js';
        document.head.appendChild(e);
    };
    G.__wgRunEntry = function(run) {
        try { run(); } catch (e) {
            var msg = (e && e.message) || String(e);
            emit('[ERROR] ' + fmt(e));
            post('error', ['script', msg, String(e && e.stack || '')]);
            if (typeof G.__gameDispatchError === 'function') G.__gameDispatchError(msg, e && e.stack || '');
        }
        flushLogs();
        post('host_event', [JSON.stringify({ type: 'started' })]);
    };
})();
