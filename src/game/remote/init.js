// 高性能+ 回放端（src/game/remote）的 JS：装在只回放、不跑游戏的那个 QuickJS 里（基础库之后）。
(function() {
    var G = globalThis;
    // 异步回调交回页面：宿主事件 {type:'wgr', js}，宿主在页面里执行这段 JS
    function toPage(js) { __native_host_event(JSON.stringify({ type: 'wgr', js: js })); }
    G.__gameImageDone = function(id, json) { toPage('__gameImageDone(' + id + ',' + JSON.stringify(json) + ')'); return 1; };
    G.__gameGlEvent = function(id, ev) { toPage('__gameGlEvent(' + id + ',' + JSON.stringify(ev) + ')'); return 1; };

    // 返回值 → JSON：类型化数组 {"$t": 构造器名, "v": [...]}、ArrayBuffer {"$b": base64}、undefined {"$u":1}
    var TA = ['Int8Array', 'Uint8Array', 'Uint8ClampedArray', 'Int16Array', 'Uint16Array', 'Int32Array', 'Uint32Array', 'Float32Array', 'Float64Array'];
    function enc(v) {
        if (v === undefined) return { $u: 1 };
        if (v === null || typeof v !== 'object') return v;
        if (v instanceof ArrayBuffer) return { $b: __native_b64_encode(v) };
        for (var i = 0; i < TA.length; i++) {
            if (typeof G[TA[i]] === 'function' && v instanceof G[TA[i]]) {
                // 大块像素（getImageData / readPixels）走 base64，小的（getParameter）直接列数
                if (v.byteLength > 256) return { $t: TA[i], $b: __native_b64_encode(v.buffer.slice(v.byteOffset, v.byteOffset + v.byteLength)) };
                return { $t: TA[i], v: Array.prototype.slice.call(v) };
            }
        }
        if (Array.isArray(v)) return v.map(enc);
        var o = {};
        for (var k in v) o[k] = enc(v[k]);
        return o;
    }
    G.__wgrEnc = function(v) { return JSON.stringify(enc(v)); };

    var O = {};
    if (typeof __game_gl_ops === 'function') __game_gl_ops().forEach(function(n, i) { O[n] = i; });
    // 程序链接之后页面要的全部信息（一次往返）：链接状态、日志、着色器日志、活跃 uniform（数组每个元素的位置）与 attribute
    G.__wgrProgramInfo = function(cid, p, shaders) {
        var gl = __game_gl;
        var link = !!gl(cid, O.getProgramParameter, p, 0x8B82);
        var out = { link: link, log: gl(cid, O.getProgramInfoLog, p) || '', s: [], u: [], a: [] };
        (shaders || []).forEach(function(s) { out.s.push(gl(cid, O.getShaderInfoLog, s) || ''); });
        if (link) {
            var nu = gl(cid, O.getProgramParameter, p, 0x8B86) || 0;
            for (var i = 0; i < nu; i++) {
                var r = gl(cid, O.getActiveUniform, p, i);
                if (!r) continue;
                var base = r.name.replace(/\[0\]$/, ''), locs = [];
                for (var k = 0; k < r.size; k++) {
                    var l = gl(cid, O.getUniformLocation, p, r.size > 1 ? base + '[' + k + ']' : r.name);
                    locs.push(l === undefined ? null : l);
                }
                out.u.push([r.name, r.size, r.type, locs]);
            }
            var na = gl(cid, O.getProgramParameter, p, 0x8B89) || 0;
            for (var j = 0; j < na; j++) {
                var q = gl(cid, O.getActiveAttrib, p, j);
                if (q) out.a.push([q.name, q.size, q.type, gl(cid, O.getAttribLocation, p, q.name)]);
            }
        }
        return JSON.stringify(out);
    };
    // 一次问多条：(函数名, [参数列表…]) → [结果…]（建上下文时把常量、精度、扩展列表一起问了）
    G.__wgrBatch = function(name, lists) {
        var f = G[name];
        return lists.map(function(a) { try { return f.apply(null, a); } catch (e) { return null; } });
    };
})();
