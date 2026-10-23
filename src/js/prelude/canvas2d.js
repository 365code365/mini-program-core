            // `<canvas type="2d">` 的 Canvas 节点（官方 api/canvas/Canvas.html、RenderingContext）。
            //
            // 拿法与微信一致：`createSelectorQuery().select('#c').fields({node: true, size: true})`，
            // `res[0].node.getContext('2d')`。上下文是属性式 API（`ctx.fillStyle = …`），画了就生效，
            // 没有 `draw()`：这里把指令攒在一个微任务里，当前这段同步代码跑完一次性交给原生光栅化
            // （`__native_canvas_draw('2d#<id>', …)`，原生按「追加」执行并记下整段日志，后备缓冲重建时重放）。
            //
            // 坐标：开发者画在 `canvas.width × canvas.height` 的后备像素上（常见写法是把它设成
            // CSS 尺寸 × dpr 再 `ctx.scale(dpr, dpr)`）。原生画布按 CSS 逻辑像素收指令，所以每次重置后
            // 先垫一个 `scale(CSS 宽 / canvas.width)`，开发者那一层 scale 叠在它上面，结果正好对齐。
            //
            // 渐变（线性 / 径向 / 锥形）、图案、阴影、globalCompositeOperation 由原生按覆盖率蒙版合成
            // （canvas/fx2d.rs）；measureText 按画布字体真实量宽；getImageData / toDataURL 先把攒着的指令
            // 交出去、原生执行完再读像素（同步桥）。clip（路径裁剪）与 setLineDash / lineDashOffset 见 canvas/clipdash.rs；
            // miterLimit 只记不用（光栅器的斜接没有长度上限）。
            var __c2dNodes = {};
            function __c2dColor(v) {
                if (v && typeof v === 'object' && v.__stops) { return v.__stops.length ? v.__stops[0][1] : '#000000'; }
                return String(v);
            }
            // fillStyle / strokeStyle 的指令：纯色、渐变或图案
            function __c2dStyleCmd(which, v) {
                if (v && typeof v === 'object' && v.__grad) {
                    return { type: 'set' + which + 'Gradient', kind: v.__grad.kind, args: v.__grad.args, stops: v.__stops };
                }
                if (v && typeof v === 'object' && v.__pattern) {
                    return { type: 'set' + which + 'Pattern', src: v.__pattern.src, repetition: v.__pattern.repetition };
                }
                return { type: 'set' + which + 'Style', color: __c2dColor(v) };
            }
            function __c2dContext(node) {
                var st = {};
                function resetState() {
                    st = { fillStyle: '#000000', strokeStyle: '#000000', lineWidth: 1, lineCap: 'butt', lineJoin: 'miter',
                           font: '10px sans-serif', textAlign: 'start', textBaseline: 'alphabetic', globalAlpha: 1,
                           globalCompositeOperation: 'source-over', shadowBlur: 0, shadowColor: 'rgba(0, 0, 0, 0)',
                           shadowOffsetX: 0, shadowOffsetY: 0 };
                }
                resetState();
                var push = function(cmd) { node._pending.push(cmd); node._queue(); };
                var ctx = { canvas: node, _resetState: resetState };
                var props = {
                    fillStyle: function(v) { push(__c2dStyleCmd('Fill', v)); },
                    strokeStyle: function(v) { push(__c2dStyleCmd('Stroke', v)); },
                    lineWidth: function(v) { push({ type: 'setLineWidth', width: Number(v) }); },
                    lineCap: function(v) { push({ type: 'setLineCap', cap: String(v) }); },
                    lineJoin: function(v) { push({ type: 'setLineJoin', join: String(v) }); },
                    font: function(v) {
                        var m = /(\d+(?:\.\d+)?)px/.exec(String(v));
                        if (m) push({ type: 'setFontSize', size: Number(m[1]) });
                    },
                    textAlign: function(v) { push({ type: 'setTextAlign', align: String(v) }); },
                    textBaseline: function(v) { push({ type: 'setTextBaseline', baseline: String(v) }); },
                    globalAlpha: function(v) { push({ type: 'setGlobalAlpha', alpha: Number(v) }); },
                    globalCompositeOperation: function(v) { push({ type: 'setComposite', op: String(v) }); },
                    shadowBlur: function() { pushShadow(); },
                    shadowColor: function() { pushShadow(); },
                    shadowOffsetX: function() { pushShadow(); },
                    shadowOffsetY: function() { pushShadow(); }
                };
                function pushShadow() {
                    push({ type: 'setShadow2d', blur: Number(st.shadowBlur) || 0, color: String(st.shadowColor || 'rgba(0,0,0,0)'),
                           offsetX: Number(st.shadowOffsetX) || 0, offsetY: Number(st.shadowOffsetY) || 0 });
                }
                Object.keys(props).forEach(function(k) {
                    Object.defineProperty(ctx, k, {
                        get: function() { return st[k]; },
                        set: function(v) { st[k] = v; props[k](v); },
                        enumerable: true
                    });
                });
                // miterLimit / lineDashOffset 进状态（canvas/clipdash.rs）；imageSmoothingEnabled 读写照常、不出指令
                var dash = { miterLimit: 10, lineDashOffset: 0 };
                Object.defineProperty(ctx, 'miterLimit', { enumerable: true, get: function() { return dash.miterLimit; },
                    set: function(v) { dash.miterLimit = Number(v); push({ type: 'setMiterLimit', limit: Number(v) }); } });
                Object.defineProperty(ctx, 'lineDashOffset', { enumerable: true, get: function() { return dash.lineDashOffset; },
                    set: function(v) { dash.lineDashOffset = Number(v) || 0; push({ type: 'setLineDashOffset', offset: dash.lineDashOffset }); } });
                ctx.imageSmoothingEnabled = true;
                var pre = function() { return [node._css[0] / (node._w || 1), node._css[1] / (node._h || 1)]; };
                var simple = {
                    fillRect: function(x, y, w, h) { return { type: 'fillRect', x: x, y: y, width: w, height: h }; },
                    strokeRect: function(x, y, w, h) { return { type: 'strokeRect', x: x, y: y, width: w, height: h }; },
                    clearRect: function(x, y, w, h) { return { type: 'clearRect', x: x, y: y, width: w, height: h }; },
                    beginPath: function() { return { type: 'beginPath' }; },
                    closePath: function() { return { type: 'closePath' }; },
                    moveTo: function(x, y) { return { type: 'moveTo', x: x, y: y }; },
                    lineTo: function(x, y) { return { type: 'lineTo', x: x, y: y }; },
                    rect: function(x, y, w, h) { return { type: 'rect', x: x, y: y, width: w, height: h }; },
                    arc: function(x, y, r, s, e, ccw) { return { type: 'arc', x: x, y: y, r: r, sAngle: s, eAngle: e, counterclockwise: !!ccw }; },
                    quadraticCurveTo: function(cpx, cpy, x, y) { return { type: 'quadraticCurveTo', cpx: cpx, cpy: cpy, x: x, y: y }; },
                    bezierCurveTo: function(a, b, c, d, x, y) { return { type: 'bezierCurveTo', cp1x: a, cp1y: b, cp2x: c, cp2y: d, x: x, y: y }; },
                    fill: function() { return { type: 'fill' }; },
                    stroke: function() { return { type: 'stroke' }; },
                    fillText: function(t, x, y, mw) { return { type: 'fillText', text: String(t), x: x, y: y, maxWidth: mw }; },
                    strokeText: function(t, x, y, mw) { return { type: 'strokeText', text: String(t), x: x, y: y, maxWidth: mw }; },
                    translate: function(x, y) { return { type: 'translate', x: x, y: y }; },
                    rotate: function(a) { return { type: 'rotate', angle: a }; },
                    scale: function(x, y) { return { type: 'scale', scaleX: x, scaleY: y === undefined ? x : y }; }
                };
                Object.keys(simple).forEach(function(k) {
                    ctx[k] = function() { push(simple[k].apply(null, arguments)); };
                });
                ctx.save = function() { node._stack.push(Object.assign({}, st)); push({ type: 'save' }); };
                ctx.restore = function() { if (node._stack.length) st = node._stack.pop(); push({ type: 'restore' }); };
                ctx.drawImage = function(img, a, b, c, d, e, f, g, h) {
                    var src = img && typeof img === 'object' ? (img.src || img._src || '') : String(img || '');
                    if (!src) return;
                    if (arguments.length <= 3) push({ type: 'drawImage', src: src, dx: a, dy: b });
                    else if (arguments.length <= 5) push({ type: 'drawImage', src: src, dx: a, dy: b, dWidth: c, dHeight: d });
                    else push({ type: 'drawImage', src: src, sx: a, sy: b, sWidth: c, sHeight: d, dx: e, dy: f, dWidth: g, dHeight: h });
                };
                ctx.measureText = function(text) {
                    var m = /(\d+(?:\.\d+)?)px/.exec(String(st.font)), size = m ? Number(m[1]) : 10;
                    var w = Number(__native_measure_text(String(size), String(text))) || 0;
                    return { width: w, actualBoundingBoxLeft: 0, actualBoundingBoxRight: w,
                             actualBoundingBoxAscent: size * 0.8, actualBoundingBoxDescent: size * 0.2,
                             fontBoundingBoxAscent: size * 0.88, fontBoundingBoxDescent: size * 0.12 };
                };
                function gradient(kind, args) {
                    var g = { __stops: [], __grad: { kind: kind, args: args } };
                    g.addColorStop = function(offset, color) { g.__stops.push([Number(offset), String(color)]); g.__stops.sort(function(p, q) { return p[0] - q[0]; }); };
                    return g;
                }
                ctx.createLinearGradient = function(x0, y0, x1, y1) { return gradient('linear', [x0, y0, x1, y1]); };
                ctx.createRadialGradient = function(x0, y0, r0, x1, y1, r1) { return gradient('radial', [x0, y0, r0, x1, y1, r1]); };
                ctx.createConicGradient = function(a, x, y) { return gradient('conic', [a, x, y]); };
                ctx.createPattern = function(img, repetition) {
                    var src = img && typeof img === 'object' ? (img.src || img._src || '') : String(img || '');
                    return src ? { __pattern: { src: src, repetition: repetition == null ? 'repeat' : String(repetition) } } : null;
                };
                var dashList = [];
                ctx.setLineDash = function(seg) {
                    seg = (seg || []).map(Number);
                    if (seg.some(function(s) { return !isFinite(s) || s < 0; })) return;
                    dashList = seg.length % 2 ? seg.concat(seg) : seg;
                    push({ type: 'setLineDash', segments: dashList });
                };
                ctx.getLineDash = function() { return dashList.slice(); };
                // clip(path2d?)：给了 Path2D 先把它放进当前路径
                ctx.clip = function(p) { if (p && p.__cmds) { push({ type: 'beginPath' }); p.__cmds.forEach(push); } push({ type: 'clip' }); };
                ctx.arcTo = function(x1, y1, x2, y2, r) { push({ type: 'arcTo', x1: x1, y1: y1, x2: x2, y2: y2, r: r }); };
                ctx.ellipse = function(x, y, rx, ry, rot, s, e, ccw) {
                    push({ type: 'ellipse', x: x, y: y, rx: rx, ry: ry, rotation: rot || 0, sAngle: s, eAngle: e, counterclockwise: !!ccw });
                };
                var mtx = [1, 0, 0, 1, 0, 0];
                ctx.setTransform = function(a, b, c, d, e, f) {
                    if (a && typeof a === 'object') { f = a.f; e = a.e; d = a.d; c = a.c; b = a.b; a = a.a; }
                    if (a === undefined) { a = 1; b = 0; c = 0; d = 1; e = 0; f = 0; }
                    mtx = [a, b, c, d, e, f];
                    push({ type: 'setTransform', a: a, b: b, c: c, d: d, e: e, f: f, pre: pre() });
                };
                ctx.resetTransform = function() { ctx.setTransform(1, 0, 0, 1, 0, 0); };
                ctx.transform = function(a, b, c, d, e, f) { push({ type: 'transform', a: a, b: b, c: c, d: d, e: e, f: f }); };
                ctx.getTransform = function() { return { a: mtx[0], b: mtx[1], c: mtx[2], d: mtx[3], e: mtx[4], f: mtx[5] }; };
                ctx.getImageData = function(x, y, w, h) {
                    node._flush();
                    var b64 = __native_canvas_read(node._key, String(x), String(y), String(w), String(h), JSON.stringify(pre()));
                    var bytes = new Uint8ClampedArray(__b64decode(b64));
                    var data = bytes.length === w * h * 4 ? bytes : new Uint8ClampedArray(Math.max(0, w * h * 4));
                    return { width: w, height: h, data: data };
                };
                ctx.putImageData = function(img, dx, dy) {
                    if (!img || !img.data) return;
                    push({ type: 'putImageData', b64: __b64encode(img.data), w: img.width, h: img.height, dx: dx, dy: dy, pre: pre() });
                };
                ctx.createImageData = function(w, h) {
                    if (w && typeof w === 'object') { h = w.height; w = w.width; }
                    return { width: w, height: h, data: new Uint8ClampedArray(Math.max(0, w * h * 4)) };
                };
                // fill(path2d) / stroke(path2d)：把 Path2D 录下的路径放进当前路径再画
                var rawFill = ctx.fill, rawStroke = ctx.stroke;
                ctx.fill = function(p) { if (p && p.__cmds) { push({ type: 'beginPath' }); p.__cmds.forEach(push); } rawFill(); };
                ctx.stroke = function(p) { if (p && p.__cmds) { push({ type: 'beginPath' }); p.__cmds.forEach(push); } rawStroke(); };
                return ctx;
            }
            function __canvas2dNode(info) {
                var key = '2d#' + info.id;
                var node = __c2dNodes[key];
                if (node) { node._css = [info.width, info.height]; return node; }
                node = { _key: key, _css: [info.width, info.height], _w: Math.round(info.width) || 300, _h: Math.round(info.height) || 150,
                         _pending: [], _stack: [], _ctx: null, _queued: false, _fresh: true };
                // 改 width / height = 清空画布、状态复位（HTML 语义）
                node._reset = function() {
                    node._pending = []; node._stack = []; node._fresh = true;
                    if (node._ctx) node._ctx._resetState();
                    node._queue();
                };
                node._queue = function() {
                    if (node._queued) return;
                    node._queued = true;
                    Promise.resolve().then(node._flush);
                };
                node._flush = function() {
                    node._queued = false;
                    var cmds = node._pending; node._pending = [];
                    if (node._fresh) {
                        node._fresh = false;
                        cmds = [{ type: '__reset' }, { type: 'scale', scaleX: node._css[0] / (node._w || 1), scaleY: node._css[1] / (node._h || 1) }].concat(cmds);
                    }
                    if (cmds.length && typeof __native_canvas_draw === 'function') __native_canvas_draw(node._key, JSON.stringify(cmds));
                };
                Object.defineProperty(node, 'width', { get: function() { return node._w; }, set: function(v) { node._w = Math.max(0, Math.floor(Number(v) || 0)); node._reset(); } });
                Object.defineProperty(node, 'height', { get: function() { return node._h; }, set: function(v) { node._h = Math.max(0, Math.floor(Number(v) || 0)); node._reset(); } });
                node.getContext = function(kind) {
                    if (kind !== '2d') return null;
                    if (!node._ctx) node._ctx = __c2dContext(node);
                    return node._ctx;
                };
                node.createImage = function() {
                    var img = { width: 0, height: 0, onload: null, onerror: null, complete: false, _src: '' };
                    Object.defineProperty(img, 'src', {
                        get: function() { return img._src; },
                        set: function(v) {
                            img._src = String(v);
                            setTimeout(function() { img.complete = true; if (typeof img.onload === 'function') img.onload(); }, 0);
                        }
                    });
                    return img;
                };
                node.requestAnimationFrame = function(cb) { return setTimeout(function() { cb(Date.now()); }, 16); };
                node.cancelAnimationFrame = function(id) { clearTimeout(id); };
                // Path2D：录下路径指令，ctx.fill(path) / stroke(path) 时放进当前路径
                node.createPath2D = function(from) {
                    var p = { __cmds: from && from.__cmds ? from.__cmds.slice() : [] };
                    ['moveTo', 'lineTo', 'rect', 'arc', 'quadraticCurveTo', 'bezierCurveTo', 'closePath', 'arcTo', 'ellipse'].forEach(function(k) {
                        p[k] = function() {
                            var a = arguments;
                            var c = { moveTo: { type: 'moveTo', x: a[0], y: a[1] }, lineTo: { type: 'lineTo', x: a[0], y: a[1] },
                                      rect: { type: 'rect', x: a[0], y: a[1], width: a[2], height: a[3] },
                                      arc: { type: 'arc', x: a[0], y: a[1], r: a[2], sAngle: a[3], eAngle: a[4], counterclockwise: !!a[5] },
                                      quadraticCurveTo: { type: 'quadraticCurveTo', cpx: a[0], cpy: a[1], x: a[2], y: a[3] },
                                      bezierCurveTo: { type: 'bezierCurveTo', cp1x: a[0], cp1y: a[1], cp2x: a[2], cp2y: a[3], x: a[4], y: a[5] },
                                      closePath: { type: 'closePath' }, arcTo: { type: 'arcTo', x1: a[0], y1: a[1], x2: a[2], y2: a[3], r: a[4] },
                                      ellipse: { type: 'ellipse', x: a[0], y: a[1], rx: a[2], ry: a[3], rotation: a[4] || 0, sAngle: a[5], eAngle: a[6], counterclockwise: !!a[7] } }[k];
                            p.__cmds.push(c);
                        };
                    });
                    p.addPath = function(o) { if (o && o.__cmds) p.__cmds = p.__cmds.concat(o.__cmds); };
                    return p;
                };
                // toDataURL：先交出攒着的指令，原生执行完再编码（PNG，或 image/jpeg + 质量）
                node.toDataURL = function(type, quality) {
                    node._flush();
                    var jpeg = /jpe?g/.test(String(type || ''));
                    return __native_canvas_encode(node._key, jpeg ? 'jpeg' : 'png', String(quality === undefined ? 0.92 : quality),
                                                  JSON.stringify({ pre: [node._css[0] / (node._w || 1), node._css[1] / (node._h || 1)] }));
                };
                __c2dNodes[key] = node;
                node._queue();
                return node;
            }
            // ---------- 画布导出与像素（官方 api/canvas/wx.canvasToTempFilePath、canvasGetImageData、canvasPutImageData）----------
            // `canvas`（type=2d 的节点）或 `canvasId`（旧接口）二选一；导出文件落 wxfile://tmp_…
            function __canvasKeyOf(o) {
                if (o && o.canvas && o.canvas._key) { o.canvas._flush(); return { key: o.canvas._key, pre: [o.canvas._css[0] / (o.canvas._w || 1), o.canvas._css[1] / (o.canvas._h || 1)] }; }
                return { key: String((o && o.canvasId) || ''), pre: [1, 1] };
            }
            function __canvasCb(o, name, ok, res) {
                res.errMsg = name + (ok ? ':ok' : ':fail ' + (res.err || ''));
                delete res.err;
                var promise;
                if (!o.success && !o.fail && !o.complete) {
                    return new Promise(function(resolve, reject) { setTimeout(function() { (ok ? resolve : reject)(res); }, 0); });
                }
                setTimeout(function() { if (ok) { o.success && o.success(res); } else { o.fail && o.fail(res); } o.complete && o.complete(res); }, 0);
                return promise;
            }
            wx.canvasToTempFilePath = function(o) {
                o = o || {};
                var k = __canvasKeyOf(o);
                var r = {};
                try {
                    r = JSON.parse(__native_canvas_save(k.key, JSON.stringify({
                        x: o.x || 0, y: o.y || 0, width: o.width || 0, height: o.height || 0,
                        destWidth: o.destWidth || 0, destHeight: o.destHeight || 0,
                        fileType: o.fileType || 'png', quality: o.quality === undefined ? 1 : o.quality, pre: k.pre
                    })));
                } catch (e) { r = { err: String(e) }; }
                return __canvasCb(o, 'canvasToTempFilePath', !r.err, r.err ? { err: r.err } : { tempFilePath: r.tempFilePath });
            };
            wx.canvasGetImageData = function(o) {
                o = o || {};
                var k = __canvasKeyOf(o), w = o.width || 0, h = o.height || 0;
                var b64 = __native_canvas_read(k.key, String(o.x || 0), String(o.y || 0), String(w), String(h), JSON.stringify(k.pre));
                return __canvasCb(o, 'canvasGetImageData', true, { width: w, height: h, data: new Uint8ClampedArray(__b64decode(b64)) });
            };
            wx.canvasPutImageData = function(o) {
                o = o || {};
                var k = __canvasKeyOf(o);
                if (typeof __native_canvas_draw === 'function') {
                    __native_canvas_draw(k.key, JSON.stringify([{ type: 'putImageData', b64: __b64encode(o.data), w: o.width, h: o.height, dx: o.x || 0, dy: o.y || 0, pre: k.pre }]));
                }
                return __canvasCb(o, 'canvasPutImageData', true, {});
            };
