// 最小 weapp-adapter（拷自 sample-game/_lib/adapter.js，game-doc/04 G9.1；包要自包含，所以每个 3D 示例各一份）：给 three.js / PixiJS / Phaser 这类面向浏览器的引擎补 window / document / navigator /
// Image / 画布元素 / 触摸与指针事件。与官方 weapp-adapter 同一思路（第一块 wx.createCanvas 是上屏画布、挂在 window.canvas），只补这几个引擎用到的。
// require 一次即可；返回上屏画布。
var g = GameGlobal;
if (g.__adapter) { module.exports = g.__adapter; return; }
var info = wx.getSystemInfoSync();
// 与 js/base.js 共用上屏主画布（第一次 wx.createCanvas() 才是它）
var canvas = g.__mainCanvas || (g.__mainCanvas = wx.createCanvas());
function listen(o) {
    var L = {};
    // 画布自带 addEventListener（game-canvas.js）：事件走它自己的表
    if (o.addEventListener && o.dispatchEvent) { o.__fire = function (t, e) { e.type = t; o.dispatchEvent(e); }; return o; }
    o.addEventListener = o.addEventListener || function (t, f) { (L[t] = L[t] || []).push(f); };
    o.removeEventListener = o.removeEventListener || function (t, f) { var l = L[t]; if (l) { var i = l.indexOf(f); if (i >= 0) l.splice(i, 1); } };
    o.__fire = function (t, e) { (L[t] || []).slice().forEach(function (f) { f.call(o, e); }); };
    if (!o.dispatchEvent) o.dispatchEvent = function (e) { o.__fire(e.type, e); return true; };
    return o;
}
function element(c) {
    c.style = c.style || {};
    c.tagName = 'CANVAS'; c.nodeName = 'CANVAS'; c.nodeType = 1;
    c.clientWidth = c.width; c.clientHeight = c.height;
    if (c === canvas) { c.clientWidth = info.windowWidth; c.clientHeight = info.windowHeight; }
    c.getBoundingClientRect = function () { return { x: 0, y: 0, left: 0, top: 0, width: c.clientWidth, height: c.clientHeight, right: c.clientWidth, bottom: c.clientHeight }; };
    c.setAttribute = c.removeAttribute = function () {};
    c.focus = c.blur = function () {};
    c.setPointerCapture = c.releasePointerCapture = function () {};
    c.parentElement = c.parentNode = document.body;
    listen(c);
    return c;
}
var rect = function () { return { x: 0, y: 0, left: 0, top: 0, width: info.windowWidth, height: info.windowHeight, right: info.windowWidth, bottom: info.windowHeight }; };
var body = listen({ style: {}, getBoundingClientRect: rect, clientWidth: info.windowWidth, clientHeight: info.windowHeight, appendChild: function (x) { return x; }, removeChild: function (x) { return x; }, insertBefore: function (x) { return x; }, contains: function () { return true; } });
var document = listen({
    readyState: 'complete', visibilityState: 'visible', hidden: false, body: body, head: body, documentElement: listen({ style: {}, getBoundingClientRect: rect, clientWidth: info.windowWidth, clientHeight: info.windowHeight }),
    createElement: function (tag) {
        tag = String(tag).toLowerCase();
        if (tag === 'canvas') return element(wx.createCanvas());
        if (tag === 'img') return new Image();
        return listen({ tagName: tag.toUpperCase(), style: {}, appendChild: function (x) { return x; }, setAttribute: function () {}, getContext: function () { return null; } });
    },
    createElementNS: function (ns, tag) { return document.createElement(tag); },
    getElementById: function () { return canvas; },
    getElementsByTagName: function (t) { return String(t).toLowerCase() === 'canvas' ? [canvas] : []; },
    querySelector: function () { return canvas; },
    querySelectorAll: function () { return [canvas]; }
});
body.ownerDocument = document;
function Image() { var i = wx.createImage(); i.style = {}; i.tagName = 'IMG'; listen(i); var ol, oe;
    Object.defineProperty(i, 'onload', { get: function () { return ol; }, set: function (f) { ol = f; } });
    return i; }
