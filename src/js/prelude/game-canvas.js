            // ==================== 小游戏：画布与 2D 上下文（game-doc/02 §5、03 §2.1~2.2） ====================
            //
            // 原生侧是立即模式（game/canvas/natives.rs）：每个方法直接调 __game_2d(id, op, ...)，返回时像素已写好。
            // 这里只做 HTML Canvas 的「属性语义」：读回值、非法值保留旧值、对象形状。OP 编号与 natives.rs 文件头一一对应。
            (function() {
                var OP = { fillRect: 1, strokeRect: 2, clearRect: 3, beginPath: 4, closePath: 5, moveTo: 6, lineTo: 7, arc: 8,
                    rect: 9, quadraticCurveTo: 10, bezierCurveTo: 11, arcTo: 12, ellipse: 13, fill: 14, stroke: 15, clip: 16,
                    save: 17, restore: 18, translate: 19, rotate: 20, scale: 21, setTransform: 22, transform: 23,
                    resetTransform: 24, fillText: 25, strokeText: 26, measureText: 27 };
                // 原生由 GameEngine 注册；只装前置代码（profile 测试）时还没有，取不到就让调用时报错而不是加载期报错。
                // 不包一层 apply：每个 2D 调用都过这里，直接引用省一次转发
                var N = typeof __game_2d === 'function' ? __game_2d : function() { throw new Error('__game_2d 未注册（不在小游戏实例里）'); };

                // 颜色读回：HTML 规范化成 #rrggbb（不透明）或 rgba(r, g, b, a)。解析在 JS 里做一遍只为读回，
                // 绘制用的还是原生那份解析（parse_color_str）。读不出的（hsl 等）原样回给游戏 —— 真机的格式以取证为准
                var named = { black: '#000000', white: '#ffffff', red: '#ff0000', green: '#008000', blue: '#0000ff',
                    yellow: '#ffff00', gray: '#808080', grey: '#808080', orange: '#ffa500', purple: '#800080',
                    transparent: 'rgba(0, 0, 0, 0)' };
                function hex2(n) { n = Math.max(0, Math.min(255, Math.round(n))); return (n < 16 ? '0' : '') + n.toString(16); }
                function normColor(s) {
                    var v = String(s).trim().toLowerCase();
                    if (named[v]) return named[v];
                    var m = /^#([0-9a-f]{3,8})$/.exec(v);
                    if (m) {
                        var h = m[1];
                        if (h.length === 3 || h.length === 4) h = h.split('').map(function(c) { return c + c; }).join('');
                        if (h.length === 6) return '#' + h;
                        if (h.length === 8) {
                            var a = parseInt(h.slice(6), 16) / 255;
                            return a === 1 ? '#' + h.slice(0, 6) : 'rgba(' + parseInt(h.slice(0, 2), 16) + ', ' + parseInt(h.slice(2, 4), 16) + ', ' + parseInt(h.slice(4, 6), 16) + ', ' + +a.toFixed(3) + ')';
                        }
                    }
                    m = /^rgba?\(([^)]+)\)$/.exec(v);
                    if (m) {
                        var p = m[1].split(/[\s,\/]+/).filter(function(x) { return x; }).map(Number);
                        if (p.length >= 3 && (p.length < 4 || p[3] === 1)) return '#' + hex2(p[0]) + hex2(p[1]) + hex2(p[2]);
                        if (p.length >= 4) return 'rgba(' + p[0] + ', ' + p[1] + ', ' + p[2] + ', ' + p[3] + ')';
                    }
                    return String(s);
                }

                function Gradient(kind, args) { this.__kind = kind; this.__args = args; this.__stops = []; }
                Gradient.prototype.addColorStop = function(offset, color) {
                    offset = Number(offset);
                    if (!(offset >= 0 && offset <= 1)) throw new RangeError('addColorStop: offset 必须在 0~1（IndexSizeError）');
                    this.__stops.push([offset, String(color)]);
                };
                // 图案：源是 Image（种类 1）或 Canvas（种类 2），原生按 id 取像素（game/canvas/image_ops.rs）
                function Pattern(kind, sid, rep) { this.__kind = kind; this.__sid = sid; this.__rep = rep; }
                // drawImage / createPattern 的源：[种类, id]；不是 Image / Canvas 返回 null
                function imageSource(img) {
                    if (img instanceof __GameImage) return [1, img.__id];
                    if (img instanceof GameCanvas) return [2, img.__id];
                    return null;
                }

                var SAVED = ['fillStyle', 'strokeStyle', 'lineWidth', 'globalAlpha', 'font', 'textAlign', 'textBaseline', 'lineCap',
                    'lineJoin', 'miterLimit', 'lineDashOffset', 'shadowBlur', 'shadowColor', 'shadowOffsetX', 'shadowOffsetY',
                    'globalCompositeOperation', 'imageSmoothingEnabled', '__dash', '__m'];
                // 矩阵乘：this × o（与原生 Canvas2DContext::mul 同一顺序：新变换右乘）
                function mul(m, o) {
                    return [m[0] * o[0] + m[2] * o[1], m[1] * o[0] + m[3] * o[1], m[0] * o[2] + m[2] * o[3], m[1] * o[2] + m[3] * o[3],
                        m[0] * o[4] + m[2] * o[5] + m[4], m[1] * o[4] + m[3] * o[5] + m[5]];
                }
                function newImageData(w, h) {
                    if (w && typeof w === 'object') { h = w.height; w = w.width; }
                    w = Math.floor(Math.abs(w)); h = Math.floor(Math.abs(h));
                    if (!(w > 0 && h > 0)) throw new RangeError('createImageData: 宽高必须为正数（IndexSizeError）');
                    return { width: w, height: h, data: new Uint8ClampedArray(w * h * 4) };
                }

                function Context2D(canvas) {
                    var id = canvas.__id;
                    var st = { fillStyle: '#000000', strokeStyle: '#000000', lineWidth: 1, globalAlpha: 1, font: '10px sans-serif',
                        textAlign: 'start', textBaseline: 'alphabetic', lineCap: 'butt', lineJoin: 'miter', miterLimit: 10,
                        lineDashOffset: 0, shadowBlur: 0, shadowColor: 'rgba(0, 0, 0, 0)', shadowOffsetX: 0, shadowOffsetY: 0,
                        globalCompositeOperation: 'source-over', imageSmoothingEnabled: true, __dash: [], __m: [1, 0, 0, 1, 0, 0] };
                    var stack = [], warnedDepth = false;
                    var self = this;
                    this.canvas = canvas;
                    this.__reset = function() { stack = []; st.fillStyle = st.strokeStyle = '#000000'; st.lineWidth = 1; st.globalAlpha = 1; st.font = '10px sans-serif'; st.textAlign = 'start'; st.textBaseline = 'alphabetic'; st.lineCap = 'butt'; st.lineJoin = 'miter'; st.miterLimit = 10; st.lineDashOffset = 0; st.shadowBlur = st.shadowOffsetX = st.shadowOffsetY = 0; st.shadowColor = 'rgba(0, 0, 0, 0)'; st.globalCompositeOperation = 'source-over'; st.__dash = []; st.__m = [1, 0, 0, 1, 0, 0]; N(id, 36, 'alphabetic'); };
                    function style(name, op, v) {
                        if (v instanceof Gradient) {
                            N(id, 44, JSON.stringify({ kind: v.__kind, args: v.__args, stops: v.__stops }), op === 31);
                            st[name] = v; return;
                        }
                        if (v instanceof Pattern) {
                            if (N(id, 48, v.__kind, v.__sid, v.__rep, op === 31)) st[name] = v;
                            return;
                        }
                        if (typeof v !== 'string') return;
                        // 存原串、读时才规范化：游戏每帧设几百次 fillStyle 而几乎不读，规范化（正则 + toLowerCase）
                        // 放在设的时候，一加上占到游戏线程的几个百分点（simpleperf：lre_exec_backtrack / js_string_toLowerCase）
                        if (N(id, op, v)) st[name] = v;
                    }
                    function shadow() { N(id, 42, st.shadowBlur, st.shadowColor, st.shadowOffsetX, st.shadowOffsetY); }
                    var props = {
                        fillStyle: function(v) { style('fillStyle', 30, v); },
                        strokeStyle: function(v) { style('strokeStyle', 31, v); },
                        lineWidth: function(v) { v = Number(v); if (v > 0 && isFinite(v)) { st.lineWidth = v; N(id, 32, v); } },
                        globalAlpha: function(v) { v = Number(v); if (v >= 0 && v <= 1) { st.globalAlpha = v; N(id, 33, v); } },
                        // 原生按 CSS 简写解析，成功返回规范化的读回值（"bold 16px Arial"），失败 null = 保留旧值
                        font: function(v) { var r = N(id, 34, String(v)); if (typeof r === 'string') st.font = r; },
                        textAlign: function(v) { if (['start', 'end', 'left', 'right', 'center'].indexOf(v) >= 0) { st.textAlign = v; N(id, 35, v); } },
                        textBaseline: function(v) { if (['top', 'hanging', 'middle', 'alphabetic', 'ideographic', 'bottom'].indexOf(v) >= 0) { st.textBaseline = v; N(id, 36, v); } },
                        lineCap: function(v) { if (['butt', 'round', 'square'].indexOf(v) >= 0) { st.lineCap = v; N(id, 37, v); } },
                        lineJoin: function(v) { if (['miter', 'round', 'bevel'].indexOf(v) >= 0) { st.lineJoin = v; N(id, 38, v); } },
                        miterLimit: function(v) { v = Number(v); if (v > 0 && isFinite(v)) { st.miterLimit = v; N(id, 39, v); } },
                        lineDashOffset: function(v) { v = Number(v); if (isFinite(v)) { st.lineDashOffset = v; N(id, 41, v); } },
                        shadowBlur: function(v) { v = Number(v); if (v >= 0 && isFinite(v)) { st.shadowBlur = v; shadow(); } },
                        shadowColor: function(v) { if (typeof v === 'string') { st.shadowColor = normColor(v); shadow(); } },
                        shadowOffsetX: function(v) { v = Number(v); if (isFinite(v)) { st.shadowOffsetX = v; shadow(); } },
                        shadowOffsetY: function(v) { v = Number(v); if (isFinite(v)) { st.shadowOffsetY = v; shadow(); } },
                        globalCompositeOperation: function(v) { if (typeof v === 'string') { st.globalCompositeOperation = v; N(id, 43, v); } },
                        // 平滑开关：drawImage 每次调用时带给原生（开 = 双线性，关 = 最近邻）
                        imageSmoothingEnabled: function(v) { st.imageSmoothingEnabled = !!v; }
                    };
                    Object.keys(props).forEach(function(k) {
                        var lazy = k === 'fillStyle' || k === 'strokeStyle';
                        Object.defineProperty(self, k, { get: lazy ? function() { var x = st[k]; return typeof x === 'string' ? normColor(x) : x; }
                            : function() { return st[k]; }, set: props[k], enumerable: true });
                    });
                    Object.keys(OP).forEach(function(k) {
                        var op = OP[k];
                        self[k] = function(a, b, c, d, e, f, g, h) { return N(id, op, a, b, c, d, e, f, g, h); };
                    });
                    // 与状态相关的几个：JS 侧的属性快照跟着压栈 / 出栈
                    this.save = function() {
                        if (N(id, OP.save) === false) {
                            if (!warnedDepth) { warnedDepth = true; console.warn('canvas save() 超过 1024 层，后面的 save 被忽略（是不是少了 restore？）'); }
                            return;
                        }
                        var s = {}; SAVED.forEach(function(k) { s[k] = st[k]; }); stack.push(s);
                    };
                    this.restore = function() {
                        if (!stack.length) return;
                        N(id, OP.restore);
                        var oldFont = st.font;
                        var s = stack.pop(); SAVED.forEach(function(k) { st[k] = s[k]; });
                        // font 存在原生的画布槽里，不在原生的状态栈上：变了就重设一次
                        if (st.font !== oldFont) N(id, 34, st.font);
                    };
                    // 变换：原生改矩阵，JS 记一份给 getTransform 读回（只在参数都是有限数时生效，与原生同一判断）
                    function fin() { for (var i = 0; i < arguments.length; i++) if (!isFinite(arguments[i])) return false; return true; }
                    this.translate = function(x, y) { x = +x; y = +y; if (!fin(x, y)) return; N(id, OP.translate, x, y); st.__m = mul(st.__m, [1, 0, 0, 1, x, y]); };
                    this.scale = function(x, y) { x = +x; y = +y; if (!fin(x, y)) return; N(id, OP.scale, x, y); st.__m = mul(st.__m, [x, 0, 0, y, 0, 0]); };
                    this.rotate = function(a) { a = +a; if (!fin(a)) return; N(id, OP.rotate, a); var c = Math.cos(a), s = Math.sin(a); st.__m = mul(st.__m, [c, s, -s, c, 0, 0]); };
                    this.transform = function(a, b, c, d, e, f) { var m = [a, b, c, d, e, f].map(Number); if (!fin.apply(null, m)) return; N(id, OP.transform, m[0], m[1], m[2], m[3], m[4], m[5]); st.__m = mul(st.__m, m); };
                    this.setTransform = function(a, b, c, d, e, f) {
                        if (a && typeof a === 'object') { f = a.f; e = a.e; d = a.d; c = a.c; b = a.b; a = a.a; }
                        if (a === undefined) { a = 1; b = 0; c = 0; d = 1; e = 0; f = 0; }
                        var m = [a, b, c, d, e, f].map(Number); if (!fin.apply(null, m)) return;
                        N(id, OP.setTransform, m[0], m[1], m[2], m[3], m[4], m[5]); st.__m = m;
                    };
                    this.resetTransform = function() { N(id, OP.resetTransform); st.__m = [1, 0, 0, 1, 0, 0]; };
                    this.getTransform = function() { var m = st.__m; return { a: m[0], b: m[1], c: m[2], d: m[3], e: m[4], f: m[5], is2D: true, isIdentity: m[0] === 1 && m[1] === 0 && m[2] === 0 && m[3] === 1 && m[4] === 0 && m[5] === 0 }; };
                    // fill(path) / stroke(path) / clip(path)：把 Path2D 录下的指令放进当前路径再画（HTML 语义里不改当前路径，
                    // 这里与小程序 canvas2d.js 一样做近似：会替换当前路径，游戏几乎不依赖这个差别）
                    function replay(p) { N(id, OP.beginPath); p.__cmds.forEach(function(c) { N.apply(null, [id, OP[c[0]]].concat(c[1])); }); }
                    ['fill', 'stroke', 'clip'].forEach(function(k) {
                        self[k] = function(p) { if (p && p.__cmds) replay(p); N(id, OP[k]); };
                    });
                    this.measureText = function(t) { return N(id, OP.measureText, String(t)); };
                    this.setLineDash = function(segs) {
                        if (!segs || typeof segs.length !== 'number') return;
                        var a = []; for (var i = 0; i < segs.length; i++) { var v = Number(segs[i]); if (!(v >= 0 && isFinite(v))) return; a.push(v); }
                        if (a.length % 2) a = a.concat(a);
                        st.__dash = a; N(id, 40, a);
                    };
                    this.getLineDash = function() { return st.__dash.slice(); };
                    this.fillText = function(t, x, y, maxWidth) { N(id, OP.fillText, String(t), x, y, maxWidth); };
                    this.strokeText = function(t, x, y, maxWidth) { N(id, OP.strokeText, String(t), x, y, maxWidth); };
                    this.createLinearGradient = function(x0, y0, x1, y1) { return new Gradient('linear', [x0, y0, x1, y1].map(Number)); };
                    this.createRadialGradient = function(x0, y0, r0, x1, y1, r1) {
                        if (r0 < 0 || r1 < 0) throw new RangeError('createRadialGradient: 半径不能为负（IndexSizeError）');
                        return new Gradient('radial', [x0, y0, r0, x1, y1, r1].map(Number));
                    };
                    this.createConicGradient = function(angle, x, y) { return new Gradient('conic', [angle, x, y].map(Number)); };
                    // HTML：repetition 只认四种（空串 / null 当 repeat），别的抛 SyntaxError；图还没加载完返回 null
                    this.createPattern = function(img, rep) {
                        var s = imageSource(img);
                        if (!s) throw new TypeError('createPattern: 第一个参数不是 Image / Canvas');
                        rep = rep === undefined || rep === null || rep === '' ? 'repeat' : String(rep);
                        if (['repeat', 'repeat-x', 'repeat-y', 'no-repeat'].indexOf(rep) < 0) throw new SyntaxError('createPattern: repetition 无效（' + rep + '）');
                        if (s[0] === 1 && !(img.complete && img.width > 0)) return null;
                        return new Pattern(s[0], s[1], rep);
                    };
                    // drawImage(img, dx, dy) / (img, dx, dy, dw, dh) / (img, sx, sy, sw, sh, dx, dy, dw, dh)
                    this.drawImage = function(img, a, b, c, d, e, f, g, h) {
                        var s = imageSource(img);
                        if (!s) throw new TypeError('drawImage: 第一个参数不是 Image / Canvas');
                        var n = arguments.length;
                        if (n !== 3 && n !== 5 && n !== 9) throw new TypeError('drawImage: 参数个数只能是 3 / 5 / 9（实际 ' + n + '）');
                        N(id, 47, s[0], s[1], st.imageSmoothingEnabled !== false, n, a, b, c, d, e, f, g, h);
                    };
                    this.createImageData = newImageData;
                    this.getImageData = function(x, y, w, h) {
                        var buf = N(id, 45, x, y, w, h);
                        return { width: Math.floor(w), height: Math.floor(h), data: new Uint8ClampedArray(buf) };
                    };
                    this.putImageData = function(img, dx, dy) {
                        if (!img || !img.data) throw new TypeError('putImageData: 参数不是 ImageData');
                        N(id, 46, img.data, img.width, img.height, dx, dy);
                    };
                }

                function GameCanvas(info) {
                    var id = info[0], w = info[1], h = info[2];
                    var ctx = null, kind = null;
                    this.__id = id;
                    this.__onscreen = info[3] === 1;
                    var self = this;
                    function resize(nw, nh) {
                        var r = __game_canvas_resize(id, nw, nh);
                        if (r) { w = r[0]; h = r[1]; }
                        if (ctx) ctx.__reset();
                    }
                    Object.defineProperty(this, 'width', { get: function() { return w; }, set: function(v) { resize(v, h); }, enumerable: true });
                    Object.defineProperty(this, 'height', { get: function() { return h; }, set: function(v) { resize(w, v); }, enumerable: true });
                    this.getContext = function(type) {
                        type = String(type);
                        if (kind !== null && kind !== type) return null;
                        if (ctx) return ctx;
                        if (!__game_canvas_ctx(id, type)) return null;
                        kind = type;
                        ctx = new Context2D(self);
                        return ctx;
                    };
                    function save(o) { return JSON.parse(__game_canvas_save(id, JSON.stringify(o || {}))); }
                    this.toTempFilePathSync = function(o) { var r = save(o); if (r.err) throw new Error('toTempFilePathSync:fail ' + r.err); return r.tempFilePath; };
                    this.toTempFilePath = function(o) {
                        o = o || {}; var r = save(o);
                        if (r.err) return __asyncFail(o, 'toTempFilePath', r.err);
                        return __asyncOk(o, 'toTempFilePath', { tempFilePath: r.tempFilePath });
                    };
                    this.toDataURL = function(type, quality) { return __game_canvas_encode(id, String(type || 'image/png'), quality === undefined ? 0.92 : Number(quality)); };
                }

                wx.createCanvas = function() { return new GameCanvas(__game_canvas_new()); };
                wx.createImageData = newImageData;
                wx.createPath2D = function() {
                    // Path2D 的录制在 JS 里（与小程序 canvas2d.js 同一思路），G2 先给形状；fill(path) / stroke(path) 在 G2.3 收尾时接
                    var cmds = [];
                    var p = { __cmds: cmds };
                    ['moveTo', 'lineTo', 'rect', 'arc', 'arcTo', 'ellipse', 'quadraticCurveTo', 'bezierCurveTo', 'closePath'].forEach(function(k) {
                        p[k] = function() { cmds.push([k, [].slice.call(arguments)]); };
                    });
                    p.addPath = function(o) { if (o && o.__cmds) cmds.push.apply(cmds, o.__cmds); };
                    return p;
                };
            })();
