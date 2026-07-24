//! 浏览器端**响应式**运行时（导出 HTML 工程用）
//!
//! 与静态首屏不同，运行时内置一个 WXML 解释器：读取页面内嵌的 `window.__WXML__` AST，
//! 结合页面 `data` 渲染真实 DOM；`setData` 会按模板**重新渲染**，从而让所有事件、
//! 列表更新、条件渲染、`model:` 双向数据绑定都能真正工作。
//!
//! 关键点：
//! - `{{表达式}}` 直接用 `new Function + with(scope)` 求值（WXML 表达式本就是 JS 语法）
//! - `wx:for` / `wx:if` / `wx:elif` / `wx:else` / `block` 支持
//! - 事件委托（bind/catch → data-*），`model:value` → `data-model` 双向绑定
//! - `wx.*` 常用 API、旧版 canvas 2D 适配

/// 运行时 JS 源码
pub const RUNTIME_JS: &str = r####"/* mini-render 响应式运行时（导出 HTML 工程） */
(function () {
  "use strict";
  var appConfig = null, appInst = null, pageConfig = null, pageInst = null;
  var AST = window.__WXML__ || [];
  var composing = false, delegated = false, swiperTimers = [];

  // ───────── 框架 API ─────────
  window.App = function (cfg) { appConfig = cfg; };
  window.Page = function (cfg) { pageConfig = cfg; };
  window.Component = function (cfg) {
    var c = {};
    if (cfg.data) c.data = cfg.data;
    var m = cfg.methods || {};
    for (var k in m) c[k] = m[k];
    if (cfg.properties) { c.data = c.data || {}; for (var p in cfg.properties) { var pd = cfg.properties[p]; c.data[p] = (pd && typeof pd === "object" && "value" in pd) ? pd.value : undefined; } }
    if (cfg.attached) c.onLoad = cfg.attached;
    if (cfg.ready) c.onReady = cfg.ready;
    pageConfig = c;
  };
  window.getApp = function () { return appInst; };
  window.getCurrentPages = function () { return pageInst ? [pageInst] : []; };
  window.Behavior = function (b) { return b; };

  // ───────── 表达式 & 插值 ─────────
  var exprCache = {};
  function evalExpr(code, scope) {
    try {
      var fn = exprCache[code];
      if (!fn) { fn = new Function("$s", "with($s){return (" + code + ");}"); exprCache[code] = fn; }
      return fn(scope || {});
    } catch (e) { return undefined; }
  }
  function interp(str, scope) {
    if (str == null) return "";
    if (("" + str).indexOf("{{") < 0) return str;
    return ("" + str).replace(/\{\{([\s\S]+?)\}\}/g, function (_, e) { var v = evalExpr(e.trim(), scope); return v == null ? "" : ("" + v); });
  }
  function evalWhole(str, scope) {
    if (str == null) return undefined;
    var m = /^\s*\{\{([\s\S]+)\}\}\s*$/.exec("" + str);
    if (m) return evalExpr(m[1].trim(), scope);
    return str;
  }
  function truthy(v) { return v === true || v === "true" || v === "1" || v === 1; }
  function esc(s) { return ("" + s).replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;"); }
  function escText(s) { return ("" + s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;"); }
  function convertRpx(s) { return ("" + s).replace(/(-?[0-9.]+)rpx/g, function (_, n) { return (parseFloat(n) * 0.5) + "px"; }); }
  function camel(s) { return s.replace(/-([a-z])/g, function (_, c) { return c.toUpperCase(); }); }

  // ───────── 标签映射 ─────────
  var VOID = { image: 1, input: 1, checkbox: 1, radio: 1, slider: 1 };
  function mapTag(tag) {
    switch (tag) {
      case "text": return "span";
      case "image": return "img";
      case "button": return "button";
      case "input": case "checkbox": case "radio": case "slider": return "input";
      case "textarea": return "textarea";
      case "navigator": return "a";
      case "icon": return "i";
      case "video": return "video";
      case "audio": return "audio";
      case "canvas": return "canvas";
      default: return "div";
    }
  }

  // ───────── 渲染 ─────────
  function renderNodes(nodes, scope) {
    var html = "", branch = false;
    for (var i = 0; i < nodes.length; i++) {
      var node = nodes[i];
      if (node.t === "tx") { html += escText(interp(node.x, scope)); branch = false; continue; }
      var a = node.a || {};
      if ("wx:for" in a) { html += renderFor(node, scope); branch = false; continue; }
      if ("wx:if" in a) { var c = !!evalWhole(a["wx:if"], scope); branch = c; if (c) html += renderEl(node, scope); continue; }
      if ("wx:elif" in a) { if (!branch) { var c2 = !!evalWhole(a["wx:elif"], scope); if (c2) { branch = true; html += renderEl(node, scope); } } continue; }
      if ("wx:else" in a) { if (!branch) html += renderEl(node, scope); branch = false; continue; }
      html += renderEl(node, scope); branch = false;
    }
    return html;
  }

  function renderFor(node, scope) {
    var a = node.a;
    var list = evalWhole(a["wx:for"], scope);
    var itemName = a["wx:for-item"] || "item", idxName = a["wx:for-index"] || "index";
    var arr = [];
    if (Array.isArray(list)) arr = list;
    else if (list && typeof list === "object") arr = Object.keys(list).map(function (k) { return list[k]; });
    else if (typeof list === "number") { for (var z = 0; z < list; z++) arr.push(z); }
    var inner = { t: "el", n: node.n, a: {}, c: node.c };
    for (var k in a) { if (k !== "wx:for" && k !== "wx:for-item" && k !== "wx:for-index" && k !== "wx:key") inner.a[k] = a[k]; }
    var html = "";
    for (var i = 0; i < arr.length; i++) {
      var cs = Object.create(scope || {}); cs[itemName] = arr[i]; cs[idxName] = i;
      html += renderEl(inner, cs);
    }
    return html;
  }

  function renderEl(node, scope) {
    var tag = node.n, a = node.a || {};
    if (tag === "block" || tag === "template" || tag === "import" || tag === "include") return renderNodes(node.c || [], scope);
    var htmlTag = mapTag(tag), isVoid = !!VOID[tag];
    var cls = "wx-" + tag;
    if (a["class"]) cls += " " + interp(a["class"], scope);
    if (tag === "icon" && a["type"]) cls += " wxicon wxicon-" + interp(a["type"], scope);
    if (tag === "switch" && truthy(evalWhole(a["checked"], scope))) cls += " wx-switch-on";
    if (tag === "button") {
      var bt = a["type"] ? interp(a["type"], scope) : "default";
      cls += " wx-button-" + (bt === "primary" || bt === "warn" ? bt : "default");
      if (interp(a["size"], scope) === "mini") cls += " wx-button-mini";
      if (truthy(evalWhole(a["plain"], scope))) cls += " wx-button-plain";
      if (truthy(evalWhole(a["disabled"], scope))) cls += " wx-button-disabled";
    }
    var attrs = ' class="' + esc(cls) + '"';
    if (a["id"]) attrs += ' id="' + esc(interp(a["id"], scope)) + '"';

    var style = a["style"] ? convertRpx(interp(a["style"], scope)) : "";
    var ex = extraStyle(tag, a, scope);
    if (ex) { if (style && !/;\s*$/.test(style)) style += ";"; style += ex; }
    if (tag === "icon") {
      if (a["size"]) { var sz = parseFloat(interp(a["size"], scope)) || 23; if (style && !/;\s*$/.test(style)) style += ";"; style += "font-size:" + Math.round(sz / 1.2) + "px"; }
      if (a["color"]) { if (style && !/;\s*$/.test(style)) style += ";"; style += "background:" + interp(a["color"], scope); }
    }
    if (style) attrs += ' style="' + esc(style) + '"';
    attrs += elAttrs(tag, a, scope);
    attrs += eventAttrs(a, scope);

    if (isVoid) return "<" + htmlTag + attrs + ">";
    var inner = customInner(tag, a, scope);
    if (inner != null) return "<" + htmlTag + attrs + ">" + inner + "</" + htmlTag + ">";
    if (tag === "textarea") return "<textarea" + attrs + ">" + escText(interp(a["value"] || "", scope)) + "</textarea>";
    return "<" + htmlTag + attrs + ">" + renderNodes(node.c || [], scope) + "</" + htmlTag + ">";
  }

  function extraStyle(tag, a, scope) {
    if (tag === "scroll-view") return truthy(evalWhole(a["scroll-x"], scope)) ? "overflow-x:auto;overflow-y:hidden" : "overflow-y:auto";
    if (tag === "image") { var mode = interp(a["mode"] || "scaleToFill", scope); return mode === "aspectFit" ? "object-fit:contain" : mode === "aspectFill" ? "object-fit:cover" : mode === "widthFix" ? "height:auto" : "object-fit:fill"; }
    return "";
  }

  function elAttrs(tag, a, scope) {
    var s = "";
    if (tag === "image") { if (a["src"]) s += ' src="' + esc(interp(a["src"], scope)) + '"'; }
    else if (tag === "input") {
      var t = interp(a["type"] || "text", scope);
      var ty = truthy(evalWhole(a["password"], scope)) ? "password" : ((t === "number" || t === "digit" || t === "idcard") ? "number" : t);
      s += ' type="' + ty + '"';
      if (a["value"] != null) s += ' value="' + esc(interp(a["value"], scope)) + '"';
      if (a["placeholder"]) s += ' placeholder="' + esc(interp(a["placeholder"], scope)) + '"';
    }
    else if (tag === "checkbox" || tag === "radio") {
      s += ' type="' + (tag === "checkbox" ? "checkbox" : "radio") + '"';
      if (a["value"] != null) s += ' value="' + esc(interp(a["value"], scope)) + '"';
      if (truthy(evalWhole(a["checked"], scope))) s += " checked";
      if (truthy(evalWhole(a["disabled"], scope))) s += " disabled";
    }
    else if (tag === "slider") {
      s += ' type="range" min="' + (interp(a["min"], scope) || "0") + '" max="' + (interp(a["max"], scope) || "100") + '" step="' + (interp(a["step"], scope) || "1") + '"';
      if (a["value"] != null) s += ' value="' + esc(interp(a["value"], scope)) + '"';
    }
    else if (tag === "swiper") {
      s += ' data-autoplay="' + truthy(evalWhole(a["autoplay"], scope)) + '" data-interval="' + (interp(a["interval"], scope) || "5000") + '" data-circular="' + truthy(evalWhole(a["circular"], scope)) + '" data-dots="' + truthy(evalWhole(a["indicator-dots"], scope)) + '"';
      if (truthy(evalWhole(a["vertical"], scope))) s += ' data-vertical="true"';
    }
    else if (tag === "navigator") { if (a["url"]) s += ' href="#' + esc(interp(a["url"], scope)) + '"'; }
    else if (tag === "video") { if (a["src"]) s += ' src="' + esc(interp(a["src"], scope)) + '"'; if (a["poster"]) s += ' poster="' + esc(interp(a["poster"], scope)) + '"'; s += " controls playsinline"; }
    else if (tag === "audio") { if (a["src"]) s += ' src="' + esc(interp(a["src"], scope)) + '"'; s += " controls"; }
    else if (tag === "canvas") { if (a["canvas-id"]) s += ' data-canvas-id="' + esc(interp(a["canvas-id"], scope)) + '"'; }
    else if (tag === "button" && a["form-type"]) { s += ' data-form-type="' + esc(interp(a["form-type"], scope)) + '"'; }
    // 表单控件的 name（供 form 提交收集）
    if (a["name"] && /^(input|textarea|checkbox|radio|switch|slider|picker)$/.test(tag)) s += ' name="' + esc(interp(a["name"], scope)) + '"';
    return s;
  }

  function customInner(tag, a, scope) {
    if (tag === "switch") return '<span class="wx-switch-knob"></span>';
    if (tag === "progress") {
      var pct = Math.max(0, Math.min(100, parseFloat(interp(a["percent"], scope)) || 0));
      var color = interp(a["activeColor"] || a["active-color"] || a["color"] || "#09bb07", scope);
      var h = '<div class="wx-progress-outer"><div class="wx-progress-inner" style="width:' + pct + "%;background:" + color + '"></div></div>';
      if (truthy(evalWhole(a["show-info"], scope))) h += '<span class="wx-progress-info">' + Math.round(pct) + "%</span>";
      return h;
    }
    if (tag === "rich-text") return renderRich(evalWhole(a["nodes"], scope));
    return null;
  }
  function renderRich(v) {
    if (v == null) return "";
    if (typeof v === "string") return v;
    if (Array.isArray(v)) return v.map(renderRich).join("");
    if (typeof v === "object") {
      if (v.type === "text") return escText(v.text || "");
      var name = v.name || "div", at = "";
      if (v.attrs) for (var k in v.attrs) at += " " + k + '="' + esc(v.attrs[k]) + '"';
      return "<" + name + at + ">" + (v.children ? renderRich(v.children) : "") + "</" + name + ">";
    }
    return "";
  }

  function eventAttrs(a, scope) {
    var s = "";
    for (var k in a) {
      var ev = k.indexOf("bind") === 0 ? k.slice(4) : (k.indexOf("catch") === 0 ? k.slice(5) : null);
      if (ev) { if (ev.charAt(0) === ":") ev = ev.slice(1); if (ev) s += " data-" + ev + '="' + esc(interp(a[k], scope)) + '"'; }
    }
    for (var k2 in a) {
      if (k2.indexOf("data-") === 0) {
        var val = evalWhole(a[k2], scope);
        var enc = (val != null && typeof val === "object") ? JSON.stringify(val) : ("" + (val == null ? "" : val));
        s += " data-ds-" + k2.slice(5) + '="' + esc(enc) + '"';
      } else if (k2.indexOf("model:") === 0) {
        var m = /^\s*\{\{([\s\S]+)\}\}\s*$/.exec(a[k2] || "");
        s += ' data-model="' + esc(m ? m[1].trim() : a[k2]) + '"';
      }
    }
    return s;
  }

  // ───────── setData / 渲染 ─────────
  function setByPath(obj, path, val) {
    var tokens = path.replace(/\[(\w+)\]/g, ".$1").split(".");
    var o = obj;
    for (var i = 0; i < tokens.length - 1; i++) { var t = tokens[i]; if (o[t] == null) o[t] = {}; o = o[t]; }
    o[tokens[tokens.length - 1]] = val;
  }
  function renderPage() {
    var app = document.getElementById("app");
    if (!app || !AST || !AST.length) return false;
    try { app.innerHTML = renderNodes(AST, pageInst ? pageInst.data : {}); setupComponents(); return true; }
    catch (e) { console.error("render error", e); return false; }
  }
  function setDataAndRender(patch, cb) {
    if (patch) for (var k in patch) setByPath(pageInst.data, k, patch[k]);
    var active = document.activeElement;
    var model = active && active.getAttribute ? active.getAttribute("data-model") : null;
    var pos = null; try { pos = active ? active.selectionStart : null; } catch (e) {}
    renderPage();
    if (model) { var el = document.querySelector('[data-model="' + model.replace(/"/g, '\\"') + '"]'); if (el) { try { el.focus(); if (pos != null && el.setSelectionRange) el.setSelectionRange(pos, pos); } catch (e) {} } }
    if (cb) try { cb(); } catch (e) {}
  }

  // ───────── 事件委托 ─────────
  function datasetOf(el) {
    var ds = {};
    if (!el || !el.attributes) return ds;
    for (var i = 0; i < el.attributes.length; i++) {
      var at = el.attributes[i];
      if (at.name.indexOf("data-ds-") === 0) { var name = camel(at.name.slice(8)); try { ds[name] = JSON.parse(at.value); } catch (e) { ds[name] = at.value; } }
    }
    return ds;
  }
  function evObj(el, detail) {
    var ds = datasetOf(el);
    return { type: "tap", timeStamp: Date.now(), detail: detail || {}, currentTarget: { id: (el && el.id) || "", dataset: ds }, target: { id: (el && el.id) || "", dataset: ds } };
  }
  function call(fn, ev) { var h = pageInst && pageInst[fn]; if (typeof h === "function") { try { h.call(pageInst, ev); } catch (err) { console.error(fn, err); } } }

  function collectForm(form) {
    var v = {};
    form.querySelectorAll("[name]").forEach(function (el) {
      var n = el.getAttribute("name"); if (!n) return;
      if (el.type === "checkbox" || el.type === "radio") { if (el.checked) { if (el.type === "checkbox") { (v[n] = v[n] || []).push(el.value); } else v[n] = el.value; } }
      else if (el.classList && el.classList.contains("wx-switch")) v[n] = el.classList.contains("wx-switch-on");
      else if ("value" in el) v[n] = el.value;
    });
    return v;
  }
  function resetForm(form) {
    form.querySelectorAll("input,textarea").forEach(function (el) {
      if (el.type === "checkbox" || el.type === "radio") el.checked = false;
      else el.value = "";
    });
  }

  function bindDelegation() {
    if (delegated) return; delegated = true;
    document.addEventListener("click", function (e) {
      if (!e.target.closest) return;
      var sw = e.target.closest(".wx-switch");
      if (sw) { sw.classList.toggle("wx-switch-on"); var f = sw.getAttribute("data-change"); if (f) call(f, evObj(sw, { value: sw.classList.contains("wx-switch-on") })); }
      // form-type 按钮：提交 / 重置所在表单
      var fbtn = e.target.closest("[data-form-type]");
      if (fbtn) {
        var ft = fbtn.getAttribute("data-form-type"), form = fbtn.closest(".wx-form");
        if (form) {
          if (ft === "submit") { var sf = form.getAttribute("data-submit"); if (sf) call(sf, { detail: { value: collectForm(form) } }); }
          else if (ft === "reset") { resetForm(form); var rf = form.getAttribute("data-reset"); if (rf) call(rf, { detail: {} }); }
        }
      }
      var tapEl = e.target.closest("[data-tap]");
      if (tapEl) call(tapEl.getAttribute("data-tap"), evObj(tapEl, {}));
    });
    document.addEventListener("input", function (e) {
      var el = e.target; if (!el.getAttribute) return;
      var val = el.value;
      var model = el.getAttribute("data-model");
      if (model) { if (composing) { setByPath(pageInst.data, model, val); } else { var p = {}; p[model] = val; setDataAndRender(p); } }
      var inp = el.getAttribute("data-input"); if (inp) call(inp, evObj(el, { value: val }));
      if (el.classList && el.classList.contains("wx-slider")) { var sc = el.getAttribute("data-change") || el.getAttribute("data-changing"); if (sc) call(sc, evObj(el, { value: Number(val) })); }
    });
    document.addEventListener("change", function (e) {
      var el = e.target; if (!el.closest) return;
      var cg = el.closest(".wx-checkbox-group");
      if (cg) { var f = cg.getAttribute("data-change"); if (f) { var vals = []; cg.querySelectorAll(".wx-checkbox:checked").forEach(function (c) { vals.push(c.value); }); call(f, { detail: { value: vals } }); } }
      var rg = el.closest(".wx-radio-group");
      if (rg) { var f2 = rg.getAttribute("data-change"); if (f2) { var c = rg.querySelector(".wx-radio:checked"); call(f2, { detail: { value: c ? c.value : "" } }); } }
      if (el.getAttribute) { var cf = el.getAttribute("data-confirm"); if (cf) call(cf, evObj(el, { value: el.value })); var ch = el.getAttribute("data-change"); if (ch && !(el.classList && el.classList.contains("wx-slider"))) call(ch, evObj(el, { value: el.value })); }
    });
    document.addEventListener("compositionstart", function () { composing = true; });
    document.addEventListener("compositionend", function (e) { composing = false; var el = e.target; if (el.getAttribute && el.getAttribute("data-model")) { var p = {}; p[el.getAttribute("data-model")] = el.value; setDataAndRender(p); } });
  }

  // ───────── 组件行为（每次渲染后重建） ─────────
  function setupComponents() {
    swiperTimers.forEach(clearInterval); swiperTimers = [];
    document.querySelectorAll(".wx-swiper").forEach(function (sw) {
      var items = sw.querySelectorAll(".wx-swiper-item");
      if (!items.length) return;
      var vertical = sw.getAttribute("data-vertical") === "true";
      var wrap = document.createElement("div"); wrap.className = "wx-swiper-wrap";
      sw.parentNode.insertBefore(wrap, sw); wrap.appendChild(sw);
      var dots = null;
      if (sw.getAttribute("data-dots") === "true") {
        dots = document.createElement("div"); dots.className = "wx-swiper-dots";
        items.forEach(function (_, i) { var d = document.createElement("i"); d.className = "wx-swiper-dot" + (i === 0 ? " active" : ""); dots.appendChild(d); });
        wrap.appendChild(dots);
      }
      var idx = 0, n = items.length;
      function upd() { if (dots) for (var i = 0; i < dots.children.length; i++) dots.children[i].className = "wx-swiper-dot" + (i === idx ? " active" : ""); }
      function go(i) { idx = (i % n + n) % n; var pos = idx * (vertical ? sw.clientHeight : sw.clientWidth); sw.scrollTo(vertical ? { top: pos, behavior: "smooth" } : { left: pos, behavior: "smooth" }); upd(); }
      sw.addEventListener("scroll", function () { var i = Math.round(vertical ? sw.scrollTop / sw.clientHeight : sw.scrollLeft / sw.clientWidth); if (i !== idx) { idx = i; upd(); } });
      if (sw.getAttribute("data-autoplay") === "true") { var iv = parseInt(sw.getAttribute("data-interval"), 10) || 5000; swiperTimers.push(setInterval(function () { go(idx + 1); }, iv)); }
    });
    document.querySelectorAll(".wx-radio-group").forEach(function (g, gi) { var name = "wxradio_" + gi; g.querySelectorAll(".wx-radio").forEach(function (r) { r.name = name; }); });
  }

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
  function toast(o) { o = o || {}; var d = document.createElement("div"); d.textContent = o.title || ""; d.style.cssText = "position:fixed;left:50%;top:50%;transform:translate(-50%,-50%);z-index:9999;background:rgba(0,0,0,.75);color:#fff;padding:14px 20px;border-radius:10px;font-size:14px;max-width:70%;text-align:center;"; document.body.appendChild(d); setTimeout(function () { d.remove(); if (o.success) o.success(); if (o.complete) o.complete(); }, o.duration || 1500); }

  window.wx = {
    navigateTo: nav, redirectTo: nav, switchTab: nav, reLaunch: nav,
    navigateBack: function () { history.back(); },
    showToast: toast, showLoading: toast, hideToast: function () {}, hideLoading: function () {}, stopPullDownRefresh: function () {},
    showModal: function (o) { o = o || {}; var ok = window.confirm((o.title ? o.title + "\n\n" : "") + (o.content || "")); if (o.success) o.success({ confirm: ok, cancel: !ok }); },
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

  // ───────── 启动 ─────────
  document.addEventListener("DOMContentLoaded", function () {
    if (appConfig) { appInst = appConfig; appInst.globalData = appConfig.globalData || {}; try { if (appInst.onLaunch) appInst.onLaunch({}); } catch (e) { console.error(e); } try { if (appInst.onShow) appInst.onShow({}); } catch (e) { console.error(e); } }
    bindDelegation();
    if (pageConfig) {
      pageInst = pageConfig;
      pageInst.data = pageConfig.data || {};
      pageInst.route = window.__PAGE_ROUTE__;
      pageInst.setData = function (patch, cb) { setDataAndRender(patch, cb); };
      pageInst.selectComponent = function () { return null; };
      var q = parseQuery();
      try { if (pageInst.onLoad) pageInst.onLoad(q); } catch (e) { console.error(e); }
      try { if (pageInst.onShow) pageInst.onShow(); } catch (e) { console.error(e); }
      if (!renderPage()) setupComponents();
      try { if (pageInst.onReady) pageInst.onReady(); } catch (e) { console.error(e); }
    } else {
      setupComponents();
    }
  });
})();
"####;
