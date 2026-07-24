/* mini-render 运行时垫片（导出 HTML 工程用） */
(function () {
  "use strict";
  var appConfig = null, appInst = null, pageConfig = null, pageInst = null;

  window.App = function (cfg) { appConfig = cfg; };
  window.Page = function (cfg) { pageConfig = cfg; };
  window.Component = function (cfg) {
    var c = {};
    if (cfg.data) c.data = cfg.data;
    var m = cfg.methods || {};
    for (var k in m) c[k] = m[k];
    if (cfg.attached) c.onLoad = cfg.attached;
    if (cfg.ready) c.onReady = cfg.ready;
    pageConfig = c;
  };
  window.getApp = function () { return appInst; };
  window.getCurrentPages = function () { return pageInst ? [pageInst] : []; };
  window.Behavior = function (b) { return b; };

  function parseQuery() {
    var q = location.search.replace(/^\?/, ""), o = {};
    if (!q) return o;
    q.split("&").forEach(function (p) {
      var kv = p.split("="); o[decodeURIComponent(kv[0])] = decodeURIComponent(kv[1] || "");
    });
    return o;
  }

  // 小程序页面路径 -> 相对当前页面的 html 链接
  function routeToHref(url) {
    var qs = "", i = url.indexOf("?");
    if (i >= 0) { qs = url.slice(i); url = url.slice(0, i); }
    url = url.replace(/^\//, "");                 // pages/detail/detail
    var cur = (window.__PAGE_ROUTE__ || "").split("/").slice(0, -1);
    var up = cur.length ? new Array(cur.length + 1).join("../") : "";
    var seg = url.split("/"), leaf = seg[seg.length - 1];
    return up + seg.slice(0, -1).join("/") + "/" + leaf + ".html" + qs;
  }
  function nav(o) { if (o && o.url) location.href = routeToHref(o.url); if (o && o.success) o.success({}); }

  function toast(o) {
    o = o || {};
    var d = document.createElement("div");
    d.textContent = o.title || "";
    d.style.cssText = "position:fixed;left:50%;top:50%;transform:translate(-50%,-50%);z-index:9999;" +
      "background:rgba(0,0,0,.75);color:#fff;padding:14px 20px;border-radius:10px;font-size:14px;max-width:70%;text-align:center;";
    document.body.appendChild(d);
    setTimeout(function () { d.remove(); if (o.success) o.success(); }, o.duration || 1500);
  }

  window.wx = {
    navigateTo: nav, redirectTo: nav, switchTab: nav, reLaunch: nav,
    navigateBack: function () { history.back(); },
    showToast: toast, showLoading: toast,
    hideToast: function () {}, hideLoading: function () {},
    stopPullDownRefresh: function () {},
    showModal: function (o) {
      o = o || {};
      var ok = window.confirm((o.title ? o.title + "\n\n" : "") + (o.content || ""));
      if (o.success) o.success({ confirm: ok, cancel: !ok });
    },
    setStorageSync: function (k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (e) {} },
    getStorageSync: function (k) { try { var v = localStorage.getItem(k); return v ? JSON.parse(v) : ""; } catch (e) { return ""; } },
    removeStorageSync: function (k) { try { localStorage.removeItem(k); } catch (e) {} },
    setStorage: function (o) { this.setStorageSync(o.key, o.data); if (o.success) o.success(); },
    getStorage: function (o) { var v = this.getStorageSync(o.key); if (o.success) o.success({ data: v }); },
    request: function (o) {
      fetch(o.url, { method: o.method || "GET", headers: o.header,
        body: o.data && o.method && o.method !== "GET" ? JSON.stringify(o.data) : undefined })
        .then(function (r) { return r.json().catch(function () { return {}; }); })
        .then(function (d) { if (o.success) o.success({ data: d, statusCode: 200 }); })
        .catch(function (e) { if (o.fail) o.fail(e); })
        .then(function () { if (o.complete) o.complete(); });
    },
    createSelectorQuery: function () {
      var api = { select: function () { return api; }, selectAll: function () { return api; },
        boundingClientRect: function () { return api; }, fields: function () { return api; },
        exec: function (cb) { if (cb) cb([]); } };
      return api;
    },
    getSystemInfoSync: function () { return { windowWidth: 375, windowHeight: 667, pixelRatio: 2, platform: "devtools" }; },
    createCanvasContext: function (id) {
      var el = document.querySelector('[data-canvas-id="' + id + '"]') || document.getElementById(id);
      if (!el || !el.getContext) return noopCanvasCtx();
      if (!el.__sized) { el.width = el.clientWidth || 320; el.height = el.clientHeight || 200; el.__sized = true; }
      return wrapCanvasCtx(el.getContext("2d"));
    },
    nextTick: function (cb) { setTimeout(cb, 0); }
  };

  // WeChat 旧版 canvas API → 标准 2D context 适配
  function wrapCanvasCtx(c) {
    var ctx = {
      setFillStyle: function (v) { c.fillStyle = v; return ctx; },
      setStrokeStyle: function (v) { c.strokeStyle = v; return ctx; },
      setLineWidth: function (v) { c.lineWidth = v; return ctx; },
      setLineCap: function (v) { c.lineCap = v; return ctx; },
      setLineJoin: function (v) { c.lineJoin = v; return ctx; },
      setMiterLimit: function (v) { c.miterLimit = v; return ctx; },
      setGlobalAlpha: function (v) { c.globalAlpha = v; return ctx; },
      setFontSize: function (v) { c.font = v + "px sans-serif"; return ctx; },
      setTextAlign: function (v) { c.textAlign = v; return ctx; },
      setTextBaseline: function (v) { c.textBaseline = v; return ctx; },
      setShadow: function (x, y, b, col) { c.shadowOffsetX = x; c.shadowOffsetY = y; c.shadowBlur = b; c.shadowColor = col; return ctx; },
      draw: function () {} // HTML canvas 立即绘制，draw() 无需缓冲
    };
    ["fillRect", "strokeRect", "clearRect", "beginPath", "closePath", "moveTo", "lineTo",
     "arc", "arcTo", "rect", "fill", "stroke", "fillText", "strokeText", "save", "restore",
     "translate", "rotate", "scale", "transform", "setTransform", "quadraticCurveTo",
     "bezierCurveTo", "clip", "drawImage"].forEach(function (m) {
      ctx[m] = function () { if (c[m]) return c[m].apply(c, arguments); };
    });
    ctx.createLinearGradient = function () { return c.createLinearGradient.apply(c, arguments); };
    ctx.createRadialGradient = function () { return c.createRadialGradient.apply(c, arguments); };
    // 兼容属性式写法
    ["fillStyle", "strokeStyle", "lineWidth", "font", "globalAlpha", "textAlign", "textBaseline", "lineCap", "lineJoin"].forEach(function (p) {
      Object.defineProperty(ctx, p, { get: function () { return c[p]; }, set: function (v) { c[p] = v; } });
    });
    return ctx;
  }
  function noopCanvasCtx() {
    var noop = function () { return noop; };
    return new Proxy({}, { get: function () { return noop; } });
  }

  // dataset：data-ds-id -> {id}, data-ds-item-id -> {itemId}
  function datasetOf(el) {
    var ds = {};
    for (var k in el.dataset) {
      if (k.indexOf("ds") === 0 && k.length > 2) {
        var name = k.slice(2); name = name.charAt(0).toLowerCase() + name.slice(1);
        var raw = el.dataset[k];
        try { ds[name] = JSON.parse(raw); } catch (e) { ds[name] = raw; }
      }
    }
    return ds;
  }
  function mkEvent(el, value) {
    var ds = datasetOf(el);
    return { type: "tap", timeStamp: Date.now(),
      currentTarget: { id: el.id || "", dataset: ds }, target: { id: el.id || "", dataset: ds },
      detail: { value: value } };
  }

  function call(fn, ev) {
    var h = pageInst && pageInst[fn];
    if (typeof h === "function") { try { h.call(pageInst, ev); } catch (err) { console.error(fn, err); } }
  }

  function bindEvents() {
    var map = { tap: "click", input: "input", confirm: "change", change: "change", blur: "blur", focus: "focus", submit: "submit" };
    Object.keys(map).forEach(function (ev) {
      document.querySelectorAll("[data-" + ev + "]").forEach(function (el) {
        // 分组控件的 change 事件单独处理（见 setupComponents）
        if (ev === "change" && (el.classList.contains("wx-radio-group") || el.classList.contains("wx-checkbox-group") || el.classList.contains("wx-switch"))) return;
        el.addEventListener(map[ev], function () {
          var fn = el.getAttribute("data-" + ev);
          call(fn, mkEvent(el, el.value));
        });
      });
    });
  }

  // swiper / radio-group / checkbox-group / switch 等组件行为
  function setupComponents() {
    // ── swiper 轮播：指示点 + 自动播放 ──
    document.querySelectorAll(".wx-swiper").forEach(function (sw) {
      var items = sw.querySelectorAll(".wx-swiper-item");
      if (!items.length) return;
      var vertical = sw.getAttribute("data-vertical") === "true";
      var wrap = document.createElement("div");
      wrap.className = "wx-swiper-wrap";
      sw.parentNode.insertBefore(wrap, sw);
      wrap.appendChild(sw);
      var dots = null;
      if (sw.getAttribute("data-dots") === "true") {
        dots = document.createElement("div");
        dots.className = "wx-swiper-dots";
        items.forEach(function (_, i) {
          var d = document.createElement("i");
          d.className = "wx-swiper-dot" + (i === 0 ? " active" : "");
          dots.appendChild(d);
        });
        wrap.appendChild(dots);
      }
      var idx = 0, n = items.length;
      function update() {
        if (dots) for (var i = 0; i < dots.children.length; i++) dots.children[i].className = "wx-swiper-dot" + (i === idx ? " active" : "");
      }
      function go(i) {
        idx = (i % n + n) % n;
        var pos = idx * (vertical ? sw.clientHeight : sw.clientWidth);
        sw.scrollTo(vertical ? { top: pos, behavior: "smooth" } : { left: pos, behavior: "smooth" });
        update();
      }
      sw.addEventListener("scroll", function () {
        var i = Math.round((vertical ? sw.scrollTop / sw.clientHeight : sw.scrollLeft / sw.clientWidth));
        if (i !== idx) { idx = i; update(); }
      });
      if (sw.getAttribute("data-autoplay") === "true") {
        var iv = parseInt(sw.getAttribute("data-interval"), 10) || 5000;
        setInterval(function () { go(idx + 1); }, iv);
      }
    });

    // ── radio-group：同组互斥 + change 事件 ──
    document.querySelectorAll(".wx-radio-group").forEach(function (g, gi) {
      var name = "wxradio_" + gi;
      g.querySelectorAll(".wx-radio").forEach(function (r) { r.name = name; });
      var fn = g.getAttribute("data-change");
      if (fn) g.addEventListener("change", function () {
        var c = g.querySelector(".wx-radio:checked");
        call(fn, { detail: { value: c ? c.value : "" } });
      });
    });

    // ── checkbox-group：change 事件返回选中值数组 ──
    document.querySelectorAll(".wx-checkbox-group").forEach(function (g) {
      var fn = g.getAttribute("data-change");
      if (fn) g.addEventListener("change", function () {
        var vals = [];
        g.querySelectorAll(".wx-checkbox:checked").forEach(function (c) { vals.push(c.value); });
        call(fn, { detail: { value: vals } });
      });
    });

    // ── switch：点击切换 + change 事件 ──
    document.querySelectorAll(".wx-switch").forEach(function (s) {
      s.addEventListener("click", function () {
        s.classList.toggle("wx-switch-on");
        var fn = s.getAttribute("data-change");
        if (fn) call(fn, { detail: { value: s.classList.contains("wx-switch-on") } });
      });
    });

    // ── slider：拖动实时回调 ──
    document.querySelectorAll(".wx-slider").forEach(function (sl) {
      var fn = sl.getAttribute("data-change") || sl.getAttribute("data-changing");
      if (fn) sl.addEventListener("input", function () {
        call(fn, { detail: { value: Number(sl.value) } });
      });
    });
  }

  document.addEventListener("DOMContentLoaded", function () {
    if (appConfig) {
      appInst = appConfig; appInst.globalData = appConfig.globalData || {};
      try { if (appInst.onLaunch) appInst.onLaunch({}); } catch (e) { console.error(e); }
      try { if (appInst.onShow) appInst.onShow({}); } catch (e) { console.error(e); }
    }
    if (pageConfig) {
      pageInst = pageConfig;
      pageInst.data = pageConfig.data || {};
      pageInst.route = window.__PAGE_ROUTE__;
      pageInst.setData = function (patch, cb) {
        for (var k in patch) this.data[k] = patch[k];
        document.dispatchEvent(new CustomEvent("setdata", { detail: patch }));
        if (cb) cb();
      };
      pageInst.selectComponent = function () { return null; };
      bindEvents();
      setupComponents();
      var q = parseQuery();
      try { if (pageInst.onLoad) pageInst.onLoad(q); } catch (e) { console.error(e); }
      try { if (pageInst.onShow) pageInst.onShow(); } catch (e) { console.error(e); }
      try { if (pageInst.onReady) pageInst.onReady(); } catch (e) { console.error(e); }
    }
  });
})();
