            // ==================== 小游戏 Worker：消息的结构化克隆（game-doc/02 §9，GT-W2） ====================
            // 主线程与 Worker 线程各一份（同一个文件）：消息编码成「JSON 骨架 + ArrayBuffer 列表」，跨线程的是字节，
            // 骨架在对方 runtime 里还原。支持 HTML 结构化克隆的常用子集：基本类型（含 undefined / NaN / ±Infinity / -0 / BigInt）、
            // 普通对象、数组、Date、RegExp、Map、Set、Error、ArrayBuffer、全部 TypedArray 与 DataView（共享同一块 buffer 的视图
            // 还原后仍共享）、循环引用与重复引用。函数、Symbol 抛 DataCloneError（与规范一致）。
            // transfer 列表里的 ArrayBuffer：字节照样拷过去，源 buffer 由原生 detach（byteLength 变 0）。
            globalThis.__wclone = (function() {
                var VIEWS = ['Int8Array', 'Uint8Array', 'Uint8ClampedArray', 'Int16Array', 'Uint16Array', 'Int32Array', 'Uint32Array',
                    'Float32Array', 'Float64Array', 'BigInt64Array', 'BigUint64Array'];
                function cloneErr(msg) { var e = new Error(msg); e.name = 'DataCloneError'; return e; }
                function encode(root) {
                    var seen = new Map(), bufs = [], bufIdx = new Map(), nextId = 0;
                    function buf(ab) {
                        if (!bufIdx.has(ab)) { bufIdx.set(ab, bufs.length); bufs.push(ab); }
                        return bufIdx.get(ab);
                    }
                    function enc(v) {
                        var t = typeof v;
                        if (v === null || t === 'boolean' || t === 'string') return v;
                        if (t === 'number') {
                            if (v !== v) return { $: 'n', v: 'NaN' };
                            if (v === Infinity) return { $: 'n', v: 'Infinity' };
                            if (v === -Infinity) return { $: 'n', v: '-Infinity' };
                            if (v === 0 && 1 / v < 0) return { $: 'n', v: '-0' };
                            return v;
                        }
                        if (t === 'undefined') return { $: 'u' };
                        if (t === 'bigint') return { $: 'bi', v: String(v) };
                        if (t === 'function' || t === 'symbol') throw cloneErr(t + ' 不能克隆');
                        if (seen.has(v)) return { $: 'ref', i: seen.get(v) };
                        if (v instanceof ArrayBuffer) return { $: 'ab', i: buf(v) };
                        if (ArrayBuffer.isView(v)) {
                            var ty = v instanceof DataView ? 'DataView' : v.constructor && v.constructor.name;
                            return { $: 'view', t: ty, i: buf(v.buffer), o: v.byteOffset, l: v instanceof DataView ? v.byteLength : v.length };
                        }
                        var id = nextId++;
                        seen.set(v, id);
                        if (v instanceof Date) return { $: 'd', id: id, v: v.getTime() };
                        if (v instanceof RegExp) return { $: 're', id: id, s: v.source, f: v.flags };
                        if (v instanceof Error) return { $: 'err', id: id, n: v.name, m: v.message, s: v.stack };
                        if (v instanceof Map) {
                            var me = []; v.forEach(function(val, k) { me.push([enc(k), enc(val)]); });
                            return { $: 'm', id: id, e: me };
                        }
                        if (v instanceof Set) {
                            var se = []; v.forEach(function(val) { se.push(enc(val)); });
                            return { $: 's', id: id, e: se };
                        }
                        if (Array.isArray(v)) {
                            var ae = new Array(v.length);
                            for (var i = 0; i < v.length; i++) ae[i] = enc(v[i]);
                            return { $: 'a', id: id, e: ae };
                        }
                        var oe = {};
                        Object.keys(v).forEach(function(k) { oe[k] = enc(v[k]); });
                        return { $: 'o', id: id, e: oe };
                    }
                    var tree = enc(root);
                    return [JSON.stringify(tree), bufs];
                }
                function decode(json, bufs) {
                    var refs = [];
                    function dec(n) {
                        if (n === null || typeof n !== 'object') return n;
                        switch (n.$) {
                            case 'n': return n.v === '-0' ? -0 : Number(n.v);
                            case 'u': return undefined;
                            case 'bi': return BigInt(n.v);
                            case 'ref': return refs[n.i];
                            case 'ab': return bufs[n.i];
                            case 'view': {
                                var C = n.t === 'DataView' ? DataView : globalThis[n.t];
                                return n.t === 'DataView' ? new DataView(bufs[n.i], n.o, n.l) : new C(bufs[n.i], n.o, n.l);
                            }
                            case 'd': return (refs[n.id] = new Date(n.v));
                            case 're': return (refs[n.id] = new RegExp(n.s, n.f));
                            case 'err': {
                                var C2 = typeof globalThis[n.n] === 'function' && /Error$/.test(n.n) ? globalThis[n.n] : Error;
                                var e = new C2(n.m); refs[n.id] = e; if (n.s) e.stack = n.s; return e;
                            }
                            case 'm': { var m = new Map(); refs[n.id] = m; n.e.forEach(function(p) { m.set(dec(p[0]), dec(p[1])); }); return m; }
                            case 's': { var s = new Set(); refs[n.id] = s; n.e.forEach(function(x) { s.add(dec(x)); }); return s; }
                            case 'a': { var a = new Array(n.e.length); refs[n.id] = a; for (var i = 0; i < n.e.length; i++) a[i] = dec(n.e[i]); return a; }
                            case 'o': { var o = {}; refs[n.id] = o; Object.keys(n.e).forEach(function(k) { o[k] = dec(n.e[k]); }); return o; }
                        }
                        return undefined;
                    }
                    return dec(JSON.parse(json));
                }
                // 编码 + transfer 列表在 bufs 里的下标（要 detach 的那几块）
                function pack(msg, transfer) {
                    var r = encode(msg), bufs = r[1], tr = [];
                    (transfer || []).forEach(function(t) {
                        if (!(t instanceof ArrayBuffer)) throw cloneErr('transfer 只接受 ArrayBuffer');
                        var i = bufs.indexOf(t);
                        if (i < 0) { i = bufs.length; bufs.push(t); }
                        if (tr.indexOf(i) < 0) tr.push(i);
                    });
                    return [r[0], bufs, tr];
                }
                return { encode: encode, decode: decode, pack: pack, VIEWS: VIEWS };
            })();
