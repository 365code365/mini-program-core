// 小游戏 Chrome 参考实现的 wx 垫片（game-doc/05 §2.4）：只覆盖 2D 夹具用到的 API。它是**像素参照**，不是 API 行为参照。
// game-compare.py 把它、夹具的模块源码、字体（data URL）拼成一个 HTML，headless Chrome 跑完 N 帧后把主画布写进 <pre id=out>。
//
// 与引擎对齐的约定：
// - 窗口 375×667、pixelRatio 2；主画布 = 窗口逻辑尺寸（引擎 GameCanvases 的默认）；
// - 帧 n（1 起）的 rAF 参数与 performance.now = n × 1000 / hz（引擎无头的固定时钟）；
// - wx.loadFont(path) 返回 game-compare.py 预先解析好的字族名（字体已作为 @font-face 注册并等它加载完）。
(function () {
  var cfg = window.__CFG;
  var W = 375, H = 667, now = 0, queue = [], nextId = 1, main = null;
  var mods = window.__MODS;
  function req(base) {
    return function (p) {
      var key = p.replace(/^\.\//, '').replace(/^\//, '');
      if (!/\.js$/.test(key)) key += '.js';
      var m = mods[key];
      if (!m) throw new Error('Cannot find module: ' + p);
      if (!m.exports) { var module = { exports: {} }; m.exports = module.exports; m.fn(module, module.exports, req(key)); m.exports = module.exports; }
      return m.exports;
    };
  }
  window.requestAnimationFrame = function (cb) { var id = nextId++; queue.push([id, cb]); return id; };
  window.cancelAnimationFrame = function (id) { queue = queue.filter(function (q) { return q[0] !== id; }); };
  var perf = { now: function () { return now; } };
  window.wx = {
    createCanvas: function () {
      var c = document.createElement('canvas');
      if (!main) { c.width = W; c.height = H; main = c; } else { c.width = 300; c.height = 150; }
      return c;
    },
    createImage: function () { return new Image(); },
    getSystemInfoSync: function () { return { windowWidth: W, windowHeight: H, screenWidth: W, screenHeight: H, pixelRatio: 2, platform: 'devtools' }; },
    getLaunchOptionsSync: function () { return { query: cfg.query || {}, scene: 1001 }; },
    getPerformance: function () { return perf; },
    loadFont: function (p) { var k = p.replace(/^\//, ''); return cfg.fonts[k] || null; },
    setPreferredFramesPerSecond: function () {},
    onTouchStart: function () {}, onTouchMove: function () {}, onTouchEnd: function () {}, onTouchCancel: function () {},
    onError: function () {}, onShow: function () {}, onHide: function () {}
  };
  window.GameGlobal = window;
  window.__runFrames = function () {
    for (var n = 1; n <= cfg.frames; n++) {
      now = n * 1000 / cfg.hz;
      var batch = queue; queue = [];
      for (var i = 0; i < batch.length; i++) batch[i][1](now);
    }
    var out = document.getElementById('out');
    out.textContent = main ? main.toDataURL('image/png') : 'NO_CANVAS';
  };
  window.__start = function () { req('')('game.js'); window.__runFrames(); };
})();
