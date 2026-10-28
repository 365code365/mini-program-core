// Khronos WebGL 一致性测试（CTS）的最小 DOM 外壳（A8.1，tools/webgl-cts/run.py 生成 game.js 时放在最前面）。
// 每个测试页里的 <script> 按原顺序在全局作用域执行（(0, eval)），<canvas id> 与着色器 <script id type=x-shader/…> 由 run.py 抽出来放进 __CTS_DOM。
// 结果：js-test-pre.js 的 RESULTS 计数；结束时（notifyFinishedToHarness → nonKhronosFrameworkNotifyDone）打一行 T:CTS done。
var g = GameGlobal;
g.window = g; g.self = g; g.top = g; g.parent = g;
var DOM = __CTS_DOM, made = {};
// js-test-pre.js 会把 console.log 换成空函数、也有自己的 nonKhronosFrameworkNotifyDone：结果走这份留下来的
var __ctsLog = console.log;
function node(tag) {
    var n = { tagName: String(tag).toUpperCase(), nodeName: String(tag).toUpperCase(), nodeType: 1, style: {}, children: [], childNodes: [], attributes: {},
        classList: { add: function () {}, remove: function () {}, contains: function () { return false; } },
        appendChild: function (c) {
            this.children.push(c); this.childNodes.push(c);
            // finishTest() 往 body 里插 <script src=".../js-test-post.js">：照浏览器那样执行它
            if (c && c.tagName === 'SCRIPT' && /js-test-post\.js$/.test(c.src || '')) setTimeout(function () { (0, eval)(__CTS_POST); }, 0);
            return c;
        },
        removeChild: function (c) { return c; }, insertBefore: function (c) { return c; }, setAttribute: function (k, v) { this.attributes[k] = v; this[k] = v; },
        getAttribute: function (k) { return this.attributes[k]; }, addEventListener: function () {}, removeEventListener: function () {},
        get innerHTML() { return ''; }, set innerHTML(v) {}, textContent: '', innerText: '' };
    return n;
}
function canvasEl(c, w, h) {
    if (w !== undefined) { c.width = w; c.height = h; }
    c.style = {}; c.tagName = 'CANVAS'; c.nodeName = 'CANVAS'; c.nodeType = 1;
    c.getBoundingClientRect = function () { return { left: 0, top: 0, width: c.width, height: c.height, right: c.width, bottom: c.height }; };
    c.setAttribute = function (k, v) { if (k === 'width' || k === 'height') c[k] = Number(v); };
    c.getAttribute = function (k) { return c[k]; };
    Object.defineProperty(c, 'clientWidth', { get: function () { return c.width; } });
    Object.defineProperty(c, 'clientHeight', { get: function () { return c.height; } });
    c.parentNode = document.body; c.ownerDocument = document;
    return c;
}
var document = {
    body: node('body'), head: node('head'), documentElement: node('html'),
    getElementById: function (id) {
        if (made[id]) return made[id];
        var d = DOM[id];
        // js-test-pre 往这几个 div 里写；别的不认识的 id 返回 null（wtu 据此判断「是元素 id 还是着色器源码」）
        if (!d) return /^(description|console|testDiv|footer)$/.test(id) ? (made[id] = node('div')) : null;
        if (d.tag === 'canvas') return (made[id] = canvasEl(wx.createCanvas(), d.width, d.height));
        var s = node('script'); s.text = s.textContent = s.innerHTML = d.text; s.type = d.type; s.id = id;
        return (made[id] = s);
    },
    createElement: function (tag) {
        tag = String(tag).toLowerCase();
        if (tag === 'canvas') return canvasEl(wx.createCanvas(), 300, 150);
        if (tag === 'img') return wx.createImage();
        return node(tag);
    },
    createElementNS: function (ns, tag) { return document.createElement(tag); },
    createTextNode: function (t) { return { nodeValue: t, textContent: t, nodeType: 3 }; },
    getElementsByTagName: function (t) {
        t = String(t).toLowerCase();
        if (t === 'script') return __CTS_SCRIPTS.map(function (src) { return { src: src }; });
        if (t === 'canvas') return Object.keys(DOM).filter(function (k) { return DOM[k].tag === 'canvas'; }).map(function (k) { return document.getElementById(k); });
        return [];
    },
    querySelector: function () { return null; }, querySelectorAll: function () { return []; },
    addEventListener: function () {}, readyState: 'complete', location: { pathname: __CTS_PATH, search: '', href: __CTS_PATH }
};
g.document = document;
g.location = document.location;
g.navigator = { userAgent: 'Mozilla/5.0 MiniGame', platform: 'mini', language: 'zh-CN' };
g.HTMLCanvasElement = __GameCanvas; g.HTMLImageElement = __GameImage;
g.Image = function () { return wx.createImage(); };
var __ctsLoad = [];
var __winL = {};
g.addEventListener = function (t, f) { if (t === 'load' || t === 'DOMContentLoaded') __ctsLoad.push(f); else (__winL[t] = __winL[t] || []).push(f); };
g.removeEventListener = function (t, f) { var l = __winL[t]; if (l) { var i = l.indexOf(f); if (i >= 0) l.splice(i, 1); } };
// wtu.setZeroTimeout 靠 window.postMessage + 'message' 事件
g.postMessage = function (data) { setTimeout(function () { (__winL.message || []).slice().forEach(function (f) { f({ source: g, data: data, stopPropagation: function () {} }); }); }, 0); };
document.addEventListener = g.addEventListener;
g.screen = { width: 375, height: 667, availWidth: 375, availHeight: 667 };
g.devicePixelRatio = 1;
g.URL = g.URL || function (href) { this.href = String(href); this.search = ''; this.searchParams = new g.URLSearchParams(''); this.toString = function () { return this.href; }; };
// 浏览器把带 id 的元素挂成 window 的同名属性（有的页面直接写 canvas.width）
Object.keys(DOM).forEach(function (id) {
    if (DOM[id].tag === 'canvas' && !(id in g)) Object.defineProperty(g, id, { get: function () { return document.getElementById(id); }, configurable: true });
});
g.getComputedStyle = function () { return { getPropertyValue: function () { return ''; } }; };
g.URLSearchParams = g.URLSearchParams || function () { return { get: function () { return null; }, set: function () {}, toString: function () { return ''; } }; };
// run.py 在 js-test-pre.js 之后插这一段：接住结束通知与失败子项
g.__ctsHook = function () {
    g.nonKhronosFrameworkNotifyDone = function () { __ctsLog('T:CTS done ' + JSON.stringify({ pass: RESULTS.pass, fail: RESULTS.fail })); };
    var tf = g.testFailed;
    g.testFailed = function (m) { __ctsLog('FAIL ' + m); return tf.apply(this, arguments); };
};