g.window = g; g.self = g; g.top = g; g.parent = g;
g.document = document; g.canvas = element(canvas);
g.navigator = { userAgent: 'Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 (KHTML, like Gecko) Mobile MiniGame', language: 'zh-CN', languages: ['zh-CN'],
    platform: info.platform, maxTouchPoints: 10, onLine: true, appVersion: '5.0', vendor: '', hardwareConcurrency: 8 };
g.location = { href: 'game.js', protocol: 'https:', host: '', hostname: '', search: '', hash: '', pathname: '/', origin: '', reload: function () {} };
g.screen = { width: info.screenWidth, height: info.screenHeight, availWidth: info.screenWidth, availHeight: info.screenHeight, orientation: { angle: 0, type: 'portrait-primary' } };
g.innerWidth = info.windowWidth; g.innerHeight = info.windowHeight; g.devicePixelRatio = info.pixelRatio;
g.outerWidth = info.windowWidth; g.outerHeight = info.windowHeight; g.scrollX = g.scrollY = g.pageXOffset = g.pageYOffset = 0;
g.Image = Image; g.HTMLImageElement = __GameImage; g.HTMLCanvasElement = __GameCanvas; g.HTMLVideoElement = function () {};
g.ImageBitmap = function () {}; g.OffscreenCanvas = undefined;
g.HTMLElement = g.Element = g.Node = function () {};
g.getComputedStyle = function () { return { getPropertyValue: function () { return ''; } }; };
g.matchMedia = function () { return { matches: false, addListener: function () {}, removeListener: function () {}, addEventListener: function () {}, removeEventListener: function () {} }; };
g.localStorage = { getItem: function (k) { var v = wx.getStorageSync(k); return v === '' ? null : v; }, setItem: function (k, v) { wx.setStorageSync(k, String(v)); },
    removeItem: function (k) { wx.removeStorageSync(k); }, clear: function () { wx.clearStorageSync(); } };
g.focus = function () {}; g.scrollTo = function () {};
listen(g);
g.Event = function (t) { this.type = t; };
g.TouchEvent = function (t) { this.type = t; };
g.PointerEvent = g.MouseEvent = function (t) { this.type = t; };
// 触摸 → touch* 与 pointer* 事件（发给画布、document、window）
function touches(list) { return Array.prototype.map.call(list || [], function (t) { return { identifier: t.identifier, pageX: t.clientX, pageY: t.clientY, clientX: t.clientX, clientY: t.clientY, screenX: t.clientX, screenY: t.clientY, target: canvas, force: 1 }; }); }
var PT = { touchstart: 'pointerdown', touchmove: 'pointermove', touchend: 'pointerup', touchcancel: 'pointercancel' };
function fire(type, r) {
    var e = { type: type, touches: touches(r.touches), targetTouches: touches(r.touches), changedTouches: touches(r.changedTouches), timeStamp: r.timeStamp,
        target: canvas, currentTarget: canvas, preventDefault: function () {}, stopPropagation: function () {} };
    [canvas, document, g].forEach(function (o) { o.__fire(type, e); });
    e.changedTouches.forEach(function (t) {
        var p = { type: PT[type], pointerId: t.identifier, pointerType: 'touch', isPrimary: true, clientX: t.clientX, clientY: t.clientY, pageX: t.clientX, pageY: t.clientY,
            offsetX: t.clientX, offsetY: t.clientY, screenX: t.clientX, screenY: t.clientY, x: t.clientX, y: t.clientY, button: 0, buttons: type === 'touchend' ? 0 : 1,
            pressure: 0.5, width: 1, height: 1, target: canvas, currentTarget: canvas, timeStamp: r.timeStamp, preventDefault: function () {}, stopPropagation: function () {} };
        [canvas, document, g].forEach(function (o) { o.__fire(p.type, p); });
    });
}
wx.onTouchStart(function (r) { fire('touchstart', r); });
wx.onTouchMove(function (r) { fire('touchmove', r); });
wx.onTouchEnd(function (r) { fire('touchend', r); });
wx.onTouchCancel(function (r) { fire('touchcancel', r); });
g.__adapter = canvas;
module.exports = canvas;
