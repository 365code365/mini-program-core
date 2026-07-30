            var __canvasContexts = {};
            
            wx.createCanvasContext = function(canvasId, component) {
                var ctx = {
                    _canvasId: canvasId,
                    _commands: [],
                    setFillStyle: function(color) { this._commands.push({ type: 'setFillStyle', color: color }); return this; },
                    setStrokeStyle: function(color) { this._commands.push({ type: 'setStrokeStyle', color: color }); return this; },
                    setLineWidth: function(width) { this._commands.push({ type: 'setLineWidth', width: width }); return this; },
                    setLineCap: function(cap) { this._commands.push({ type: 'setLineCap', cap: cap }); return this; },
                    setLineJoin: function(join) { this._commands.push({ type: 'setLineJoin', join: join }); return this; },
                    setFontSize: function(size) { this._commands.push({ type: 'setFontSize', size: size }); return this; },
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