// 同步 XHR（wtu.readFile / loadTextFileAsync 读着色器、测试列表）：按测试页的相对路径解析，读包里的 cts/ 镜像（run.py 建的符号链接）
function resolve(url) {
    if (/^[a-z]+:/.test(url)) return null;
    var base = __CTS_PATH.split('/'); base.pop();
    url.split('/').forEach(function (p) { if (p === '..') base.pop(); else if (p !== '.' && p !== '') base.push(p); });
    return 'cts' + base.join('/');
}
g.XMLHttpRequest = function () { this.readyState = 0; this.status = 0; this.responseText = ''; };
g.XMLHttpRequest.prototype.open = function (m, url, async) { this._url = url; this._async = async !== false; };
g.XMLHttpRequest.prototype.overrideMimeType = function () {};
g.XMLHttpRequest.prototype.setRequestHeader = function () {};
g.XMLHttpRequest.prototype.send = function () {
    var self = this, p = resolve(this._url);
    try { this.responseText = this.response = wx.getFileSystemManager().readFileSync(p, 'utf8'); this.status = 200; }
    catch (e) { this.status = 404; this.responseText = ''; }
    this.readyState = 4;
    var fire = function () { if (self.onreadystatechange) self.onreadystatechange(); if (self.status === 200 && self.onload) self.onload(); if (self.status !== 200 && self.onerror) self.onerror(); };
    if (this._async) setTimeout(fire, 0); else fire();
};
