// ==================== 高性能模式：画布、图片、字体（WebKit 自己的 Canvas 2D / WebGL，都在 GPU 上） ====================
//
// 语义对齐 QuickJS 模式（game-canvas.js / game-image.js）：
// - 第一次 wx.createCanvas() 是上屏画布（铺满视图，默认尺寸 = 窗口逻辑尺寸），之后的都是离屏画布（默认 300×150）；
// - 图片 / 音频 / 字体的路径：包内相对路径、`wxfile://`、`https://` 都认（经 __wgUrl 换成同源地址，远程的不跨域）；
// - wx.loadFont 同步返回字族名（读不出返回 null），canvas.toTempFilePath(Sync) 落成 wxfile://tmp 的文件。
(function() {
    var G = globalThis, info = __hostSystemInfo, N = __wgNative;

    // 浏览器里 img.src / audio.src 直接给 `wxfile://…`、`https://cdn/…`（跨源）都不行：在原型上换掉 src 的写入
    function patchSrc(proto) {
        var d = proto && Object.getOwnPropertyDescriptor(proto, 'src');
        if (!d || !d.set || d.set.__wg) return;
        var set = function(v) {
            var s = String(v == null ? '' : v);
            this.__wgSrc = s;
            d.set.call(this, /^(wxfile:|https?:|\/\/)/i.test(s) ? __wgUrl(s) : s);
        };
        set.__wg = true;
        Object.defineProperty(proto, 'src', {
            configurable: true, enumerable: d.enumerable,
            // 读回游戏设的原值（官方：img.src 读回的就是设的路径）
            get: function() { return this.__wgSrc !== undefined ? this.__wgSrc : d.get.call(this); },
            set: set
        });
    }
    patchSrc(HTMLImageElement.prototype);
    patchSrc(G.HTMLMediaElement && HTMLMediaElement.prototype);

    // 图片加载失败：onerror / addEventListener('error') 收到的事件带 errMsg（QuickJS 模式 game-image.js 的形状）。
    // 不能在 window 的捕获阶段补：没挂进 DOM 的 Image（游戏里几乎都是）的事件路径只有它自己，window 收不到
    function withErr(img, fn) {
        return function(e) {
            if (e && e.type === 'error' && e.errMsg === undefined) {
                try { e.errMsg = 'loadImage:fail ' + (img.__wgSrc !== undefined ? img.__wgSrc : img.src); } catch (x) {}
            }
            return fn.apply(this, arguments);
        };
    }
    var IP = HTMLImageElement.prototype, onerr = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'onerror');
    if (onerr && onerr.set) {
        Object.defineProperty(IP, 'onerror', {
            configurable: true, enumerable: true,
            get: function() { return this.__wgOnerror === undefined ? null : this.__wgOnerror; },
            set: function(f) { this.__wgOnerror = f; onerr.set.call(this, typeof f === 'function' ? withErr(this, f) : f); }
        });
    }
    var wrapped = new WeakMap(), add = EventTarget.prototype.addEventListener, rm = EventTarget.prototype.removeEventListener;
    IP.addEventListener = function(type, f, o) {
        if (type === 'error' && typeof f === 'function') { var w = wrapped.get(f); if (!w) { w = withErr(this, f); wrapped.set(f, w); } return add.call(this, type, w, o); }
        return add.call(this, type, f, o);
    };
    IP.removeEventListener = function(type, f, o) { return rm.call(this, type, type === 'error' && wrapped.get(f) || f, o); };

    // 适配层按这两个名字认画布 / 图片（instanceof）：就是 DOM 的
    G.__GameCanvas = HTMLCanvasElement;
    G.__GameImage = HTMLImageElement;

    var onscreen = null;
    function decorate(c) {
        // toTempFilePath(Sync)：{x, y, width, height, destWidth, destHeight, fileType, quality}（官方 Canvas.toTempFilePath）
        c.toTempFilePathSync = function(o) {
            var r = save(c, o || {});
            if (r.err) throw new Error('toTempFilePathSync:fail ' + r.err);
            return r.tempFilePath;
        };
        c.toTempFilePath = function(o) {
            o = o || {};
            var r = save(c, o);
            if (r.err) return __asyncFail(o, 'toTempFilePath', r.err);
            return __asyncOk(o, 'toTempFilePath', { tempFilePath: r.tempFilePath });
        };
        return c;
    }
    function save(c, o) {
        var x = Number(o.x) || 0, y = Number(o.y) || 0;
        var w = o.width === undefined ? c.width - x : Number(o.width), h = o.height === undefined ? c.height - y : Number(o.height);
        var dw = o.destWidth === undefined ? w : Number(o.destWidth), dh = o.destHeight === undefined ? h : Number(o.destHeight);
        if (!(w > 0 && h > 0 && dw > 0 && dh > 0)) return { err: 'invalid size' };
        var t = N.createElement('canvas');
        t.width = Math.round(dw); t.height = Math.round(dh);
        t.getContext('2d').drawImage(c, x, y, w, h, 0, 0, t.width, t.height);
        var type = o.fileType === 'jpg' ? 'image/jpeg' : 'image/png';
        var url = t.toDataURL(type, o.quality === undefined ? 1 : Number(o.quality));
        var path = __native_temp_write(url.slice(url.indexOf(',') + 1));
        return path ? { tempFilePath: path } : { err: 'write file failed' };
    }

    wx.createCanvas = function() {
        var c = N.createElement('canvas');
        if (!onscreen) {
            onscreen = c;
            c.width = info.windowWidth;
            c.height = info.windowHeight;
            c.style.cssText = 'position:fixed;left:0;top:0;width:100vw;height:100vh;z-index:0;';
            c.__onscreen = true;
            document.body.appendChild(c);
        }
        return decorate(c);
    };
    G.__wgMainCanvas = function() { return onscreen; };
    wx.createOffscreenCanvas = function(o) {
        var c = N.createElement('canvas');
        if (o && typeof o === 'object') { if (o.width) c.width = o.width; if (o.height) c.height = o.height; }
        else if (arguments.length >= 2) { c.width = arguments[0]; c.height = arguments[1]; }
        return decorate(c);
    };
    wx.createImageData = function(w, h) {
        if (w && typeof w === 'object') { h = w.height; w = w.width; }
        w = Math.floor(Math.abs(w)); h = Math.floor(Math.abs(h));
        if (!(w > 0 && h > 0)) throw new RangeError('createImageData: 宽高必须为正数（IndexSizeError）');
        return new ImageData(w, h);
    };
    wx.createPath2D = function(p) { return new Path2D(p); };

    // 适配层常在实例上 defineProperty('onload', {get, set}) 自己存回调（QuickJS 模式里 GameImage 按属性去读，照样能调到）。
    // DOM 元素上这样做就绕过了浏览器的事件处理器槽位、回调永远不来（Phaser 的默认纹理一直等着，游戏不启动）。
    // 所以：实例上有自己的 onload / onerror 属性时，事件到了按属性取出来调一次
    function ownHandler(img, k) {
        var d = Object.getOwnPropertyDescriptor(img, k);
        if (!d) return null;
        var f = d.get ? d.get.call(img) : d.value;
        return typeof f === 'function' ? f : null;
    }
    wx.createImage = function() {
        var img = new N.Image();
        img.addEventListener('load', function(e) { var f = ownHandler(img, 'onload'); if (f) f.call(img, e); });
        img.addEventListener('error', function(e) { var f = ownHandler(img, 'onerror'); if (f) f.call(img, e); });
        return img;
    };

    // wx.loadFont：同步返回字族名。字族名由原生按 name 表取（与 QuickJS 模式同一个算法），字体本身交给 WebKit：
    // 字节同步取回（同源 XHR），FontFace 按这份字节建 —— 返回之前就已经加进 document.fonts，下一帧 fillText 就用得上
    var fontCache = {};
    wx.loadFont = function(path) {
        path = String(path || '');
        if (fontCache[path] !== undefined) return fontCache[path];
        var family = __wgSync('font_family', [path]) || null;
        if (family) {
            try {
                var x = new N.XHR();
                x.open('GET', __wgUrl(path), false);
                x.overrideMimeType('text/plain; charset=x-user-defined');
                x.send();
                var s = x.responseText, u = new Uint8Array(s.length);
                for (var i = 0; i < s.length; i++) u[i] = s.charCodeAt(i) & 0xff;
                var ff = new N.FontFace(family, u.buffer);
                document.fonts.add(ff);
                ff.load().catch(function() {});
            } catch (e) { family = null; }
        }
        fontCache[path] = family;
        return family;
    };
    // wx.getTextLineHeight（官方：按字号、字族给一行的自然行高）
    var mctx = null;
    wx.getTextLineHeight = function(o) {
        o = o || {};
        mctx = mctx || N.createElement('canvas').getContext('2d');
        var size = Number(o.fontSize) > 0 ? Number(o.fontSize) : 16;
        mctx.font = (o.fontStyle || 'normal') + ' ' + (o.fontWeight || 'normal') + ' ' + size + 'px ' + (o.fontFamily || 'sans-serif');
        var m = mctx.measureText(String(o.text || '中'));
        var h = (m.fontBoundingBoxAscent || 0) + (m.fontBoundingBoxDescent || 0);
        return Math.round(h > 0 ? h : size * 1.2);
    };

    // 共享画布 / 开放数据域：高性能模式不支持（宿主看到 openDataContext 会回落 QuickJS 模式，见 MiniGameViewController）
    wx.getOpenDataContext = function() {
        console.warn('高性能模式不支持开放数据域（game.json 写了 openDataContext 的游戏走普通模式）');
        return { canvas: N.createElement('canvas'), postMessage: function() {} };
    };
    wx.getSharedCanvas = function() { return N.createElement('canvas'); };
})();
