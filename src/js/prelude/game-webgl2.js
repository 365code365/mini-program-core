            // WebGL2RenderingContext 独有的方法（game-webgl.js 之后装；工具函数从 __glPrivate 取）。
            (function() {
                var X = globalThis.__glPrivate;
                if (!X) return;
                delete globalThis.__glPrivate;
                var G = X.G, OP = X.OP, P = X.P2, oid = X.oid, err = X.err, wrap = X.wrap, lookup = X.lookup, typed = X.typed, info = X.info;
                var E_OP = 0x502, E_VALUE = 0x501;
                P.createVertexArray = function() { return this.__createVao(); };
                P.deleteVertexArray = function(v) { this.__deleteVao(v); };
                P.isVertexArray = function(v) { return !this.__lost && X.live(this, v, 'VertexArrayObject') && v.__bound; };
                P.bindVertexArray = function(v) { this.__bindVao(v); };
                P.createQuery = X.create('Query', 'createQuery');
                P.createSampler = X.create('Sampler', 'createSampler');
                P.createTransformFeedback = X.create('TransformFeedback', 'createTransformFeedback');
                P.deleteQuery = X.del('Query', 'deleteQuery');
                P.deleteSampler = X.del('Sampler', 'deleteSampler');
                P.deleteTransformFeedback = X.del('TransformFeedback', 'deleteTransformFeedback');
                P.isQuery = X.isFn('Query', true);
                P.isSampler = X.isFn('Sampler');
                P.isTransformFeedback = X.isFn('TransformFeedback', true);
                P.isSync = X.isFn('Sync');
                P.beginQuery = function(t, q) {
                    var id = oid(this, q, 'Query'); if (id <= 0) { if (id === 0) err(this, E_OP); return; }
                    G(this.__cid, OP.beginQuery, t, id); q.__bound = true; this.__queries[t] = q;
                };
                P.endQuery = function(t) { G(this.__cid, OP.endQuery, t); delete this.__queries[t]; };
                P.getQuery = function(t, p) { return p === 0x8865 ? (this.__queries[t] || null) : (err(this, 0x500), null); };
                P.getQueryParameter = function(q, p) { var id = oid(this, q, 'Query'); return id > 0 ? G(this.__cid, OP.getQueryParameter, id, p) : null; };
                P.bindSampler = function(u, s) { var id = oid(this, s, 'Sampler'); if (id >= 0) G(this.__cid, OP.bindSampler, u, id); };
                P.samplerParameteri = function(s, p, v) { var id = oid(this, s, 'Sampler'); if (id > 0) G(this.__cid, OP.samplerParameteri, id, p, v); };
                P.samplerParameterf = function(s, p, v) { var id = oid(this, s, 'Sampler'); if (id > 0) G(this.__cid, OP.samplerParameterf, id, p, v); };
                P.getSamplerParameter = function(s, p) { var id = oid(this, s, 'Sampler'); return id > 0 ? G(this.__cid, OP.getSamplerParameter, id, p) : null; };
                P.fenceSync = function(c, f) { return wrap(this, 'Sync', G(this.__cid, OP.fenceSync, c, f)); };
                P.deleteSync = X.del('Sync', 'deleteSync');
                P.clientWaitSync = function(s, f, t) { var id = oid(this, s, 'Sync'); return id > 0 ? G(this.__cid, OP.clientWaitSync, id, f, t) : 0x911D; };
                P.waitSync = function(s, f, t) { var id = oid(this, s, 'Sync'); if (id > 0) G(this.__cid, OP.waitSync, id, f, t); };
                P.getSyncParameter = function(s, p) { var id = oid(this, s, 'Sync'); return id > 0 ? G(this.__cid, OP.getSyncParameter, id, p) : null; };
                P.bindTransformFeedback = function(t, x) { var id = oid(this, x, 'TransformFeedback'); if (id < 0) return; G(this.__cid, OP.bindTransformFeedback, t, id); if (x) x.__bound = true; };
                P.transformFeedbackVaryings = function(p, names, mode) { var id = oid(this, p, 'Program'); if (id > 0) G(this.__cid, OP.transformFeedbackVaryings, id, Array.prototype.slice.call(names).map(String), mode); };
                P.getTransformFeedbackVarying = function(p, i) { var id = oid(this, p, 'Program'); if (id <= 0) return null; var r = G(this.__cid, OP.getTransformFeedbackVarying, id, i); return r ? info(X.WebGLActiveInfo, r) : null; };
                P.bindBufferBase = function(t, i, b) { var id = oid(this, b, 'Buffer'); if (id < 0) return; G(this.__cid, OP.bindBufferBase, t, i, id); if (b) b.__bound = true; };
                P.bindBufferRange = function(t, i, b, o, s) { var id = oid(this, b, 'Buffer'); if (id < 0) return; G(this.__cid, OP.bindBufferRange, t, i, id, o, s); if (b) b.__bound = true; };
                P.getIndexedParameter = function(t, i) {
                    var r = G(this.__cid, OP.getIndexedParameter, t, i);
                    return t === 0x8C8F || t === 0x8A28 ? lookup(this, 'Buffer', r) : r;
                };
                P.getUniformBlockIndex = function(p, n) { var id = oid(this, p, 'Program'); return id > 0 ? G(this.__cid, OP.getUniformBlockIndex, id, String(n)) : 0xFFFFFFFF; };
                P.getActiveUniformBlockParameter = function(p, i, n) { var id = oid(this, p, 'Program'); return id > 0 ? G(this.__cid, OP.getActiveUniformBlockParameter, id, i, n) : null; };
                P.getActiveUniformBlockName = function(p, i) { var id = oid(this, p, 'Program'); return id > 0 ? G(this.__cid, OP.getActiveUniformBlockName, id, i) : null; };
                P.uniformBlockBinding = function(p, i, b) { var id = oid(this, p, 'Program'); if (id > 0) G(this.__cid, OP.uniformBlockBinding, id, i, b); };
                P.getUniformIndices = function(p, names) { var id = oid(this, p, 'Program'); return id > 0 ? G(this.__cid, OP.getUniformIndices, id, Array.prototype.slice.call(names).map(String)) : null; };
                P.getActiveUniforms = function(p, idx, n) { var id = oid(this, p, 'Program'); return id > 0 ? G(this.__cid, OP.getActiveUniforms, id, Array.prototype.slice.call(idx), n) : null; };
                P.getFragDataLocation = function(p, n) { var id = oid(this, p, 'Program'); return id > 0 ? G(this.__cid, OP.getFragDataLocation, id, String(n)) : -1; };
                ['1ui', '2ui', '3ui', '4ui'].forEach(function(s) {
                    var op = OP['uniform' + s];
                    P['uniform' + s] = function(l, x, y, z, w) { var n = X.loc(this, l); if (n !== null) G(this.__cid, op, n, x, y, z, w); };
                });
                [1, 2, 3, 4].forEach(function(n) {
                    P['uniform' + n + 'uiv'] = function(l, v, off, len) { var k = X.loc(this, l); if (k !== null) G(this.__cid, OP._uniformuiv, k, n, typed(Uint32Array, v, off, len)); };
                });
                [[2, 3], [2, 4], [3, 2], [3, 4], [4, 2], [4, 3]].forEach(function(cr) {
                    P['uniformMatrix' + cr[0] + 'x' + cr[1] + 'fv'] = function(l, tr, v, off, len) { var k = X.loc(this, l); if (k !== null) G(this.__cid, OP._uniformMatrix, k, cr[0], cr[1], tr, typed(Float32Array, v, off, len)); };
                });
                P.vertexAttribI4iv = function(i, v) { v = typed(Int32Array, v); G(this.__cid, OP.vertexAttribI4i, i, v[0], v[1], v[2], v[3]); };
                P.vertexAttribI4uiv = function(i, v) { v = typed(Uint32Array, v); G(this.__cid, OP.vertexAttribI4ui, i, v[0], v[1], v[2], v[3]); };
                P.drawBuffers = function(list) { G(this.__cid, OP.drawBuffers, Array.prototype.slice.call(list)); };
                P.invalidateFramebuffer = function(t, list) { G(this.__cid, OP.invalidateFramebuffer, t, Array.prototype.slice.call(list)); };
                P.invalidateSubFramebuffer = function(t, list, x, y, w, h) { G(this.__cid, OP.invalidateSubFramebuffer, t, Array.prototype.slice.call(list), x, y, w, h); };
                P.framebufferTextureLayer = function(t, a, x, lv, layer) { var id = oid(this, x, 'Texture'); if (id >= 0) G(this.__cid, OP.framebufferTextureLayer, t, a, id, lv, layer); };
                P.clearBufferfv = function(b, i, v, off) { G(this.__cid, OP._clearBufferfv, b, i, typed(Float32Array, v, off)); };
                P.clearBufferiv = function(b, i, v, off) { G(this.__cid, OP._clearBufferiv, b, i, typed(Int32Array, v, off)); };
                P.clearBufferuiv = function(b, i, v, off) { G(this.__cid, OP._clearBufferuiv, b, i, typed(Uint32Array, v, off)); };
                P.getBufferSubData = function(t, o, dst, off, len) {
                    if (!ArrayBuffer.isView(dst)) throw new TypeError('getBufferSubData: 目标应当是 ArrayBufferView');
                    if (off || len) dst = dst.subarray(off || 0, len ? (off || 0) + len : undefined);
                    G(this.__cid, OP.getBufferSubData, t, o, dst);
                };
                P.texImage3D = function(t, lv, ifmt, w, h, d, b, f, ty, src, off) {
                    if (typeof src === 'number') return G(this.__cid, OP._texImage3D, t, lv, ifmt, w, h, d, b, f, ty, src);
                    if (src && typeof src === 'object' && !ArrayBuffer.isView(src)) return X.texSrc(this, OP._texImage3DSource, [t, lv, ifmt, w, h, d, b, f, ty], src);
                    var v = X.pixView(this, src, ty, off); if (v === undefined) return;
                    G(this.__cid, OP._texImage3D, t, lv, ifmt, w, h, d, b, f, ty, v);
                };
                P.texSubImage3D = function(t, lv, x, y, z, w, h, d, f, ty, src, off) {
                    if (typeof src === 'number') return G(this.__cid, OP._texSubImage3D, t, lv, x, y, z, w, h, d, f, ty, src);
                    if (src && typeof src === 'object' && !ArrayBuffer.isView(src)) return X.texSrc(this, OP._texSubImage3DSource, [t, lv, x, y, z, w, h, d, f, ty], src);
                    var v = X.pixView(this, src, ty, off); if (v === undefined) return;
                    if (v === null) { err(this, E_VALUE); return; }
                    G(this.__cid, OP._texSubImage3D, t, lv, x, y, z, w, h, d, f, ty, v);
                };
                P.compressedTexImage3D = function(t, lv, ifmt, w, h, d, b, data, off, len) {
                    if (!ArrayBuffer.isView(data)) throw new TypeError('compressedTexImage3D: 数据应当是 ArrayBufferView');
                    if (off || len) data = data.subarray(off || 0, len ? (off || 0) + len : undefined);
                    G(this.__cid, OP._compressedTexImage3D, t, lv, ifmt, w, h, d, b, data);
                };
                P.compressedTexSubImage3D = function(t, lv, x, y, z, w, h, d, f, data, off, len) {
                    if (!ArrayBuffer.isView(data)) throw new TypeError('compressedTexSubImage3D: 数据应当是 ArrayBufferView');
                    if (off || len) data = data.subarray(off || 0, len ? (off || 0) + len : undefined);
                    G(this.__cid, OP._compressedTexSubImage3D, t, lv, x, y, z, w, h, d, f, data);
                };
            })();
