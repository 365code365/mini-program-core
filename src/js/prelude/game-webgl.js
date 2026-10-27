            // 小游戏的 WebGLRenderingContext / WebGL2RenderingContext（game-doc/04 G8，原生在 game/webgl，设计见 game-doc/design-webgl.md）。
            //
            // 原生只有一个分派函数 __game_gl(画布 id, op, ...)，op 编号按 __game_gl_ops() 的名字表查（一份来源，在 Rust）。
            // 这一层管 WebGL 自己的语义：对象包装与归属（别的上下文 / 已删 / 丢失前的对象 → INVALID_OPERATION）、
            // 重载（texImage2D 的 6 / 9 / 10 参数）、ArrayBufferView 与 type 是否匹配、uniform 位置是否属于当前程序、
            // getParameter 返回对象、扩展对象、上下文丢失与恢复。
            (function() {
                if (typeof __game_gl !== 'function') {
                    // 没编进 WebGL（cargo feature webgl 关了）：getContext('webgl') 返回 null，与「设备不支持」同形
                    globalThis.__gameWebGL = function() { return null; };
                    return;
                }
                var G = __game_gl, OP = {};
                __game_gl_ops().forEach(function(n, i) { OP[n] = i; });
                var K1 = __GL_CONSTS1, K2 = __GL_CONSTS2;
                var E_ENUM = 0x500, E_VALUE = 0x501, E_OP = 0x502;
                var byCanvas = {};

                function illegal() { throw new TypeError('Illegal constructor'); }
                function cls(name, base) {
                    var F = function() { illegal(); };
                    Object.defineProperty(F, 'name', { value: name });
                    if (base) { F.prototype = Object.create(base.prototype); F.prototype.constructor = F; }
                    globalThis[name] = F;
                    return F;
                }
                function hide(o, k, v, w) { Object.defineProperty(o, k, { value: v, writable: !!w, enumerable: false, configurable: true }); }
                var WebGLObject = cls('WebGLObject');
                var C = {};
                ['Buffer', 'Framebuffer', 'Program', 'Renderbuffer', 'Shader', 'Texture', 'VertexArrayObject', 'Query', 'Sampler', 'Sync', 'TransformFeedback'].forEach(function(k) {
                    C[k] = cls('WebGL' + k, WebGLObject);
                });
                globalThis.WebGLVertexArrayObjectOES = C.VertexArrayObject;
                var WebGLUniformLocation = cls('WebGLUniformLocation');
                var WebGLActiveInfo = cls('WebGLActiveInfo');
                var WebGLShaderPrecisionFormat = cls('WebGLShaderPrecisionFormat');
                var WebGLContextEvent = cls('WebGLContextEvent');
                function info(F, props) { var o = Object.create(F.prototype); for (var k in props) Object.defineProperty(o, k, { value: props[k], enumerable: true }); return o; }

                var GL1 = cls('WebGLRenderingContext');
                var GL2 = cls('WebGL2RenderingContext');
                Object.keys(K1).forEach(function(k) { GL1[k] = GL1.prototype[k] = GL2[k] = GL2.prototype[k] = K1[k]; });
                Object.keys(K2).forEach(function(k) { GL2[k] = GL2.prototype[k] = K2[k]; });
                var P1 = GL1.prototype, P2 = GL2.prototype;

                // ── 对象 ──
                function wrap(gl, kind, id, extra) {
                    // 丢失期间 create* 也返回对象（新版规范：永远非 null，只是无效 —— 换代号 -1，任何上下文都不认）
                    if ((id === null || id === undefined) && gl.__lost) {
                        var dead = Object.create(C[kind].prototype);
                        hide(dead, '__id', 0); hide(dead, '__ctx', gl); hide(dead, '__gen', -1); hide(dead, '__del', false, true); hide(dead, '__bound', false, true);
                        if (extra) for (var k0 in extra) hide(dead, k0, extra[k0], true);
                        return dead;
                    }
                    if (id === null || id === undefined) return null;
                    var o = Object.create(C[kind].prototype);
                    hide(o, '__id', id); hide(o, '__ctx', gl); hide(o, '__gen', gl.__gen);
                    hide(o, '__del', false, true); hide(o, '__bound', false, true);
                    if (extra) for (var k in extra) hide(o, k, extra[k], true);
                    gl.__objs[kind][id] = o;
                    return o;
                }
                function err(gl, code) { G(gl.__cid, OP._synthError, code); }
                // 对象参数 → GL 名字；null → 0；不属于这个上下文 / 已删 → 记错误返回 -1
                // deletedOk：true = 已删也给名字；'bind' = 绑定类调用（已删 → INVALID_OPERATION）；其余已删 → INVALID_VALUE（Chromium 同款）
                function oid(gl, o, kind, deletedOk) {
                    if (o === null || o === undefined) return 0;
                    if (!(o instanceof C[kind])) throw new TypeError('参数应当是 WebGL' + kind + ' 或 null');
                    if (o.__ctx !== gl || o.__gen !== gl.__gen) { err(gl, E_OP); return -1; }
                    // 标了删除但 GL 里还活着：当前程序、还挂在程序上的着色器（真正的删除推迟到不再使用）
                    var dead = o.__del && !(kind === 'Program' && o === gl.__program) && !(kind === 'Shader' && o.__attached > 0);
                    if (dead && deletedOk !== true) { err(gl, deletedOk === 'bind' ? E_OP : E_VALUE); return -1; }
                    return o.__id;
                }
                // 不可为 null 的对象参数（WebIDL 里没有 ?）：null / undefined 抛 TypeError
                function req(gl, o, kind, deletedOk) {
                    if (o === null || o === undefined) throw new TypeError('参数应当是 WebGL' + kind + '（不能是 null）');
                    return oid(gl, o, kind, deletedOk);
                }
                function live(gl, o, kind) { return o instanceof C[kind] && o.__ctx === gl && o.__gen === gl.__gen && !o.__del; }
                function lookup(gl, kind, id) { return id ? (gl.__objs[kind][id] || null) : null; }

                // getParameter 里返回对象的 pname → 种类
                var OBJ_PARAM = { 0x8894: 'Buffer', 0x8895: 'Buffer', 0x8F36: 'Buffer', 0x8F37: 'Buffer', 0x88ED: 'Buffer', 0x88EF: 'Buffer',
                    0x8C8F: 'Buffer', 0x8A28: 'Buffer', 0x8B8D: 'Program', 0x8CA6: 'Framebuffer', 0x8CAA: 'Framebuffer', 0x8CA7: 'Renderbuffer',
                    0x8069: 'Texture', 0x8514: 'Texture', 0x8C1D: 'Texture', 0x806A: 'Texture', 0x85B5: 'VertexArrayObject', 0x8919: 'Sampler',
                    0x8E25: 'TransformFeedback' };

                // ArrayBufferView 的元素类型必须与 type 对得上（WebGL §5.14.8）
                function viewOk(v, type) {
                    switch (type) {
                        case 0x1401: return v instanceof Uint8Array || v instanceof Uint8ClampedArray;
                        case 0x1400: return v instanceof Int8Array;
                        case 0x1403: case 0x8363: case 0x8033: case 0x8034: case 0x140B: case 0x8D61: return v instanceof Uint16Array;
                        case 0x1402: return v instanceof Int16Array;
                        case 0x1405: case 0x8368: case 0x8C3B: case 0x8C3E: case 0x84FA: return v instanceof Uint32Array;
                        case 0x1404: return v instanceof Int32Array;
                        case 0x1406: return v instanceof Float32Array;
                        default: return false;
                    }
                }
                function pixView(gl, v, type, off) {
                    if (v === null || v === undefined) return null;
                    if (!ArrayBuffer.isView(v)) throw new TypeError('像素数据应当是 ArrayBufferView');
                    if (!viewOk(v, type)) { err(gl, E_OP); return undefined; }
                    return off ? v.subarray(off) : v;
                }
                function typed(T, v, off, len) {
                    if (!(v instanceof T)) {
                        if (v === null || v === undefined || typeof v.length !== 'number') throw new TypeError('参数应当是 ' + T.name + ' 或数组');
                        v = new T(v);
                    }
                    if (off || len) v = v.subarray(off || 0, len ? (off || 0) + len : undefined);
                    return v;
                }
                // 纹理来源：[种类, id | 宽, 高, 数据]；没加载完的 Image 返回 'skip'（与浏览器一样什么都不传）
                function srcArgs(s) {
                    if (s && typeof __GameImage === 'function' && s instanceof __GameImage) {
                        if (!(s.complete && s.width > 0)) return 'skip';
                        return [1, s.__id];
                    }
                    if (s && typeof __GameCanvas === 'function' && s instanceof __GameCanvas) {
                        if (s.__glctx) __game_gl_sync(s.__id);
                        return [2, s.__id];
                    }
                    if (s && s.data && typeof s.width === 'number' && typeof s.height === 'number') return [3, s.width, s.height, s.data];
                    // 小游戏里没有 <video> 当纹理源（wx.createVideo 是原生层）：与浏览器里「视频还没有帧」一样什么都不传
                    if (s && s.tagName === 'VIDEO') return 'skip';
                    throw new TypeError('纹理来源应当是 Image / Canvas / ImageData / ArrayBufferView');
                }

                // ── 原样转发的方法 ──
                function fwd(P, names) {
                    names.forEach(function(n) {
                        var op = OP[n];
                        if (op === undefined) throw new Error('game-webgl: 原生没有 ' + n);
                        P[n] = function(a, b, c, d, e, f, g, h, i, j) { return G(this.__cid, op, a, b, c, d, e, f, g, h, i, j); };
                    });
                }
                var SIMPLE1 = ['activeTexture', 'blendColor', 'blendEquation', 'blendEquationSeparate', 'blendFunc', 'blendFuncSeparate', 'clearColor',
                    'clearDepth', 'clearStencil', 'colorMask', 'cullFace', 'depthFunc', 'depthMask', 'depthRange', 'disable', 'enable', 'frontFace', 'hint',
                    'isEnabled', 'lineWidth', 'pixelStorei', 'polygonOffset', 'sampleCoverage', 'scissor', 'stencilFunc', 'stencilFuncSeparate',
                    'stencilMask', 'stencilMaskSeparate', 'stencilOp', 'stencilOpSeparate', 'viewport', 'clear', 'finish', 'flush', 'getError',
                    'getBufferParameter', 'enableVertexAttribArray', 'disableVertexAttribArray', 'vertexAttrib1f', 'vertexAttrib2f', 'vertexAttrib3f',
                    'vertexAttrib4f', 'drawArrays', 'drawElements', 'texParameterf', 'texParameteri', 'getTexParameter', 'generateMipmap', 'copyTexImage2D',
                    'copyTexSubImage2D', 'checkFramebufferStatus', 'renderbufferStorage', 'getRenderbufferParameter'];
                var SIMPLE2 = ['vertexAttribIPointer', 'vertexAttribI4i', 'vertexAttribI4ui', 'drawArraysInstanced', 'drawElementsInstanced',
                    'drawRangeElements', 'vertexAttribDivisor', 'copyTexSubImage3D', 'texStorage2D', 'texStorage3D', 'renderbufferStorageMultisample',
                    'blitFramebuffer', 'readBuffer', 'getInternalformatParameter', 'clearBufferfi', 'copyBufferSubData', 'beginTransformFeedback',
                    'endTransformFeedback', 'pauseTransformFeedback', 'resumeTransformFeedback'];
                [P1, P2].forEach(function(P) { fwd(P, SIMPLE1); });
                fwd(P2, SIMPLE2);

                // 两个版本共有的、要处理对象 / 重载的方法（下面逐个定义在 B 上，再拷到两个原型）
                var B = {};
                function create(kind, op) { return function() { return wrap(this, kind, G(this.__cid, OP[op])); }; }
                function del(kind, op, after) {
                    return function(o) {
                        var id = oid(this, o, kind, true);
                        if (id <= 0 || o.__del) return;
                        G(this.__cid, OP[op], id);
                        o.__del = true;
                        // 不从表里删：还挂在帧缓冲上 / 还是当前程序的已删对象，查询仍要返回它（GL 复用名字时 wrap 覆盖）
                        if (after) after.call(this, o);
                    };
                }
                function isFn(kind, needBound) { return function(o) { return !this.__lost && live(this, o, kind) && (!needBound || o.__bound); }; }
                B.createBuffer = create('Buffer', 'createBuffer');
                B.createFramebuffer = create('Framebuffer', 'createFramebuffer');
                B.createRenderbuffer = create('Renderbuffer', 'createRenderbuffer');
                B.createTexture = create('Texture', 'createTexture');
                B.createProgram = function() { return wrap(this, 'Program', G(this.__cid, OP.createProgram), { __shaders: [], __link: 0 }); };
                B.createShader = function(type) { return wrap(this, 'Shader', G(this.__cid, OP.createShader, type), { __type: type, __src: '', __attached: 0 }); };
                B.deleteBuffer = del('Buffer', 'deleteBuffer');
                B.deleteFramebuffer = del('Framebuffer', 'deleteFramebuffer');
                // 删掉还挂在当前帧缓冲上的渲染缓冲 / 纹理：GL 从当前帧缓冲上摘掉它
                function detachDeleted(gl, o) {
                    var f = gl.__fbo; if (!f || !f.__att) return;
                    Object.keys(f.__att).forEach(function(k) { var x = f.__att[k]; if (x === o || (x && x.__tex === o)) f.__att[k] = null; });
                    fbDirty(gl);
                }
                B.deleteRenderbuffer = del('Renderbuffer', 'deleteRenderbuffer', function(r) { detachDeleted(this, r); if (this.__rbo === r) this.__rbo = null; });
                // 删掉的纹理从各个单元上解绑（GL 也是这么做的；之后在这个目标上调 texParameter → 没绑纹理）
                B.deleteTexture = del('Texture', 'deleteTexture', function(x) {
                    detachDeleted(this, x);
                    var u = this.__units; Object.keys(u).forEach(function(k) { Object.keys(u[k]).forEach(function(t) { if (u[k][t] === x) u[k][t] = null; }); });
                });
                B.deleteShader = del('Shader', 'deleteShader');
                // 删掉当前程序：GL 里它还在用（CURRENT_PROGRAM 照常返回它），直到 useProgram 换掉
                B.deleteProgram = del('Program', 'deleteProgram', function(p) { p.__shaders.forEach(function(s) { s.__attached--; }); });
                B.isBuffer = isFn('Buffer', true);
                B.isFramebuffer = isFn('Framebuffer', true);
                B.isRenderbuffer = isFn('Renderbuffer', true);
                B.isTexture = isFn('Texture', true);
                // 删掉但还挂在程序上的着色器：GL 里还没真删，isShader 仍是 true
                B.isShader = function(o) { return !this.__lost && o instanceof C.Shader && o.__ctx === this && o.__gen === this.__gen && (!o.__del || o.__attached > 0); };
                B.isProgram = function(o) { return !this.__lost && o instanceof C.Program && o.__ctx === this && o.__gen === this.__gen && (!o.__del || o === this.__program); };
                B.bindBuffer = function(t, b) {
                    var id = oid(this, b, 'Buffer', 'bind'); if (id < 0) return;
                    // WebGL §5.1：绑过 ELEMENT_ARRAY_BUFFER 的缓冲不能再绑到别的目标，反之亦然
                    if (b) {
                        var el = t === 0x8893;
                        if (b.__el !== undefined && b.__el !== el && t !== 0x8F36 && t !== 0x8F37) { err(this, E_OP); return; }
                        if (t !== 0x8F36 && t !== 0x8F37) b.__el = el;
                    }
                    G(this.__cid, OP.bindBuffer, t, id);
                    if (b) b.__bound = true;
                };
                B.bindFramebuffer = function(t, f) { if (!fbTarget(this, t)) return; var id = oid(this, f, 'Framebuffer', 'bind'); if (id < 0) return;
                    if (t !== 0x8CA8) { this.__fbo = f || null; fbDirty(this); } G(this.__cid, OP.bindFramebuffer, t, id); if (f) f.__bound = true; };
                B.renderbufferStorage = function(t, f, w, h) {
                    G(this.__cid, OP.renderbufferStorage, t, f, w, h);
                    var r = this.__rbo; if (r) { r.__fmt = f; r.__w = w; r.__h = h; }
                    fbDirty(this);
                };
                B.bindRenderbuffer = function(t, r) {
                    this.__rbo = r || null; var id = oid(this, r, 'Renderbuffer', 'bind'); if (id < 0) return; G(this.__cid, OP.bindRenderbuffer, t, id); if (r) r.__bound = true; };
                // 纹理绑定按单元记一份：没绑纹理就 texImage / texParameter … → INVALID_OPERATION（GL 的默认纹理 0 在 WebGL 里不存在）
                var CUBE_FACES = [0x8515, 0x8516, 0x8517, 0x8518, 0x8519, 0x851A];
                // image = true：texImage 一类（目标是 TEXTURE_2D 或立方体的某个面）；否则 texParameter 一类（TEXTURE_2D / TEXTURE_CUBE_MAP）
                function texBound(gl, t, image) {
                    var v2 = gl.__v === 2 && (t === 0x806F || t === 0x8C1A);
                    var okT = image ? (t === 0x0DE1 || CUBE_FACES.indexOf(t) >= 0) : (t === 0x0DE1 || t === 0x8513 || v2);
                    if (!okT) { err(gl, E_ENUM); return false; }
                    var k = CUBE_FACES.indexOf(t) >= 0 ? 0x8513 : t;
                    var u = gl.__units[gl.__unit] || {};
                    if (u[k]) return true;
                    err(gl, E_OP); return false;
                }
                B.activeTexture = function(u) { G(this.__cid, OP.activeTexture, u); this.__unit = u - 0x84C0; };
                B.bindTexture = function(t, x) {
                    if (!(t === 0x0DE1 || t === 0x8513 || (this.__v === 2 && (t === 0x806F || t === 0x8C1A)))) { err(this, E_ENUM); return; }
                    var id = oid(this, x, 'Texture', 'bind'); if (id < 0) return;
                    // 纹理第一次绑到哪个目标就一直是那个目标
                    if (x) { if (x.__target !== undefined && x.__target !== t) { err(this, E_OP); return; } x.__target = t; }
                    G(this.__cid, OP.bindTexture, t, id);
                    if (x) x.__bound = true;
                    (this.__units[this.__unit] = this.__units[this.__unit] || {})[t] = x || null;
                };
                B.attachShader = function(p, s) {
                    var pi = req(this, p, 'Program'), si = req(this, s, 'Shader'); if (pi <= 0 || si <= 0) return;
                    if (p.__shaders.some(function(x) { return x === s || x.__type === s.__type; })) { err(this, E_OP); return; }
                    G(this.__cid, OP.attachShader, pi, si); p.__shaders.push(s); s.__attached++;
                };
                B.detachShader = function(p, s) {
                    var pi = req(this, p, 'Program'), si = req(this, s, 'Shader', true); if (pi <= 0 || si <= 0) return;
                    var i = p.__shaders.indexOf(s); if (i < 0) { err(this, E_OP); return; }
                    G(this.__cid, OP.detachShader, pi, si); p.__shaders.splice(i, 1); s.__attached--;
                };
                B.getAttachedShaders = function(p) { return req(this, p, 'Program') > 0 && !this.__lost ? p.__shaders.slice() : null; };
                B.shaderSource = function(s, src) { var id = req(this, s, 'Shader'); if (id <= 0) return; s.__src = String(src); G(this.__cid, OP.shaderSource, id, s.__src); };
                B.getShaderSource = function(s) { return req(this, s, 'Shader') > 0 && !this.__lost ? s.__src : null; };
                B.compileShader = function(s) { var id = req(this, s, 'Shader'); if (id > 0) G(this.__cid, OP.compileShader, id); };
                B.getShaderParameter = function(s, p) {
                    var id = req(this, s, 'Shader', true); if (id <= 0 || this.__lost) return null;
                    if (p === 0x8B4F) return s.__type; // SHADER_TYPE
                    if (p === 0x8B80) return s.__del;  // DELETE_STATUS
                    return G(this.__cid, OP.getShaderParameter, id, p);
                };
                B.getShaderInfoLog = function(s) { var id = req(this, s, 'Shader', true); return id > 0 ? G(this.__cid, OP.getShaderInfoLog, id) : null; };
                B.linkProgram = function(p) {
                    var id = req(this, p, 'Program'); if (id <= 0) return;
                    p.__link++; G(this.__cid, OP.linkProgram, id);
                    p.__linked = !!G(this.__cid, OP.getProgramParameter, id, 0x8B82);
                };
                // getUniformLocation / getAttribLocation 要程序链接成功过（WebGL：INVALID_OPERATION）
                function linked(gl, p) { if (p.__linked) return true; err(gl, E_OP); return false; }
                B.validateProgram = function(p) { var id = req(this, p, 'Program'); if (id > 0) G(this.__cid, OP.validateProgram, id); };
                B.useProgram = function(p) { var id = oid(this, p, 'Program', 'bind'); if (id < 0) return; G(this.__cid, OP.useProgram, id); this.__program = p || null; };
                B.getProgramParameter = function(p, n) {
                    var id = req(this, p, 'Program', true); if (id <= 0 || this.__lost) return null;
                    if (n === 0x8B80) return p.__del;
                    return G(this.__cid, OP.getProgramParameter, id, n);
                };
                B.getProgramInfoLog = function(p) { var id = req(this, p, 'Program', true); return id > 0 ? G(this.__cid, OP.getProgramInfoLog, id) : null; };
                function active(op) {
                    return function(p, i) {
                        var id = req(this, p, 'Program'); if (id <= 0) return null;
                        var r = G(this.__cid, OP[op], id, i);
                        // WebGL：数组 uniform 的名字带 [0]
                        if (r && op === 'getActiveUniform' && r.size > 1 && !/\]$/.test(r.name)) r.name += '[0]';
                        return r ? info(WebGLActiveInfo, r) : null;
                    };
                }
                B.getActiveAttrib = active('getActiveAttrib');
                B.getActiveUniform = active('getActiveUniform');
                // WebGL §6.18 / §6.21：名字只能用 GLSL ES 的字符集，长度 ≤ 256；webgl_ / _webgl_ 前缀是保留的
                // （bindAttribLocation → INVALID_OPERATION；getXxxLocation 只是查不到，不报错）
                var BAD_CHAR = /[^\x09-\x0d\x20-\x7e]|["$'@\\`]/;
                function nameOk(gl, name, bind) {
                    name = String(name);
                    if (name.length > 256 || BAD_CHAR.test(name)) { err(gl, E_VALUE); return null; }
                    if (/^_?webgl_/.test(name)) { if (bind) err(gl, E_OP); return null; }
                    return name;
                }
                B.getAttribLocation = function(p, name) { var id = req(this, p, 'Program'); if (id <= 0 || !linked(this, p)) return -1; name = nameOk(this, name); return name === null ? -1 : G(this.__cid, OP.getAttribLocation, id, name); };
                B.bindAttribLocation = function(p, i, name) { var id = req(this, p, 'Program'); if (id <= 0) return; name = nameOk(this, name, true); if (name !== null) G(this.__cid, OP.bindAttribLocation, id, i, name); };
                B.getUniformLocation = function(p, name) {
                    var id = req(this, p, 'Program'); if (id <= 0 || !linked(this, p)) return null;
                    name = nameOk(this, name); if (name === null) return null;
                    var l = G(this.__cid, OP.getUniformLocation, id, name);
                    if (l === null || l === undefined) return null;
                    var o = Object.create(WebGLUniformLocation.prototype);
                    hide(o, '__p', p); hide(o, '__l', l); hide(o, '__name', name); hide(o, '__link', p.__link); hide(o, '__gen', this.__gen);
                    return o;
                };
                // uniform 位置 → 整数；null 静默忽略（返回 null）；不属于当前程序 / 程序重新链接过 → INVALID_OPERATION
                function loc(gl, l) {
                    if (l === null || l === undefined) return null;
                    if (!(l instanceof WebGLUniformLocation)) throw new TypeError('参数应当是 WebGLUniformLocation');
                    if (l.__gen !== gl.__gen || l.__p !== gl.__program || l.__link !== l.__p.__link) { err(gl, E_OP); return null; }
                    return l.__l;
                }
                B.getUniform = function(p, l) {
                    var id = req(this, p, 'Program'); if (id <= 0) return null;
                    if (!(l instanceof WebGLUniformLocation)) throw new TypeError('getUniform：第二个参数应当是 WebGLUniformLocation');
                    if (l.__p !== p || l.__link !== p.__link) { err(this, E_OP); return null; }
                    return G(this.__cid, OP.getUniform, id, l.__l, l.__name);
                };
                ['1f', '2f', '3f', '4f', '1i', '2i', '3i', '4i'].forEach(function(s) {
                    var op = OP['uniform' + s];
                    B['uniform' + s] = function(l, x, y, z, w) { var n = loc(this, l); if (n !== null) G(this.__cid, op, n, x, y, z, w); };
                });
                function uv(T, op, n) { return function(l, v, off, len) { var k = loc(this, l); if (k !== null) G(this.__cid, op, k, n, typed(T, v, off, len)); }; }
                [1, 2, 3, 4].forEach(function(n) {
                    B['uniform' + n + 'fv'] = uv(Float32Array, OP._uniformfv, n);
                    B['uniform' + n + 'iv'] = uv(Int32Array, OP._uniformiv, n);
                });
                function um(c, r) { return function(l, tr, v, off, len) { var k = loc(this, l); if (k !== null) G(this.__cid, OP._uniformMatrix, k, c, r, tr, typed(Float32Array, v, off, len)); }; }
                B.uniformMatrix2fv = um(2, 2); B.uniformMatrix3fv = um(3, 3); B.uniformMatrix4fv = um(4, 4);
                [1, 2, 3, 4].forEach(function(n) {
                    B['vertexAttrib' + n + 'fv'] = function(i, v) { G(this.__cid, OP._vertexAttribfv, i, n, typed(Float32Array, v)); };
                });
                // 顶点属性：偏移记在当前 VAO 上（getVertexAttribOffset 要读回）
                var VA1 = [0x1400, 0x1401, 0x1402, 0x1403, 0x1406], VA2 = VA1.concat([0x1404, 0x1405, 0x140B, 0x8D9F, 0x8368]);
                B.vertexAttribPointer = function(i, size, type, norm, stride, off) {
                    if ((this.__v === 2 ? VA2 : VA1).indexOf(type) < 0) { err(this, E_ENUM); return; }
                    if (!(size >= 1 && size <= 4) && !(this.__v === 2 && size === 0x80E1 && (type === 0x1401 || type === 0x8368))) { err(this, E_VALUE); return; }
                    G(this.__cid, OP.vertexAttribPointer, i, size, type, norm, stride, off);
                    (this.__vao || this.__defVao)[i] = off;
                };
                B.getVertexAttribOffset = function(i, p) { if (p !== 0x8645) { err(this, E_ENUM); return 0; } var v = (this.__vao || this.__defVao)[i]; return v === undefined ? 0 : v; };
                B.getVertexAttrib = function(i, p) {
                    var r = G(this.__cid, OP.getVertexAttrib, i, p);
                    return p === 0x889F ? lookup(this, 'Buffer', r) : r;
                };
                // WebGL1 只有 FRAMEBUFFER 一个目标（READ / DRAW_FRAMEBUFFER 是 WebGL2）
                function attOk(gl, a) {
                    if (a === 0x8CE0 || a === 0x8D00 || a === 0x8D20 || a === 0x821A) return true;
                    var max = gl.__v === 2 || gl.__ext.webgl_draw_buffers ? 16 : 1;
                    if (a > 0x8CE0 && a < 0x8CE0 + max) return true;
                    err(gl, E_ENUM); return false;
                }
                function fbTarget(gl, t) {
                    if (t === 0x8D40 || (gl.__v === 2 && (t === 0x8CA8 || t === 0x8CA9))) return true;
                    err(gl, E_ENUM); return false;
                }
                // WebGL1 §6.6 在 GL 之上多出来的完整性规则（Chromium WebGLFramebuffer::CheckStatus 同款）：深度 / 模板挂点的格式必须对、
                // 深度 / 模板 / 深度模板三者只能挂一个、渲染缓冲尺寸必须一致
                function fbExtra(gl) {
                    var f = gl.__fbo;
                    if (gl.__v === 1 && f && f.__att) {
                        var A = f.__att, ds = 0, size = null, bad = false;
                        var want = { 0x8D00: 0x81A5, 0x8D20: 0x8D48, 0x821A: 0x84F9 };
                        Object.keys(A).forEach(function(k) {
                            var r = A[k]; if (!r) return; k = Number(k);
                            if (want[k]) ds++;
                            if (r instanceof C.Renderbuffer) {
                                if (want[k] && r.__fmt !== want[k]) bad = true;
                                if (r.__w === 0 || r.__h === 0) bad = true;
                                var sz = r.__w + 'x' + r.__h; if (size === null) size = sz; else if (size !== sz) size = 'mismatch';
                            }
                        });
                        if (bad) return 0x8CD6;
                        if (ds > 1) return 0x8CDD;
                        if (size === 'mismatch') return 0x8CD9;
                    }
                    return 0;
                }
                // 挂点 / 存储 / 绑定一变就重算：画 / 清 / 读的热路径只看 __fbBad 一个标志（不完整 → INVALID_FRAMEBUFFER_OPERATION）
                function fbDirty(gl) { gl.__fbBad = fbExtra(gl) !== 0; }
                B.checkFramebufferStatus = function(t) {
                    if (!fbTarget(this, t)) return 0;
                    return fbExtra(this) || G(this.__cid, OP.checkFramebufferStatus, t);
                };
                ['clear', 'drawArrays', 'drawElements'].forEach(function(n) {
                    var op = OP[n];
                    B[n] = function(a, b, c, d) { if (this.__fbBad) { err(this, 0x506); return; } return G(this.__cid, op, a, b, c, d); };
                });
                B.framebufferTexture2D = function(t, a, tt, x, lv) { if (!fbTarget(this, t) || !attOk(this, a)) return;
                    if (this.__v === 1 && lv !== 0) { err(this, E_VALUE); return; }
                    if (this.__fbo) { this.__fbo.__att = this.__fbo.__att || {}; this.__fbo.__att[a] = x ? { __tex: x } : null; fbDirty(this); } var id = oid(this, x, 'Texture'); if (id >= 0) G(this.__cid, OP.framebufferTexture2D, t, a, tt, id, lv); };
                B.framebufferRenderbuffer = function(t, a, rt, r) { if (!fbTarget(this, t) || !attOk(this, a)) return; var id = oid(this, r, 'Renderbuffer');
                    if (id >= 0 && this.__fbo) { this.__fbo.__att = this.__fbo.__att || {}; this.__fbo.__att[a] = r || null; fbDirty(this); } if (id >= 0) G(this.__cid, OP.framebufferRenderbuffer, t, a, rt, id); };
                B.getFramebufferAttachmentParameter = function(t, a, p) {
                    if (!fbTarget(this, t) || !attOk(this, a)) return null;
                    if (this.__v === 1) {
                        var okp = p === 0x8CD0 || p === 0x8CD1 || p === 0x8CD2 || p === 0x8CD3 || (p === 0x8210 && this.__ext.ext_srgb)
                            || (p === 0x8211 && (this.__ext.ext_color_buffer_half_float || this.__ext.webgl_color_buffer_float));
                        if (!okp) { err(this, E_ENUM); return null; }
                        // 没挂东西的挂点：只能问 OBJECT_TYPE（WebGL1）
                        if (p !== 0x8CD0 && G(this.__cid, OP.getFramebufferAttachmentParameter, t, a, 0x8CD0) === 0) { err(this, E_ENUM); return null; }
                    }
                    if (p === 0x8CD1) { // FRAMEBUFFER_ATTACHMENT_OBJECT_NAME → 对象
                        var ty = G(this.__cid, OP.getFramebufferAttachmentParameter, t, a, 0x8CD0);
                        var n = G(this.__cid, OP.getFramebufferAttachmentParameter, t, a, p);
                        return ty === 0x1702 ? lookup(this, 'Texture', n) : ty === 0x8D41 ? lookup(this, 'Renderbuffer', n) : null;
                    }
                    return G(this.__cid, OP.getFramebufferAttachmentParameter, t, a, p);
                };
                // blendEquation：MIN / MAX 要 WebGL2 或 EXT_blend_minmax
                function eqOk(gl, m) {
                    if (m === 0x8006 || m === 0x800A || m === 0x800B) return true;
                    if ((m === 0x8007 || m === 0x8008) && (gl.__v === 2 || gl.__ext.ext_blend_minmax)) return true;
                    err(gl, E_ENUM); return false;
                }
                B.blendEquation = function(m) { if (eqOk(this, m)) G(this.__cid, OP.blendEquation, m); };
                B.blendEquationSeparate = function(a, b) { if (eqOk(this, a) && eqOk(this, b)) G(this.__cid, OP.blendEquationSeparate, a, b); };
                // hint：FRAGMENT_SHADER_DERIVATIVE_HINT 要 WebGL2 或 OES_standard_derivatives
                B.hint = function(t, m) {
                    if (t !== 0x8192 && !(t === 0x8B8B && (this.__v === 2 || this.__ext.oes_standard_derivatives))) { err(this, E_ENUM); return; }
                    G(this.__cid, OP.hint, t, m);
                };
                var USAGE1 = [0x88E0, 0x88E4, 0x88E8], USAGE2 = USAGE1.concat([0x88E1, 0x88E2, 0x88E5, 0x88E6, 0x88E9, 0x88EA]);
                var BT1 = [0x8892, 0x8893], BT2 = BT1.concat([0x8F36, 0x8F37, 0x88EB, 0x88EC, 0x8C8E, 0x8A11]);
                B.getBufferParameter = function(t, p) {
                    if ((this.__v === 2 ? BT2 : BT1).indexOf(t) < 0) { err(this, E_ENUM); return null; }
                    return G(this.__cid, OP.getBufferParameter, t, p);
                };
                B.bufferData = function(t, d, usage, off, len) {
                    if ((this.__v === 2 ? USAGE2 : USAGE1).indexOf(usage) < 0) { err(this, E_ENUM); return; }
                    if (d === null || d === undefined) { err(this, E_VALUE); return; }
                    // WebIDL (BufferSource or GLsizeiptr)：不是缓冲的一律 ToNumber（字符串 / 普通对象 → NaN → 0）
                    if (!(d instanceof ArrayBuffer || ArrayBuffer.isView(d) || (typeof SharedArrayBuffer === 'function' && d instanceof SharedArrayBuffer))) {
                        var n = Number(d); n = isFinite(n) ? Math.trunc(n) : 0;
                        return G(this.__cid, OP._bufferDataSize, t, n, usage);
                    }
                    if (off !== undefined && ArrayBuffer.isView(d)) d = d.subarray(off, len ? off + len : undefined);
                    G(this.__cid, OP.bufferData, t, d, usage);
                };
                B.bufferSubData = function(t, o, d, off, len) {
                    if (!(d instanceof ArrayBuffer || ArrayBuffer.isView(d) || (typeof SharedArrayBuffer === 'function' && d instanceof SharedArrayBuffer))) throw new TypeError('bufferSubData：数据应当是 BufferSource');
                    if (off !== undefined && ArrayBuffer.isView(d)) d = d.subarray(off, len ? off + len : undefined);
                    G(this.__cid, OP.bufferSubData, t, o, d);
                };
                B.readPixels = function(x, y, w, h, f, ty, dst, off) {
                    if (this.__fbBad) { err(this, 0x506); return; }
                    if (typeof dst === 'number') { if (this.__v < 2) throw new TypeError('readPixels: WebGL1 不支持 PBO 偏移'); return G(this.__cid, OP._readPixels, x, y, w, h, f, ty, dst); }
                    if (dst === null || dst === undefined) { err(this, E_VALUE); return; }
                    var v = pixView(this, dst, ty, off); if (v === undefined) return;
                    G(this.__cid, OP._readPixels, x, y, w, h, f, ty, v);
                };
                ['texParameterf', 'texParameteri', 'generateMipmap', 'copyTexImage2D', 'copyTexSubImage2D', 'getTexParameter'].forEach(function(n) {
                    var op = OP[n], image = n.indexOf('copyTex') === 0, reads = image;
                    B[n] = function(t, a, b, c, d, e, f, g) {
                        if (!texBound(this, t, image)) return n === 'getTexParameter' ? null : undefined;
                        if (reads && this.__fbBad) { err(this, 0x506); return; }
                        return G(this.__cid, op, t, a, b, c, d, e, f, g);
                    };
                });
                // WebGL1 的 texImage type：扩展开了才认 FLOAT / HALF_FLOAT_OES / 深度纹理那几种
                function texType(gl, ty) {
                    if (gl.__v === 2) return true;
                    var x = gl.__ext;
                    if (ty === 0x1401 || ty === 0x8363 || ty === 0x8033 || ty === 0x8034) return true;
                    if (ty === 0x1406 && x.oes_texture_float) return true;
                    if (ty === 0x8D61 && x.oes_texture_half_float) return true;
                    if ((ty === 0x1403 || ty === 0x1405 || ty === 0x84FA) && x.webgl_depth_texture) return true;
                    err(gl, E_ENUM); return false;
                }
                B.texImage2D = function(t, lv, ifmt, a, b, c, d, e, f, g) {
                    if (!texBound(this, t, true)) return;
                    if (!texType(this, arguments.length === 6 ? b : e)) return;
                    if (arguments.length === 6) return texSrc(this, OP._texImage2DSource, [t, lv, ifmt, a, b], c);
                    // (t, lv, ifmt, w, h, border, format, type, 像素 | PBO 偏移 | 来源 [, srcOffset])
                    if (typeof f === 'number') return G(this.__cid, OP._texImage2D, t, lv, ifmt, a, b, c, d, e, f);
                    if (f && typeof f === 'object' && !ArrayBuffer.isView(f)) return texSrc(this, OP._texImage2DSource, [t, lv, ifmt, d, e], f);
                    var v = pixView(this, f, e, g); if (v === undefined) return;
                    G(this.__cid, OP._texImage2D, t, lv, ifmt, a, b, c, d, e, v);
                };
                B.texSubImage2D = function(t, lv, x, y, a, b, c, d, e, f) {
                    if (!texBound(this, t, true)) return;
                    if (!texType(this, arguments.length === 7 ? b : d)) return;
                    if (arguments.length === 7) return texSrc(this, OP._texSubImage2DSource, [t, lv, x, y, a, b], c);
                    if (typeof e === 'number') return G(this.__cid, OP._texSubImage2D, t, lv, x, y, a, b, c, d, e);
                    if (e && typeof e === 'object' && !ArrayBuffer.isView(e)) return texSrc(this, OP._texSubImage2DSource, [t, lv, x, y, c, d], e);
                    var v = pixView(this, e, d, f); if (v === undefined) return;
                    if (v === null) { err(this, E_VALUE); return; }
                    G(this.__cid, OP._texSubImage2D, t, lv, x, y, a, b, c, d, v);
                };
                function texSrc(gl, op, head, s) {
                    var a = srcArgs(s);
                    if (a === 'skip') return;
                    G.apply(null, [gl.__cid, op].concat(head, a));
                }
                B.compressedTexImage2D = function(t, lv, ifmt, w, h, b, d, off, len) {
                    if (!texBound(this, t, true)) return;
                    if (!ArrayBuffer.isView(d)) throw new TypeError('compressedTexImage2D: 数据应当是 ArrayBufferView');
                    if (off || len) d = d.subarray(off || 0, len ? (off || 0) + len : undefined);
                    G(this.__cid, OP._compressedTexImage2D, t, lv, ifmt, w, h, b, d);
                };
                B.compressedTexSubImage2D = function(t, lv, x, y, w, h, f, d, off, len) {
                    if (!ArrayBuffer.isView(d)) throw new TypeError('compressedTexSubImage2D: 数据应当是 ArrayBufferView');
                    if (off || len) d = d.subarray(off || 0, len ? (off || 0) + len : undefined);
                    G(this.__cid, OP._compressedTexSubImage2D, t, lv, x, y, w, h, f, d);
                };
                B.getShaderPrecisionFormat = function(st, pt) {
                    var r = G(this.__cid, OP.getShaderPrecisionFormat, st, pt);
                    return r ? info(WebGLShaderPrecisionFormat, { rangeMin: r[0], rangeMax: r[1], precision: r[2] }) : null;
                };
                var STR = { 0x1F00: 'WebKit', 0x1F01: 'WebKit WebGL' };
                B.getParameter = function(p) {
                    if (this.__lost) return null;
                    if (STR[p]) return STR[p];
                    if (p === 0x1F02) return this.__v === 2 ? 'WebGL 2.0 (OpenGL ES 3.0 Chromium)' : 'WebGL 1.0 (OpenGL ES 2.0 Chromium)';
                    if (p === 0x8B8C) return this.__v === 2 ? 'WebGL GLSL ES 3.00 (OpenGL ES GLSL ES 3.0 Chromium)' : 'WebGL GLSL ES 1.0 (OpenGL ES GLSL ES 1.0 Chromium)';
                    if (p === 0x8B8D) return this.__program;
                    var r = G(this.__cid, OP.getParameter, p);
                    var k = OBJ_PARAM[p];
                    return k ? lookup(this, k, r) : r;
                };
                // WEBGL_lose_context 在丢失状态下误用：GL 那边不再产生错误，这一条由这里报
                B.getError = function() { if (this.__pendingLostErr) { this.__pendingLostErr = false; return E_OP; } return G(this.__cid, OP.getError); };
                B.getSupportedExtensions = function() { return this.__lost ? null : G(this.__cid, OP.getSupportedExtensions); };
                B.getExtension = function(name) {
                    if (this.__lost) return null;
                    name = String(name);
                    var key = name.toLowerCase();
                    if (this.__ext[key]) return this.__ext[key];
                    var make = EXT[EXT_KEYS[key]];
                    if (!make || !G(this.__cid, OP._getExtension, EXT_KEYS[key])) return null;
                    return (this.__ext[key] = make(this));
                };
                B.getContextAttributes = function() {
                    if (this.__lost) return null;
                    var a = {}, s = this.__attrs; for (var k in s) if (k.charAt(0) !== '_') a[k] = s[k];
                    return a;
                };
                B.isContextLost = function() { return this.__lost; };
                B.makeXRCompatible = function() { return Promise.resolve(); };
                // 微信扩展（官方文档：WebGLRenderingContext.wxBindCanvasTexture）：把一块画布当前的像素传进当前绑着的纹理。
                // 微信是零拷贝共享；这里是调用那一刻拷一份（之后画布再变要再调一次）
                B.wxBindCanvasTexture = function(t, canvas) { texSrc(this, OP._texImage2DSource, [t, 0, 0x1908, 0x1908, 0x1401], canvas); };
                // 微信扩展 wxBindCanvasTexture 不进 for-in（CTS 的 methods 用例按可枚举属性对照标准方法表）
                Object.keys(B).forEach(function(k) { [P1, P2].forEach(function(P) { Object.defineProperty(P, k, { value: B[k], writable: true, configurable: true, enumerable: k !== 'wxBindCanvasTexture' }); }); });
                [P1, P2].forEach(function(P) {
                    Object.defineProperty(P, 'drawingBufferWidth', { get: function() { return Math.max(1, this.canvas.width); }, enumerable: true });
                    Object.defineProperty(P, 'drawingBufferHeight', { get: function() { return Math.max(1, this.canvas.height); }, enumerable: true });
                    P.drawingBufferColorSpace = 'srgb';
                    P.unpackColorSpace = 'srgb';
                });
                globalThis.__glPrivate = { G: G, OP: OP, C: C, wrap: wrap, oid: oid, err: err, live: live, lookup: lookup, typed: typed, pixView: pixView,
                    srcArgs: srcArgs, texSrc: texSrc, loc: loc, info: info, WebGLActiveInfo: WebGLActiveInfo, create: create, del: del, isFn: isFn, P2: P2 };

                // ── 扩展对象 ──
                var EXT = {}, EXT_KEYS = {};
                function ext(name, f) { EXT[name] = f; EXT_KEYS[name.toLowerCase()] = name; }
                function consts(o) { return function() { var e = {}; for (var k in o) e[k] = o[k]; return e; }; }
                ext('ANGLE_instanced_arrays', function(gl) {
                    return { VERTEX_ATTRIB_ARRAY_DIVISOR_ANGLE: 0x88FE,
                        drawArraysInstancedANGLE: function(m, f, c, n) { G(gl.__cid, OP.drawArraysInstanced, m, f, c, n); },
                        drawElementsInstancedANGLE: function(m, c, t, o, n) { G(gl.__cid, OP.drawElementsInstanced, m, c, t, o, n); },
                        vertexAttribDivisorANGLE: function(i, d) { G(gl.__cid, OP.vertexAttribDivisor, i, d); } };
                });
                ext('OES_vertex_array_object', function(gl) {
                    return { VERTEX_ARRAY_BINDING_OES: 0x85B5,
                        createVertexArrayOES: function() { return gl.__createVao(); },
                        deleteVertexArrayOES: function(v) { gl.__deleteVao(v); },
                        isVertexArrayOES: function(v) { return !gl.__lost && live(gl, v, 'VertexArrayObject') && v.__bound; },
                        bindVertexArrayOES: function(v) { gl.__bindVao(v); } };
                });
                ext('WEBGL_draw_buffers', function(gl) {
                    var e = { MAX_COLOR_ATTACHMENTS_WEBGL: 0x8CDF, MAX_DRAW_BUFFERS_WEBGL: 0x8824,
                        drawBuffersWEBGL: function(list) { G(gl.__cid, OP.drawBuffers, Array.prototype.slice.call(list)); } };
                    for (var i = 0; i < 16; i++) { e['COLOR_ATTACHMENT' + i + '_WEBGL'] = 0x8CE0 + i; e['DRAW_BUFFER' + i + '_WEBGL'] = 0x8825 + i; }
                    return e;
                });
                ext('WEBGL_lose_context', function(gl) {
                    // 丢失是同步的（isContextLost 立刻为 true），事件在帧末派发；已经丢了再调 → INVALID_OPERATION
                    return { loseContext: function() { if (gl.__lost) { gl.__pendingLostErr = true; return; } gl.__byApi = true; gl.__lost = true; __game_gl_lose(gl.__cid); },
                        // 只能在 webglcontextlost 派发之后恢复（之前 / 没丢 / 不是 loseContext 丢的 → INVALID_OPERATION）
                        // 游戏没在 webglcontextlost 里 preventDefault 也不能恢复
                        restoreContext: function() { if (!gl.__lost || !gl.__byApi || !gl.__lostSeen || !gl.__restorable) { if (gl.__lost) gl.__pendingLostErr = true; else err(gl, E_OP); return; } gl.__restoreSoon(); } };
                });
                ext('WEBGL_debug_renderer_info', consts({ UNMASKED_VENDOR_WEBGL: 0x9245, UNMASKED_RENDERER_WEBGL: 0x9246 }));
                ext('WEBGL_debug_shaders', function(gl) { return { getTranslatedShaderSource: function(s) { var id = oid(gl, s, 'Shader'); return id > 0 ? G(gl.__cid, OP.getTranslatedShaderSource, id) : ''; } }; });
                ext('OES_standard_derivatives', consts({ FRAGMENT_SHADER_DERIVATIVE_HINT_OES: 0x8B8B }));
                ext('OES_texture_half_float', consts({ HALF_FLOAT_OES: 0x8D61 }));
                ext('EXT_texture_filter_anisotropic', consts({ TEXTURE_MAX_ANISOTROPY_EXT: 0x84FE, MAX_TEXTURE_MAX_ANISOTROPY_EXT: 0x84FF }));
                ext('WEBGL_depth_texture', consts({ UNSIGNED_INT_24_8_WEBGL: 0x84FA }));
                ext('EXT_blend_minmax', consts({ MIN_EXT: 0x8007, MAX_EXT: 0x8008 }));
                ext('EXT_sRGB', consts({ SRGB_EXT: 0x8C40, SRGB_ALPHA_EXT: 0x8C42, SRGB8_ALPHA8_EXT: 0x8C43, FRAMEBUFFER_ATTACHMENT_COLOR_ENCODING_EXT: 0x8210 }));
                ext('EXT_color_buffer_half_float', consts({ RGBA16F_EXT: 0x881A, RGB16F_EXT: 0x881B, FRAMEBUFFER_ATTACHMENT_COMPONENT_TYPE_EXT: 0x8211, UNSIGNED_NORMALIZED_EXT: 0x8C17 }));
                ext('WEBGL_color_buffer_float', consts({ RGBA32F_EXT: 0x8814, FRAMEBUFFER_ATTACHMENT_COMPONENT_TYPE_EXT: 0x8211, UNSIGNED_NORMALIZED_EXT: 0x8C17 }));
                ext('WEBGL_compressed_texture_s3tc', consts({ COMPRESSED_RGB_S3TC_DXT1_EXT: 0x83F0, COMPRESSED_RGBA_S3TC_DXT1_EXT: 0x83F1, COMPRESSED_RGBA_S3TC_DXT3_EXT: 0x83F2, COMPRESSED_RGBA_S3TC_DXT5_EXT: 0x83F3 }));
                ext('WEBGL_compressed_texture_s3tc_srgb', consts({ COMPRESSED_SRGB_S3TC_DXT1_EXT: 0x8C4C, COMPRESSED_SRGB_ALPHA_S3TC_DXT1_EXT: 0x8C4D, COMPRESSED_SRGB_ALPHA_S3TC_DXT3_EXT: 0x8C4E, COMPRESSED_SRGB_ALPHA_S3TC_DXT5_EXT: 0x8C4F }));
                ext('WEBGL_compressed_texture_etc1', consts({ COMPRESSED_RGB_ETC1_WEBGL: 0x8D64 }));
                ext('WEBGL_compressed_texture_etc', consts({ COMPRESSED_R11_EAC: 0x9270, COMPRESSED_SIGNED_R11_EAC: 0x9271, COMPRESSED_RG11_EAC: 0x9272, COMPRESSED_SIGNED_RG11_EAC: 0x9273,
                    COMPRESSED_RGB8_ETC2: 0x9274, COMPRESSED_SRGB8_ETC2: 0x9275, COMPRESSED_RGB8_PUNCHTHROUGH_ALPHA1_ETC2: 0x9276, COMPRESSED_SRGB8_PUNCHTHROUGH_ALPHA1_ETC2: 0x9277,
                    COMPRESSED_RGBA8_ETC2_EAC: 0x9278, COMPRESSED_SRGB8_ALPHA8_ETC2_EAC: 0x9279 }));
                ext('WEBGL_compressed_texture_pvrtc', consts({ COMPRESSED_RGB_PVRTC_4BPPV1_IMG: 0x8C00, COMPRESSED_RGB_PVRTC_2BPPV1_IMG: 0x8C01, COMPRESSED_RGBA_PVRTC_4BPPV1_IMG: 0x8C02, COMPRESSED_RGBA_PVRTC_2BPPV1_IMG: 0x8C03 }));
                ext('WEBGL_compressed_texture_astc', function() {
                    var e = { getSupportedProfiles: function() { return ['ldr']; } };
                    ['4x4', '5x4', '5x5', '6x5', '6x6', '8x5', '8x6', '8x8', '10x5', '10x6', '10x8', '10x10', '12x10', '12x12'].forEach(function(s, i) {
                        e['COMPRESSED_RGBA_ASTC_' + s + '_KHR'] = 0x93B0 + i; e['COMPRESSED_SRGB8_ALPHA8_ASTC_' + s + '_KHR'] = 0x93D0 + i;
                    });
                    return e;
                });
                ext('EXT_texture_compression_bptc', consts({ COMPRESSED_RGBA_BPTC_UNORM_EXT: 0x8E8C, COMPRESSED_SRGB_ALPHA_BPTC_UNORM_EXT: 0x8E8D, COMPRESSED_RGB_BPTC_SIGNED_FLOAT_EXT: 0x8E8E, COMPRESSED_RGB_BPTC_UNSIGNED_FLOAT_EXT: 0x8E8F }));
                ext('EXT_texture_compression_rgtc', consts({ COMPRESSED_RED_RGTC1_EXT: 0x8DBB, COMPRESSED_SIGNED_RED_RGTC1_EXT: 0x8DBC, COMPRESSED_RED_GREEN_RGTC2_EXT: 0x8DBD, COMPRESSED_SIGNED_RED_GREEN_RGTC2_EXT: 0x8DBE }));
                ext('KHR_parallel_shader_compile', consts({ COMPLETION_STATUS_KHR: 0x91B1 }));
                ['OES_element_index_uint', 'OES_texture_float', 'OES_texture_float_linear', 'OES_texture_half_float_linear', 'OES_fbo_render_mipmap',
                    'EXT_frag_depth', 'EXT_shader_texture_lod', 'EXT_float_blend', 'EXT_color_buffer_float'].forEach(function(n) { ext(n, consts({})); });
                ext('WEBGL_multi_draw', function(gl) {
                    // 逐个画（与 Chromium 在不支持原生多绘制时的回退一致）
                    function arr(a, o) { return Array.prototype.slice.call(a, o || 0); }
                    return {
                        multiDrawArraysWEBGL: function(m, f, fo, c, co, n) { f = arr(f, fo); c = arr(c, co); for (var i = 0; i < n; i++) gl.drawArrays(m, f[i], c[i]); },
                        multiDrawElementsWEBGL: function(m, c, co, t, o, oo, n) { c = arr(c, co); o = arr(o, oo); for (var i = 0; i < n; i++) gl.drawElements(m, c[i], t, o[i]); },
                        multiDrawArraysInstancedWEBGL: function(m, f, fo, c, co, ic, io, n) { f = arr(f, fo); c = arr(c, co); ic = arr(ic, io); for (var i = 0; i < n; i++) G(gl.__cid, OP.drawArraysInstanced, m, f[i], c[i], ic[i]); },
                        multiDrawElementsInstancedWEBGL: function(m, c, co, t, o, oo, ic, io, n) { c = arr(c, co); o = arr(o, oo); ic = arr(ic, io); for (var i = 0; i < n; i++) G(gl.__cid, OP.drawElementsInstanced, m, c[i], t, o[i], ic[i]); }
                    };
                });

                // ── 建上下文 ──
                function base(gl, canvas, v, attrs) {
                    hide(gl, '__cid', canvas.__id); hide(gl, '__v', v); hide(gl, '__gen', 1, true); hide(gl, '__lost', false, true);
                    hide(gl, '__attrs', attrs, true); hide(gl, '__byApi', false, true);
                    Object.defineProperty(gl, 'canvas', { value: canvas, enumerable: true });
                    reset(gl);
                }
                // 浏览器的恢复是异步的：下一轮再建，恢复事件在帧末派发（放在原型上：上下文不许有标准之外的自有方法）
                [P1, P2].forEach(function(P) {
                    Object.defineProperty(P, '__restoreSoon', { value: function() { var id = this.__cid; setTimeout(function() { __game_gl_restore(id); }, 0); }, enumerable: false });
                });
                // 换代：旧对象全部作废（丢失 / 恢复之后）
                function reset(gl) {
                    var objs = {}; Object.keys(C).forEach(function(k) { objs[k] = {}; });
                    hide(gl, '__objs', objs, true); hide(gl, '__program', null, true); hide(gl, '__vao', null, true);
                    hide(gl, '__defVao', {}, true); hide(gl, '__ext', {}, true); hide(gl, '__queries', {}, true);
                    hide(gl, '__units', {}, true); hide(gl, '__unit', 0, true);
                }
                // VAO（WebGL2 核心与 OES 扩展共用）
                function vaoFns(P) {
                    function def(k, f) { Object.defineProperty(P, k, { value: f, writable: true, configurable: true, enumerable: false }); }
                    def('__createVao', function() { return wrap(this, 'VertexArrayObject', G(this.__cid, OP.createVertexArray)); });
                    def('__deleteVao', function(v) {
                        var id = oid(this, v, 'VertexArrayObject', true); if (id <= 0 || v.__del) return;
                        G(this.__cid, OP.deleteVertexArray, id); v.__del = true;
                        if (this.__vao === v) this.__vao = null;
                    });
                    def('__bindVao', function(v) {
                        var id = oid(this, v, 'VertexArrayObject', 'bind'); if (id < 0) return;
                        G(this.__cid, OP.bindVertexArray, id); this.__vao = v || null; if (v) v.__bound = true;
                    });
                }
                vaoFns(P1); vaoFns(P2);

                globalThis.__gameWebGL = function(canvas, type, attrs) {
                    var v = type === 'webgl2' ? 2 : 1;
                    var json = __game_gl_create(canvas.__id, v, JSON.stringify(attrs || {}));
                    if (!json) return null;
                    var gl = Object.create((v === 2 ? GL2 : GL1).prototype);
                    base(gl, canvas, v, JSON.parse(json));
                    byCanvas[canvas.__id] = gl;
                    return gl;
                };
                globalThis.__gameWebGLReset = reset;
                // 原生在帧末派发：丢失 → 游戏 preventDefault 了就安排恢复；恢复 → 换代后派发
                globalThis.__gameGlEvent = function(id, type) {
                    var gl = byCanvas[id]; if (!gl) return;
                    if (type === 'webglcontextlost') { gl.__lost = true; gl.__lostSeen = true; gl.__restorable = false; }
                    if (type === 'webglcontextrestored') {
                        gl.__lost = false; gl.__byApi = false; gl.__lostSeen = false; gl.__gen++;
                        // 恢复之后扩展要重新 getExtension（新对象）；只有 WEBGL_lose_context 还是原来那个（CTS context-lost-restored）
                        var lc = gl.__ext.webgl_lose_context; reset(gl);
                        if (lc) gl.__ext.webgl_lose_context = lc;
                    }
                    var ev = Object.create(WebGLContextEvent.prototype);
                    var prevented = false;
                    ev.type = type; ev.statusMessage = ''; ev.target = ev.currentTarget = gl.canvas;
                    ev.preventDefault = function() { prevented = true; if (type === 'webglcontextlost') gl.__restorable = true; };
                    Object.defineProperty(ev, 'defaultPrevented', { get: function() { return prevented; } });
                    if (typeof gl.canvas.dispatchEvent === 'function') gl.canvas.dispatchEvent(ev);
                    if (type === 'webglcontextlost' && prevented && !gl.__byApi) gl.__restoreSoon();
                };
            })();
            // wx.getGPU（G8.9，11 B-5）：WebGPU 还没有映射到 wgpu。按 WebGPU 标准里「没有可用适配器」的写法给：
            // requestAdapter() resolve null —— 引擎（three.js WebGPURenderer、Babylon 等）据此回退 WebGL（design-webgl.md §1）
            (function() {
                var gpu = {
                    requestAdapter: function() { return Promise.resolve(null); },
                    getPreferredCanvasFormat: function() { return 'rgba8unorm'; },
                    wgslLanguageFeatures: typeof Set === 'function' ? new Set() : []
                };
                wx.getGPU = function() { return gpu; };
            })();
