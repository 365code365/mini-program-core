            // WXWebAssembly（game-doc/04 G10，原生在 game/wasm）：instantiate(代码包内路径, imports)、Memory / Table / Global / Instance、
            // CompileError / LinkError / RuntimeError。与标准 WebAssembly JS API 同一套转换：i64 ↔ BigInt、funcref 只能是 wasm 导出的函数。
            (function() {
                if (typeof __wasm_compile !== 'function') return;
                // 每块内存当前交给 JS 的 buffer（原生在内存搬家时从这里拿出来 detach，game/wasm/mod.rs）
                globalThis.__wasmBufs = [];
                function errClass(name) {
                    function E(message) {
                        if (!(this instanceof E)) return new E(message);
                        var e = new Error(message); Object.setPrototypeOf(e, E.prototype); e.name = name; return e;
                    }
                    E.prototype = Object.create(Error.prototype);
                    E.prototype.constructor = E; E.prototype.name = name;
                    return E;
                }
                var CompileError = errClass('CompileError'), LinkError = errClass('LinkError'), RuntimeError = errClass('RuntimeError');
                var KIND = { CompileError: CompileError, LinkError: LinkError, RuntimeError: RuntimeError };
                // 原生抛的「Kind: 文案」换成对应的错误类；JS 导入函数抛的原样
                function conv(e) {
                    var m = e && typeof e.message === 'string' ? e.message : '';
                    var i = m.indexOf(': ');
                    if (i > 0 && KIND[m.slice(0, i)] && e.name === 'Error') return new KIND[m.slice(0, i)](m.slice(i + 2));
                    return e;
                }
                function call(f) { try { return f(); } catch (e) { throw conv(e); } }

                // externref：JS 值按编号存（0 留给 undefined）
                var ext = [undefined];
                function extId(v) { if (v === null) return null; var i = ext.indexOf(v); if (i < 0) { ext.push(v); i = ext.length - 1; } return i; }
                function toWasm(v, t) {
                    if (t === 'anyfunc' || t === 'funcref') {
                        if (v === null || v === undefined) return null;
                        if (typeof v === 'function' && v.__fid !== undefined) return v.__fid;
                        throw new TypeError('funcref 只能是 wasm 导出的函数或 null');
                    }
                    if (t === 'externref') return v === undefined ? 0 : extId(v);
                    return v;
                }
                function toJS(v, t) {
                    if (t === 'anyfunc' || t === 'funcref') return v === null ? null : fn(v, String(v));
                    if (t === 'externref') return v === null ? null : ext[v];
                    return v;
                }
                // wasm 函数 → JS 函数（length = 参数个数，name = 导出名）
                function fn(fid, name, sig) {
                    sig = sig || JSON.parse(__wasm_func_type(fid));
                    var P = sig.params, R = sig.results;
                    var f = function() {
                        var a = [fid];
                        for (var i = 0; i < P.length; i++) a.push(toWasm(arguments[i], P[i]));
                        var r = call(function() { return __wasm_call.apply(null, a); });
                        if (R.length === 0) return undefined;
                        if (R.length === 1) return toJS(r, R[0]);
                        return r.map(function(x, i) { return toJS(x, R[i]); });
                    };
                    Object.defineProperty(f, 'length', { value: P.length });
                    Object.defineProperty(f, 'name', { value: name });
                    Object.defineProperty(f, '__fid', { value: fid });
                    return f;
                }
                var hosts = [], hostSig = [];
                globalThis.__wasmHostCall = function(hid, args) {
                    var s = hostSig[hid];
                    var a = args.map(function(x, i) { return toJS(x, s.params[i]); });
                    var r = hosts[hid].apply(undefined, a);
                    if (s.results.length === 0) return undefined;
                    if (s.results.length === 1) return toWasm(r, s.results[0]);
                    r = Array.prototype.slice.call(r);
                    return r.map(function(x, i) { return toWasm(x, s.results[i]); });
                };

                function Memory(d) {
                    if (!(this instanceof Memory)) throw new TypeError('WXWebAssembly.Memory 要用 new');
                    if (!d || (d.initial === undefined && d.__id === undefined)) throw new TypeError('WXWebAssembly.Memory：缺 initial');
                    Object.defineProperty(this, '__id', { value: d.__id !== undefined ? d.__id : __wasm_mem_new(Number(d.initial), d.maximum === undefined ? undefined : Number(d.maximum)) });
                }
                Object.defineProperty(Memory.prototype, 'buffer', { get: function() { return __wasm_mem_buffer(this.__id); } });
                Memory.prototype.grow = function(n) { return __wasm_mem_grow(this.__id, Number(n)); };
                function Table(d, v) {
                    if (!(this instanceof Table)) throw new TypeError('WXWebAssembly.Table 要用 new');
                    if (!d || (d.initial === undefined && d.__id === undefined)) throw new TypeError('WXWebAssembly.Table：缺 initial');
                    var el = d.element === 'externref' ? 'externref' : 'anyfunc';
                    Object.defineProperty(this, '__id', { value: d.__id !== undefined ? d.__id : __wasm_table_new(el, Number(d.initial), d.maximum === undefined ? undefined : Number(d.maximum)) });
                    Object.defineProperty(this, '__el', { value: d.__el || el });
                    if (v !== undefined && d.__id === undefined) for (var i = 0; i < d.initial; i++) this.set(i, v);
                }
                Object.defineProperty(Table.prototype, 'length', { get: function() { return __wasm_table_size(this.__id); } });
                Table.prototype.get = function(i) { return toJS(__wasm_table_get(this.__id, Number(i)), this.__el); };
                Table.prototype.set = function(i, v) { __wasm_table_set(this.__id, Number(i), toWasm(v === undefined ? null : v, this.__el)); };
                Table.prototype.grow = function(n, v) { var old = __wasm_table_grow(this.__id, Number(n)); if (v !== undefined) for (var i = old; i < old + n; i++) this.set(i, v); return old; };
                function Global(d, v) {
                    if (!(this instanceof Global)) throw new TypeError('WXWebAssembly.Global 要用 new');
                    var t = d && d.value;
                    if (d && d.__id !== undefined) { Object.defineProperty(this, '__id', { value: d.__id }); t = __wasm_global_type(d.__id)[0]; }
                    else Object.defineProperty(this, '__id', { value: __wasm_global_new(String(t), !!(d && d.mutable), v === undefined ? undefined : toWasm(v, t)) });
                    Object.defineProperty(this, '__t', { value: t });
                }
                Object.defineProperty(Global.prototype, 'value', { get: function() { return toJS(__wasm_global_get(this.__id), this.__t); },
                    set: function(v) { __wasm_global_set(this.__id, toWasm(v, this.__t)); } });
                Global.prototype.valueOf = function() { return this.value; };
                function Instance() { throw new TypeError('WXWebAssembly.Instance 只能由 instantiate 得到'); }

                function instantiateSync(path, imports) {
                    var m = call(function() { return JSON.parse(__wasm_compile(String(path))); });
                    var list = m.imports.map(function(d) {
                        var ns = imports && imports[d.module];
                        if (ns === undefined || ns === null || (typeof ns !== 'object' && typeof ns !== 'function')) throw new TypeError('导入对象里没有模块 "' + d.module + '"');
                        var v = ns[d.name];
                        if (d.kind === 'function') {
                            if (typeof v !== 'function') throw new LinkError('导入 ' + d.module + '.' + d.name + ' 应当是函数');
                            if (v.__fid !== undefined) return ['f', v.__fid];
                            hosts.push(v); hostSig.push(d.type);
                            return ['h', hosts.length - 1];
                        }
                        if (d.kind === 'memory') { if (!(v instanceof Memory)) throw new LinkError('导入 ' + d.module + '.' + d.name + ' 应当是 Memory'); return ['m', v.__id]; }
                        if (d.kind === 'table') { if (!(v instanceof Table)) throw new LinkError('导入 ' + d.module + '.' + d.name + ' 应当是 Table'); return ['t', v.__id]; }
                        if (!(v instanceof Global)) {
                            if (typeof v !== 'number' && typeof v !== 'bigint') throw new LinkError('导入 ' + d.module + '.' + d.name + ' 应当是 Global 或数值');
                            v = new Global({ value: d.value, mutable: false }, v);
                        }
                        return ['g', v.__id];
                    });
                    var ex = call(function() { return JSON.parse(__wasm_instantiate(m.id, list)); });
                    var exports = Object.create(null);
                    ex.forEach(function(e) {
                        var d = m.exports.filter(function(x) { return x.name === e.name; })[0];
                        exports[e.name] = e.kind === 'function' ? fn(e.id, e.name, d && d.type)
                            : e.kind === 'memory' ? new Memory({ __id: e.id })
                            : e.kind === 'table' ? new Table({ __id: e.id, __el: d && d.element === 'externref' ? 'externref' : 'anyfunc', initial: 0 })
                            : new Global({ __id: e.id });
                    });
                    Object.freeze(exports);
                    var inst = Object.create(Instance.prototype);
                    Object.defineProperty(inst, 'exports', { value: exports, enumerable: true });
                    var mod = { imports: m.imports.map(function(d) { return { module: d.module, name: d.name, kind: d.kind }; }),
                        exports: m.exports.map(function(d) { return { name: d.name, kind: d.kind }; }) };
                    return { module: mod, instance: inst };
                }
                var WX = {
                    instantiate: function(path, imports) {
                        try { return Promise.resolve(instantiateSync(path, imports)); } catch (e) { return Promise.reject(e); }
                    },
                    Memory: Memory, Table: Table, Global: Global, Instance: Instance,
                    CompileError: CompileError, LinkError: LinkError, RuntimeError: RuntimeError
                };
                globalThis.WXWebAssembly = WX;
                if (typeof GameGlobal !== 'undefined') GameGlobal.WXWebAssembly = WX;
            })();
