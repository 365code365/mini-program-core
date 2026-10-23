            function __oldGradient(kind, args) {
                var g = { __stops: [], __grad: { kind: kind, args: args } };
                g.addColorStop = function(offset, color) { g.__stops.push([Number(offset), String(color)]); g.__stops.sort(function(p, q) { return p[0] - q[0]; }); };
                return g;
            }
            function __oldStyleCmd(which, v) {
                if (v && typeof v === 'object' && v.__grad) return { type: 'set' + which + 'Gradient', kind: v.__grad.kind, args: v.__grad.args, stops: v.__stops };
                if (v && typeof v === 'object' && v.__pattern) return { type: 'set' + which + 'Pattern', src: v.__pattern.src, repetition: v.__pattern.repetition };
                return { type: 'set' + which + 'Style', color: v };
            }
            var __canvasContexts = {};
            
            wx.createCanvasContext = function(canvasId, component) {
                var ctx = {
                    _canvasId: canvasId,
                    _commands: [],
                    // 渐变对象（createLinearGradient / createCircularGradient）与 type=2d 同一种指令
                    setFillStyle: function(color) { this._commands.push(__oldStyleCmd('Fill', color)); return this; },
                    setStrokeStyle: function(color) { this._commands.push(__oldStyleCmd('Stroke', color)); return this; },
                    setShadow: function(x, y, blur, color) { this._commands.push({ type: 'setShadow', offsetX: x || 0, offsetY: y || 0, blur: blur || 0, color: color || '#000000' }); return this; },
                    createLinearGradient: function(x0, y0, x1, y1) { return __oldGradient('linear', [x0, y0, x1, y1]); },
                    createCircularGradient: function(x, y, r) { return __oldGradient('radial', [x, y, 0, x, y, r]); },
                    setGlobalCompositeOperation: function(op) { this._commands.push({ type: 'setComposite', op: String(op) }); return this; },
                    setLineWidth: function(width) { this._commands.push({ type: 'setLineWidth', width: width }); return this; },
                    setLineCap: function(cap) { this._commands.push({ type: 'setLineCap', cap: cap }); return this; },
                    setLineJoin: function(join) { this._commands.push({ type: 'setLineJoin', join: join }); return this; },
                    setFontSize: function(size) { this._fontSize = Number(size) || 10; this._commands.push({ type: 'setFontSize', size: size }); return this; },
                    setTextAlign: function(align) { this._commands.push({ type: 'setTextAlign', align: align }); return this; },
                    setTextBaseline: function(baseline) { this._commands.push({ type: 'setTextBaseline', baseline: baseline }); return this; },
                    setGlobalAlpha: function(alpha) { this._commands.push({ type: 'setGlobalAlpha', alpha: alpha }); return this; },
                    fillRect: function(x, y, w, h) { this._commands.push({ type: 'fillRect', x: x, y: y, width: w, height: h }); return this; },
                    strokeRect: function(x, y, w, h) { this._commands.push({ type: 'strokeRect', x: x, y: y, width: w, height: h }); return this; },
                    clearRect: function(x, y, w, h) { this._commands.push({ type: 'clearRect', x: x, y: y, width: w, height: h }); return this; },
                    beginPath: function() { this._commands.push({ type: 'beginPath' }); return this; },
                    closePath: function() { this._commands.push({ type: 'closePath' }); return this; },
                    moveTo: function(x, y) { this._commands.push({ type: 'moveTo', x: x, y: y }); return this; },
                    lineTo: function(x, y) { this._commands.push({ type: 'lineTo', x: x, y: y }); return this; },
                    arc: function(x, y, r, s, e, cc) { this._commands.push({ type: 'arc', x: x, y: y, r: r, sAngle: s, eAngle: e, counterclockwise: cc || false }); return this; },
                    quadraticCurveTo: function(cpx, cpy, x, y) { this._commands.push({ type: 'quadraticCurveTo', cpx: cpx, cpy: cpy, x: x, y: y }); return this; },
                    bezierCurveTo: function(cp1x, cp1y, cp2x, cp2y, x, y) { this._commands.push({ type: 'bezierCurveTo', cp1x: cp1x, cp1y: cp1y, cp2x: cp2x, cp2y: cp2y, x: x, y: y }); return this; },
                    fill: function() { this._commands.push({ type: 'fill' }); return this; },
                    stroke: function() { this._commands.push({ type: 'stroke' }); return this; },
                    fillText: function(text, x, y, maxWidth) { this._commands.push({ type: 'fillText', text: text, x: x, y: y, maxWidth: maxWidth }); return this; },
                    strokeText: function(text, x, y, maxWidth) { this._commands.push({ type: 'strokeText', text: text, x: x, y: y, maxWidth: maxWidth }); return this; },
                    drawImage: function(src, sx, sy, sw, sh, dx, dy, dw, dh) {
                        if (arguments.length === 3) { this._commands.push({ type: 'drawImage', src: src, dx: sx, dy: sy }); }
                        else if (arguments.length === 5) { this._commands.push({ type: 'drawImage', src: src, dx: sx, dy: sy, dWidth: sw, dHeight: sh }); }
                        else { this._commands.push({ type: 'drawImage', src: src, sx: sx, sy: sy, sWidth: sw, sHeight: sh, dx: dx, dy: dy, dWidth: dw, dHeight: dh }); }
                        return this;
                    },
                    save: function() { this._commands.push({ type: 'save' }); return this; },
                    restore: function() { this._commands.push({ type: 'restore' }); return this; },
                    translate: function(x, y) { this._commands.push({ type: 'translate', x: x, y: y }); return this; },
                    rotate: function(angle) { this._commands.push({ type: 'rotate', angle: angle }); return this; },
                    scale: function(sx, sy) { this._commands.push({ type: 'scale', scaleX: sx, scaleY: sy }); return this; },
                    // 与 type="2d" 同一套指令（光栅器 canvas/commands.rs）
                    rect: function(x, y, w, h) { this._commands.push({ type: 'rect', x: x, y: y, width: w, height: h }); return this; },
                    arcTo: function(x1, y1, x2, y2, r) { this._commands.push({ type: 'arcTo', x1: x1, y1: y1, x2: x2, y2: y2, r: r }); return this; },
                    clip: function() { this._commands.push({ type: 'clip' }); return this; },
                    setLineDash: function(pattern, offset) { this._commands.push({ type: 'setLineDash', segments: (pattern || []).map(Number), offset: Number(offset) || 0 }); return this; },
                    setMiterLimit: function(v) { this._commands.push({ type: 'setMiterLimit', limit: Number(v) }); return this; },
                    // 旧接口的 setTransform 不受 dpr 预缩放之外的东西影响（pre 按 1）
                    setTransform: function(a, b, c, d, e, f) { this._commands.push({ type: 'setTransform', a: a, b: b, c: c, d: d, e: e, f: f, pre: [1, 1] }); return this; },
                    transform: function(a, b, c, d, e, f) { this._commands.push({ type: 'transform', a: a, b: b, c: c, d: d, e: e, f: f }); return this; },
                    // createPattern(image, repetition)：image 是图片路径；setFillStyle(pattern) 时发 setFillPattern
                    createPattern: function(image, repetition) { return { __pattern: { src: String(image && image.src || image || ''), repetition: repetition || 'repeat' } }; },
                    // 同步量宽：按当前字号（setFontSize / font），与画出来的字同一套字体度量
                    measureText: function(text) { return { width: Number(__native_measure_text(String(this._fontSize || 10), String(text == null ? '' : text))) }; },
                    setFont: function(f) { var m = /(\d+(?:\.\d+)?)px/.exec(String(f || '')); if (m) { this._fontSize = Number(m[1]); this._commands.push({ type: 'setFontSize', size: this._fontSize }); } return this; },
                    draw: function(reserve, callback) {
                        if (typeof __native_canvas_draw === 'function') {
                            __native_canvas_draw(this._canvasId, JSON.stringify(this._commands));
                        }
                        if (!reserve) { this._commands = []; }
                        if (typeof callback === 'function') { setTimeout(callback, 0); }
                    }
                };
                __canvasContexts[canvasId] = ctx;
                return ctx;
            };
