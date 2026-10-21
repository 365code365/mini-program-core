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
  // ───────── 提示框 / 弹窗：与原生端同一套语义（prelude/ui.js）与外观（host::ui_overlay::geometry，WeUI 2.x） ─────────
  // Toast 与 Loading 是同一个槽位：show 一个就顶掉另一个；hide 默认互相能关，noConflict 只关自己那种
  var FB_CSS =
    ".wxtip-wrp{position:fixed;left:0;right:0;top:40%;transform:translateY(-50%);display:flex;justify-content:center;z-index:5000;pointer-events:none;}" +
    ".wxtip-mask{position:fixed;left:0;top:0;right:0;bottom:0;z-index:4999;}" +
    ".wxtip{background:#4c4c4c;color:rgba(255,255,255,.9);border-radius:12px;text-align:center;line-height:1.4;box-sizing:border-box;display:flex;flex-direction:column;align-items:center;justify-content:center;}" +
    ".wxtip_icon{width:136px;height:136px;}" +
    ".wxtip_icon .wxtip-i{width:40px;height:40px;display:block;flex-shrink:0;}" +
    ".wxtip_icon .wxtip-i+.wxtip-t{margin-top:16px;}" +
    ".wxtip_icon .wxtip-t{font-size:17px;max-width:119px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;}" +
    ".wxtip_text{min-width:152px;max-width:216px;padding:12px;}" +
    ".wxtip_text .wxtip-t{font-size:14px;white-space:pre-wrap;word-break:break-all;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden;}" +
    ".wxtip-spin{position:relative;color:#ededed;animation:wxtipspin 1s steps(60,end) infinite;}" +
    ".wxtip-spin:before,.wxtip-spin:after{content:'';position:absolute;top:0;width:20px;height:40px;box-sizing:border-box;border:4px solid currentColor;}" +
    ".wxtip-spin:before{left:0;border-right-width:0;border-radius:40px 0 0 40px;-webkit-mask-image:linear-gradient(180deg,#000 8%,rgba(0,0,0,.3) 95%);mask-image:linear-gradient(180deg,#000 8%,rgba(0,0,0,.3) 95%);}" +
    ".wxtip-spin:after{left:20px;border-left-width:0;border-radius:0 40px 40px 0;-webkit-mask-image:linear-gradient(180deg,rgba(0,0,0,0) 8%,rgba(0,0,0,.3) 95%);mask-image:linear-gradient(180deg,rgba(0,0,0,0) 8%,rgba(0,0,0,.3) 95%);}" +
    ".wxtip-spin i{position:absolute;top:0;left:50%;margin-left:-2px;width:4px;height:4px;border-radius:0 4px 4px 0;background:currentColor;}" +
    "@keyframes wxtipspin{from{transform:rotate(0deg);}to{transform:rotate(360deg);}}" +
    ".wxmodal-mask{position:fixed;left:0;top:0;right:0;bottom:0;background:rgba(0,0,0,.6);z-index:5000;display:flex;align-items:center;justify-content:center;}" +
    ".wxmodal{width:320px;max-width:calc(100% - 32px);max-height:90%;background:#fff;border-radius:12px;overflow:hidden;text-align:center;display:flex;flex-direction:column;}" +
    ".wxmodal-hd{padding:32px 24px 16px;font-size:17px;font-weight:700;line-height:1.4;color:rgba(0,0,0,.9);word-break:break-all;}" +
    ".wxmodal-hd:last-child{padding-bottom:32px;}" +
    ".wxmodal-bd{padding:0 24px;margin-bottom:32px;font-size:17px;line-height:1.4;color:rgba(0,0,0,.5);white-space:pre-wrap;word-break:break-all;overflow-y:auto;}" +
    ".wxmodal-bd:first-child{min-height:40px;padding:32px 24px 0;font-weight:700;color:rgba(0,0,0,.9);display:flex;flex-direction:column;justify-content:center;}" +
    ".wxmodal-ft{display:flex;min-height:56px;box-shadow:inset 0 .5px 0 rgba(0,0,0,.1);flex-shrink:0;}" +
    ".wxmodal-btn{flex:1;border:0;background:none;padding:0;margin:0;font:inherit;font-size:17px;font-weight:700;line-height:56px;cursor:pointer;}" +
    ".wxmodal-btn+.wxmodal-btn{box-shadow:inset .5px 0 0 rgba(0,0,0,.1);}" +
    ".wxmodal-btn:active{background:#ececec;}";
  var fbStyled = false;
  function fbStyle() {
    if (fbStyled) return;
    fbStyled = true;
    var s = document.createElement("style");
    s.textContent = FB_CSS;
    document.head.appendChild(s);
  }
  function fbDone(o, res, ok) {
    if (ok) { o.success && o.success(res); } else { o.fail && o.fail(res); }
    o.complete && o.complete(res);
    if (!o.success && !o.fail && !o.complete) return ok ? Promise.resolve(res) : Promise.reject(res);
  }
  function fbTitleError(api, t) {
    if (typeof t === "string") return null;
    var ty = t === null ? "Null" : t === undefined ? "Undefined" : Object.prototype.toString.call(t).slice(8, -1);
    return { errMsg: api + ":fail parameter error: parameter.title should be String instead of " + ty + ";" };
  }
  var tipEls = null, tipKind = null, tipTimer = null;
  var TIP_PATHS = {
    success: "M8.657 18.435L3 12.778l1.414-1.414 4.95 4.95L20.678 5l1.414 1.414-12.02 12.021a1 1 0 01-1.415 0z",
    error: "M12 22C6.477 22 2 17.523 2 12S6.477 2 12 2s10 4.477 10 10-4.477 10-10 10zm-.763-15.864l.11 7.596h1.305l.11-7.596h-1.525zm.759 10.967c.512 0 .902-.383.902-.882 0-.5-.39-.882-.902-.882a.878.878 0 00-.896.882c0 .499.396.882.896.882z"
  };
  function tipClear() {
    if (tipTimer) { clearTimeout(tipTimer); tipTimer = null; }
    if (tipEls) tipEls.forEach(function (e) { e.remove(); });
    tipEls = null; tipKind = null;
  }
  function tipShow(kind, title, icon, image, mask) {
    fbStyle();
    tipClear();
    var icHtml = "";
    if (image) icHtml = "<img class='wxtip-i' style='object-fit:contain' src='" + esc(image) + "'>";
    else if (icon === "loading") icHtml = "<span class='wxtip-i wxtip-spin'><i></i></span>";
    else if (TIP_PATHS[icon]) icHtml = "<svg class='wxtip-i' viewBox='0 0 24 24'><path fill='currentColor' d='" + TIP_PATHS[icon] + "'/></svg>";
    var wrp = document.createElement("div");
    wrp.className = "wxtip-wrp";
    wrp.innerHTML = "<div class='wxtip " + (icon === "none" && !image ? "wxtip_text" : "wxtip_icon") + "'>" + icHtml +
      (title ? "<div class='wxtip-t'>" + esc(title) + "</div>" : "") + "</div>";
    tipEls = [wrp];
    if (mask) {
      var m = document.createElement("div");
      m.className = "wxtip-mask";
      document.body.appendChild(m);
      tipEls.push(m);
    }
    document.body.appendChild(wrp);
    tipKind = kind;
  }
  function toast(o) {
    o = o || {};
    var bad = fbTitleError("showToast", o.title);
    if (bad) return fbDone(o, bad, false);
    var icon = o.icon;
    if (icon !== "success" && icon !== "error" && icon !== "loading" && icon !== "none") icon = "success";
    var dur = o.duration === undefined ? 1500 : Number(o.duration);
    if (!isFinite(dur) || dur < 0) dur = 1500;
    tipShow("toast", o.title, icon, typeof o.image === "string" ? o.image : "", !!o.mask);
    tipTimer = setTimeout(function () { tipTimer = null; if (tipKind === "toast") tipClear(); }, dur);
    return fbDone(o, { errMsg: "showToast:ok" }, true);
  }
  function showLoading(o) {
    o = o || {};
    var bad = fbTitleError("showLoading", o.title);
    if (bad) return fbDone(o, bad, false);
    tipShow("loading", o.title, "loading", "", !!o.mask);
    return fbDone(o, { errMsg: "showLoading:ok" }, true);
  }
  function hideTip(api, kind, o) {
    o = o || {};
    if (!tipKind || (o.noConflict && tipKind !== kind)) return fbDone(o, { errMsg: api + ":fail:toast can't be found" }, false);
    tipClear();
    return fbDone(o, { errMsg: api + ":ok" }, true);
  }
  function modalLen(s) { var n = 0; for (var i = 0; i < s.length; i++) n += s.charCodeAt(i) > 0xFF ? 2 : 1; return n; }
  function modalColor(v, d) { return typeof v === "string" && /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(v) ? v : d; }
  function showModal(o) {
    o = o || {};
    fbStyle();
    var title = o.title == null ? "" : String(o.title), content = o.content == null ? "" : String(o.content);
    var cancelText = o.cancelText == null ? "取消" : String(o.cancelText), confirmText = o.confirmText == null ? "确定" : String(o.confirmText);
    if (modalLen(cancelText) > 8) return fbDone(o, { errMsg: "showModal:fail cancelText length should not larger than 4 Chinese characters" }, false);
    if (modalLen(confirmText) > 8) return fbDone(o, { errMsg: "showModal:fail confirmText length should not larger than 4 Chinese characters" }, false);
    var resolveP = null, promise = null;
    if (!o.success && !o.fail && !o.complete) promise = new Promise(function (r) { resolveP = r; });
    var mask = document.createElement("div");
    mask.className = "wxmodal-mask";
    mask.innerHTML = "<div class='wxmodal'>" +
      (title ? "<div class='wxmodal-hd'>" + esc(title) + "</div>" : "") +
      (content ? "<div class='wxmodal-bd'>" + esc(content) + "</div>" : "") +
      "<div class='wxmodal-ft'>" +
      (o.showCancel !== false ? "<button class='wxmodal-btn wxmc' style='color:" + modalColor(o.cancelColor, "#000000") + "'>" + esc(cancelText) + "</button>" : "") +
      "<button class='wxmodal-btn wxmo' style='color:" + modalColor(o.confirmColor, "#576B95") + "'>" + esc(confirmText) + "</button></div></div>";
    document.body.appendChild(mask);
    function finish(confirm) {
      mask.remove();
      var r = { errMsg: "showModal:ok", confirm: confirm, cancel: !confirm };
      o.success && o.success(r); resolveP && resolveP(r); o.complete && o.complete(r);
    }
    var ok = mask.querySelector(".wxmo"), no = mask.querySelector(".wxmc");
    ok.addEventListener("click", function () { finish(true); });
    if (no) no.addEventListener("click", function () { finish(false); });
    return promise;
  }

  window.wx = {
    navigateTo: nav, redirectTo: nav, switchTab: nav, reLaunch: nav,
    navigateBack: function () { history.back(); },
    showToast: toast,
    showLoading: showLoading,
    hideToast: function (o) { return hideTip("hideToast", "toast", o); },
    hideLoading: function (o) { return hideTip("hideLoading", "loading", o); },
    showModal: showModal,
    createAnimation: createAnimation,
    startPullDownRefresh: function (o) { pullDown.start(); o && o.success && o.success(); o && o.complete && o.complete(); },
    stopPullDownRefresh: function (o) { pullDown.stop(); o && o.success && o.success(); o && o.complete && o.complete(); },
    setStorageSync: function (k, v) { try { localStorage.setItem(k, JSON.stringify(v)); } catch (e) {} },
    getStorageSync: function (k) { try { var v = localStorage.getItem(k); return v ? JSON.parse(v) : ""; } catch (e) { return ""; } },
    removeStorageSync: function (k) { try { localStorage.removeItem(k); } catch (e) {} },
    setStorage: function (o) { this.setStorageSync(o.key, o.data); if (o.success) o.success(); },
    getStorage: function (o) { var v = this.getStorageSync(o.key); if (o.success) o.success({ data: v }); },
    request: function (o) { fetch(o.url, { method: o.method || "GET", headers: o.header, body: o.data && o.method && o.method !== "GET" ? JSON.stringify(o.data) : undefined }).then(function (r) { return r.json().catch(function () { return {}; }); }).then(function (d) { if (o.success) o.success({ data: d, statusCode: 200 }); }).catch(function (e) { if (o.fail) o.fail(e); }).then(function () { if (o.complete) o.complete(); }); },
    getSystemInfoSync: function () { return { windowWidth: 375, windowHeight: 667, pixelRatio: 2, platform: "devtools" }; },
    createCanvasContext: function (id) { var el = document.querySelector('[data-canvas-id="' + id + '"]') || document.getElementById(id); if (!el || !el.getContext) return noopCanvasCtx(); if (!el.__sized) { el.width = el.clientWidth || 320; el.height = el.clientHeight || 200; el.__sized = true; } return wrapCanvasCtx(el.getContext("2d")); }
    // createSelectorQuery / createIntersectionObserver / nextTick 在 08c_query.js
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

