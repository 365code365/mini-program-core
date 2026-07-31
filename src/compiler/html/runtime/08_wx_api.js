  // ───────── 页面跳转 & wx.* ─────────
  function parseQuery() { var q = location.search.replace(/^\?/, ""), o = {}; if (!q) return o; q.split("&").forEach(function (p) { var kv = p.split("="); o[decodeURIComponent(kv[0])] = decodeURIComponent(kv[1] || ""); }); return o; }
  function routeToHref(url) {
    var qs = "", i = url.indexOf("?"); if (i >= 0) { qs = url.slice(i); url = url.slice(0, i); }
    url = url.replace(/^\//, "");
    var cur = (window.__PAGE_ROUTE__ || "").split("/").slice(0, -1);
    var up = cur.length ? new Array(cur.length + 1).join("../") : "";
    var seg = url.split("/"), leaf = seg[seg.length - 1];
    return up + seg.slice(0, -1).join("/") + "/" + leaf + ".html" + qs;
  }
  function nav(o) { if (o && o.url) location.href = routeToHref(o.url); if (o && o.success) o.success({}); if (o && o.complete) o.complete({}); }
  var toastEl = null;
  function toast(o) {
    o = o || {};
    if (toastEl) { toastEl.remove(); toastEl = null; }
    var d = document.createElement("div");
    var big = o.icon && o.icon !== "none";
    var ic = "";
    if (o.icon === "success") ic = "<div style='font-size:40px;line-height:1;margin-bottom:6px;'>\u2713</div>";
    else if (o.icon === "error" || o.icon === "fail") ic = "<div style='font-size:40px;line-height:1;margin-bottom:6px;'>\u2715</div>";
    else if (o.icon === "loading") ic = "<div style='width:34px;height:34px;margin:0 auto 8px;border:3px solid rgba(255,255,255,.35);border-top-color:#fff;border-radius:50%;animation:wxspin .8s linear infinite;'></div>";
    d.innerHTML = ic + "<div>" + esc(o.title || "") + "</div>";
    d.style.cssText = "position:fixed;left:50%;top:50%;transform:translate(-50%,-50%);z-index:10000;background:rgba(0,0,0,.72);color:#fff;border-radius:12px;text-align:center;font-size:14px;pointer-events:none;" + (big ? "padding:20px 24px;min-width:120px;" : "padding:11px 18px;max-width:70%;");
    document.body.appendChild(d);
    toastEl = d;
    if (o.icon !== "loading" || o.duration) {
      setTimeout(function () { if (d.parentNode) d.remove(); if (toastEl === d) toastEl = null; if (o.success) o.success(); if (o.complete) o.complete(); }, o.duration || 1500);
    }
  }
  function hideToastNow() { if (toastEl) { toastEl.remove(); toastEl = null; } }

  window.wx = {
    navigateTo: nav, redirectTo: nav, switchTab: nav, reLaunch: nav,
    navigateBack: function () { history.back(); },
    showToast: toast,
    showLoading: function (o) { o = o || {}; o.icon = "loading"; toast(o); },
    hideToast: hideToastNow, hideLoading: hideToastNow,
    createAnimation: createAnimation,
    startPullDownRefresh: function (o) { pullDown.start(); o && o.success && o.success(); o && o.complete && o.complete(); },
    stopPullDownRefresh: function (o) { pullDown.stop(); o && o.success && o.success(); o && o.complete && o.complete(); },
    showModal: function (o) {
      o = o || {};
      var mask = document.createElement("div");
      mask.style.cssText = "position:fixed;inset:0;left:0;top:0;right:0;bottom:0;background:rgba(0,0,0,.5);z-index:10001;display:flex;align-items:center;justify-content:center;";
      var showCancel = o.showCancel !== false;
      var box = document.createElement("div");
      box.style.cssText = "width:280px;background:#fff;border-radius:14px;overflow:hidden;box-shadow:0 10px 40px rgba(0,0,0,.2);";
      box.innerHTML =
        "<div style='padding:26px 20px 20px;text-align:center;'>" +
        (o.title ? "<div style='font-size:17px;font-weight:600;color:#111;margin-bottom:8px;'>" + esc(o.title) + "</div>" : "") +
        "<div style='font-size:14px;color:#666;line-height:1.5;'>" + esc(o.content || "") + "</div></div>" +
        "<div style='display:flex;border-top:1px solid #eee;'>" +
        (showCancel ? "<button class='wxmc' style='flex:1;border:0;background:none;padding:13px;font-size:16px;color:" + (o.cancelColor || "#333") + ";border-right:1px solid #eee;cursor:pointer;'>" + esc(o.cancelText || "取消") + "</button>" : "") +
        "<button class='wxmo' style='flex:1;border:0;background:none;padding:13px;font-size:16px;font-weight:500;color:" + (o.confirmColor || "#07c160") + ";cursor:pointer;'>" + esc(o.confirmText || "确定") + "</button></div>";
      mask.appendChild(box); document.body.appendChild(mask);
      function done(r) { mask.remove(); if (o.success) o.success(r); if (o.complete) o.complete(); }
      var ok = box.querySelector(".wxmo"), no = box.querySelector(".wxmc");
      if (ok) ok.addEventListener("click", function () { done({ confirm: true, cancel: false }); });
      if (no) no.addEventListener("click", function () { done({ confirm: false, cancel: true }); });
    },
    setStorageSync: function (k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (e) {} },
    getStorageSync: function (k) { try { var v = localStorage.getItem(k); return v ? JSON.parse(v) : ""; } catch (e) { return ""; } },
    removeStorageSync: function (k) { try { localStorage.removeItem(k); } catch (e) {} },
    setStorage: function (o) { this.setStorageSync(o.key, o.data); if (o.success) o.success(); },
    getStorage: function (o) { var v = this.getStorageSync(o.key); if (o.success) o.success({ data: v }); },
    request: function (o) { fetch(o.url, { method: o.method || "GET", headers: o.header, body: o.data && o.method && o.method !== "GET" ? JSON.stringify(o.data) : undefined }).then(function (r) { return r.json().catch(function () { return {}; }); }).then(function (d) { if (o.success) o.success({ data: d, statusCode: 200 }); }).catch(function (e) { if (o.fail) o.fail(e); }).then(function () { if (o.complete) o.complete(); }); },
    createSelectorQuery: function () { var api = { select: function () { return api; }, selectAll: function () { return api; }, boundingClientRect: function () { return api; }, fields: function () { return api; }, exec: function (cb) { if (cb) cb([]); } }; return api; },
    getSystemInfoSync: function () { return { windowWidth: 375, windowHeight: 667, pixelRatio: 2, platform: "devtools" }; },
    createCanvasContext: function (id) { var el = document.querySelector('[data-canvas-id="' + id + '"]') || document.getElementById(id); if (!el || !el.getContext) return noopCanvasCtx(); if (!el.__sized) { el.width = el.clientWidth || 320; el.height = el.clientHeight || 200; el.__sized = true; } return wrapCanvasCtx(el.getContext("2d")); },
    nextTick: function (cb) { setTimeout(cb, 0); }
  };

  function wrapCanvasCtx(c) {
    var ctx = {
      setFillStyle: function (v) { c.fillStyle = v; return ctx; }, setStrokeStyle: function (v) { c.strokeStyle = v; return ctx; },
      setLineWidth: function (v) { c.lineWidth = v; return ctx; }, setLineCap: function (v) { c.lineCap = v; return ctx; },
      setLineJoin: function (v) { c.lineJoin = v; return ctx; }, setMiterLimit: function (v) { c.miterLimit = v; return ctx; },
      setGlobalAlpha: function (v) { c.globalAlpha = v; return ctx; }, setFontSize: function (v) { c.font = v + "px sans-serif"; return ctx; },
      setTextAlign: function (v) { c.textAlign = v; return ctx; }, setTextBaseline: function (v) { c.textBaseline = v; return ctx; },
      setShadow: function (x, y, b, col) { c.shadowOffsetX = x; c.shadowOffsetY = y; c.shadowBlur = b; c.shadowColor = col; return ctx; }, draw: function () {}
    };
    ["fillRect", "strokeRect", "clearRect", "beginPath", "closePath", "moveTo", "lineTo", "arc", "arcTo", "rect", "fill", "stroke", "fillText", "strokeText", "save", "restore", "translate", "rotate", "scale", "transform", "setTransform", "quadraticCurveTo", "bezierCurveTo", "clip", "drawImage"].forEach(function (m) { ctx[m] = function () { if (c[m]) return c[m].apply(c, arguments); }; });
    ctx.createLinearGradient = function () { return c.createLinearGradient.apply(c, arguments); };
    ctx.createRadialGradient = function () { return c.createRadialGradient.apply(c, arguments); };
    ["fillStyle", "strokeStyle", "lineWidth", "font", "globalAlpha", "textAlign", "textBaseline", "lineCap", "lineJoin"].forEach(function (p) { Object.defineProperty(ctx, p, { get: function () { return c[p]; }, set: function (v) { c[p] = v; } }); });
    return ctx;
  }
  function noopCanvasCtx() { var noop = function () { return noop; }; return new Proxy({}, { get: function () { return noop; } }); }

  // ───────── 下拉刷新（与原生端同一套语义：64px 阈值 + 三点指示器） ─────────
  // ───────── wx.createAnimation（与原生端 API 与语义一致） ─────────
  function createAnimation(option) {
    option = option || {};
    var def = {
      duration: option.duration === undefined ? 400 : option.duration,
      delay: option.delay || 0,
      timingFunction: option.timingFunction || "linear"
    };
    var actions = [], pending = [];
    function rec(type, args) { pending.push({ type: type, args: args }); return api; }
    var api = {
      translate: function (x, y) { return rec("translate", [x || 0, y || 0]); },
      translateX: function (v) { return rec("translateX", [v || 0]); },
      translateY: function (v) { return rec("translateY", [v || 0]); },
      rotate: function (d) { return rec("rotate", [d || 0]); },
      rotateZ: function (d) { return rec("rotate", [d || 0]); },
      scale: function (sx, sy) { return rec("scale", [sx === undefined ? 1 : sx, sy === undefined ? sx : sy]); },
      scaleX: function (v) { return rec("scaleX", [v === undefined ? 1 : v]); },
      scaleY: function (v) { return rec("scaleY", [v === undefined ? 1 : v]); },
      skew: function (x, y) { return rec("skew", [x || 0, y || 0]); },
      opacity: function (v) { return rec("opacity", [v]); },
      backgroundColor: function (c) { return rec("backgroundColor", [c]); },
      width: function (v) { return rec("width", [v]); },
      height: function (v) { return rec("height", [v]); },
      step: function (cfg) {
        cfg = cfg || {};
        actions.push({
          animates: pending,
          option: {
            transition: {
              duration: cfg.duration === undefined ? def.duration : cfg.duration,
              delay: cfg.delay === undefined ? def.delay : cfg.delay,
              timingFunction: cfg.timingFunction || def.timingFunction
            }
          }
        });
        pending = [];
        return api;
      },
      export: function () { var o = { actions: actions }; actions = []; return o; }
    };
    return api;
  }

