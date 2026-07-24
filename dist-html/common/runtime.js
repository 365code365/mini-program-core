/* mini-render 响应式运行时（导出 HTML 工程） */
(function () {
  "use strict";
  var appConfig = null, appInst = null, pageConfig = null, pageInst = null;
  var AST = window.__WXML__ || [];
  var composing = false, delegated = false;

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

  // ───────── 渲染为 VNode 树 ─────────
  // VNode: { t:'el', tag, attrs:{}, children:[], value?, checked?, innerHTML? } | { t:'tx', text }
  function renderNodesV(nodes, scope) {
    var out = [], branch = false;
    for (var i = 0; i < nodes.length; i++) {
      var node = nodes[i];
      if (node.t === "tx") { out.push({ t: "tx", text: interp(node.x, scope) }); branch = false; continue; }
      var a = node.a || {};
      if ("wx:for" in a) { renderForV(node, scope, out); branch = false; continue; }
      if ("wx:if" in a) { var c = !!evalWhole(a["wx:if"], scope); branch = c; if (c) pushEl(out, node, scope); continue; }
      if ("wx:elif" in a) { if (!branch) { var c2 = !!evalWhole(a["wx:elif"], scope); if (c2) { branch = true; pushEl(out, node, scope); } } continue; }
      if ("wx:else" in a) { if (!branch) pushEl(out, node, scope); branch = false; continue; }
      pushEl(out, node, scope); branch = false;
    }
    return out;
  }
  function pushEl(out, node, scope) {
    var tag = node.n;
    if (tag === "block" || tag === "template" || tag === "import" || tag === "include") {
      var kids = renderNodesV(node.c || [], scope);
      for (var i = 0; i < kids.length; i++) out.push(kids[i]);
      return;
    }
    out.push(vnodeEl(node, scope));
  }
  function renderForV(node, scope, out) {
    var a = node.a;
    var list = evalWhole(a["wx:for"], scope);
    var itemName = a["wx:for-item"] || "item", idxName = a["wx:for-index"] || "index";
    var arr = [];
    if (Array.isArray(list)) arr = list;
    else if (list && typeof list === "object") arr = Object.keys(list).map(function (k) { return list[k]; });
    else if (typeof list === "number") { for (var z = 0; z < list; z++) arr.push(z); }
    var inner = { t: "el", n: node.n, a: {}, c: node.c };
    for (var k in a) { if (k !== "wx:for" && k !== "wx:for-item" && k !== "wx:for-index" && k !== "wx:key") inner.a[k] = a[k]; }
    for (var i = 0; i < arr.length; i++) {
      var cs = Object.create(scope || {}); cs[itemName] = arr[i]; cs[idxName] = i;
      pushEl(out, inner, cs);
    }
  }

  function vnodeEl(node, scope) {
    var tag = node.n, a = node.a || {};
    if (tag === "swiper") return swiperVNode(node, a, scope);
    var htmlTag = mapTag(tag), isVoid = !!VOID[tag];
    var attrs = {};
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
    attrs["class"] = cls;
    if (a["id"]) attrs["id"] = interp(a["id"], scope);
    var style = a["style"] ? convertRpx(interp(a["style"], scope)) : "";
    var ex = extraStyle(tag, a, scope);
    if (ex) { if (style && !/;\s*$/.test(style)) style += ";"; style += ex; }
    if (tag === "icon") {
      if (a["size"]) { var sz = parseFloat(interp(a["size"], scope)) || 23; if (style && !/;\s*$/.test(style)) style += ";"; style += "font-size:" + Math.round(sz / 1.2) + "px"; }
      if (a["color"]) { if (style && !/;\s*$/.test(style)) style += ";"; style += "background:" + interp(a["color"], scope); }
    }
    if (style) attrs["style"] = style;

    var v = { t: "el", tag: htmlTag, attrs: attrs };
    applyElAttrs(tag, a, scope, attrs, v);
    applyEvents(a, scope, attrs);

    if (isVoid) return v;
    var inner = customInner(tag, a, scope);
    if (inner != null) { v.innerHTML = inner; return v; }
    if (tag === "textarea") { v.value = interp(a["value"] != null ? a["value"] : (a["model:value"] || ""), scope); v.children = []; return v; }
    v.children = renderNodesV(node.c || [], scope);
    return v;
  }

  function applyElAttrs(tag, a, scope, attrs, v) {
    if (tag === "image") { if (a["src"]) attrs["src"] = interp(a["src"], scope); }
    else if (tag === "input") {
      var t = interp(a["type"] || "text", scope);
      attrs["type"] = truthy(evalWhole(a["password"], scope)) ? "password" : ((t === "number" || t === "digit" || t === "idcard") ? "number" : t);
      var iv = a["value"] != null ? interp(a["value"], scope) : (a["model:value"] != null ? interp(a["model:value"], scope) : null);
      if (iv != null) v.value = iv;
      if (a["placeholder"]) attrs["placeholder"] = interp(a["placeholder"], scope);
    }
    else if (tag === "checkbox" || tag === "radio") {
      attrs["type"] = tag === "checkbox" ? "checkbox" : "radio";
      if (a["value"] != null) attrs["value"] = interp(a["value"], scope);
      v.checked = truthy(evalWhole(a["checked"], scope));
      if (truthy(evalWhole(a["disabled"], scope))) attrs["disabled"] = "disabled";
    }
    else if (tag === "slider") {
      attrs["type"] = "range";
      attrs["min"] = interp(a["min"], scope) || "0";
      attrs["max"] = interp(a["max"], scope) || "100";
      attrs["step"] = interp(a["step"], scope) || "1";
      if (a["value"] != null) v.value = interp(a["value"], scope);
    }
    else if (tag === "navigator") { if (a["url"]) attrs["href"] = "#" + interp(a["url"], scope); }
    else if (tag === "video") { if (a["src"]) attrs["src"] = interp(a["src"], scope); if (a["poster"]) attrs["poster"] = interp(a["poster"], scope); attrs["controls"] = "controls"; attrs["playsinline"] = "playsinline"; }
    else if (tag === "audio") { if (a["src"]) attrs["src"] = interp(a["src"], scope); attrs["controls"] = "controls"; }
    else if (tag === "canvas") { if (a["canvas-id"]) attrs["data-canvas-id"] = interp(a["canvas-id"], scope); }
    else if (tag === "button" && a["form-type"]) { attrs["data-form-type"] = interp(a["form-type"], scope); }
    if (a["name"] && /^(input|textarea|checkbox|radio|switch|slider|picker)$/.test(tag)) attrs["name"] = interp(a["name"], scope);
  }

  function applyEvents(a, scope, attrs) {
    for (var k in a) {
      var ev = k.indexOf("bind") === 0 ? k.slice(4) : (k.indexOf("catch") === 0 ? k.slice(5) : null);
      if (ev) { if (ev.charAt(0) === ":") ev = ev.slice(1); if (ev) attrs["data-" + ev] = interp(a[k], scope); }
    }
    for (var k2 in a) {
      if (k2.indexOf("data-") === 0) {
        var val = evalWhole(a[k2], scope);
        attrs["data-ds-" + k2.slice(5)] = (val != null && typeof val === "object") ? JSON.stringify(val) : ("" + (val == null ? "" : val));
      } else if (k2.indexOf("model:") === 0) {
        var m = /^\s*\{\{([\s\S]+)\}\}\s*$/.exec(a[k2] || "");
        attrs["data-model"] = m ? m[1].trim() : a[k2];
      }
    }
  }

  function swiperVNode(node, a, scope) {
    var swCls = "wx-swiper"; if (a["class"]) swCls += " " + interp(a["class"], scope);
    var swAttrs = { "class": swCls };
    swAttrs["data-autoplay"] = "" + truthy(evalWhole(a["autoplay"], scope));
    swAttrs["data-interval"] = interp(a["interval"], scope) || "5000";
    if (truthy(evalWhole(a["vertical"], scope))) swAttrs["data-vertical"] = "true";
    var style = a["style"] ? convertRpx(interp(a["style"], scope)) : ""; if (style) swAttrs["style"] = style;
    var items = renderNodesV(node.c || [], scope);
    var wrapCh = [{ t: "el", tag: "div", attrs: swAttrs, children: items }];
    if (truthy(evalWhole(a["indicator-dots"], scope))) {
      var dots = [];
      for (var i = 0; i < items.length; i++) dots.push({ t: "el", tag: "i", attrs: { "class": "wx-swiper-dot" }, children: [] });
      wrapCh.push({ t: "el", tag: "div", attrs: { "class": "wx-swiper-dots" }, children: dots });
    }
    return { t: "el", tag: "div", attrs: { "class": "wx-swiper-wrap" }, children: wrapCh };
  }

  function extraStyle(tag, a, scope) {
    if (tag === "scroll-view") return truthy(evalWhole(a["scroll-x"], scope)) ? "display:flex;flex-direction:row;flex-wrap:nowrap;overflow-x:auto;overflow-y:hidden" : "overflow-y:auto";
    if (tag === "image") { var mode = interp(a["mode"] || "scaleToFill", scope); return mode === "aspectFit" ? "object-fit:contain" : mode === "aspectFill" ? "object-fit:cover" : mode === "widthFix" ? "height:auto" : "object-fit:fill"; }
    return "";
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

  // ───────── VNode → 真实 DOM（创建 & diff/patch，增量更新，保留焦点/输入状态）─────────
  function createEl(v) {
    if (v.t === "tx") return document.createTextNode(v.text);
    var el = document.createElement(v.tag);
    var a = v.attrs || {};
    for (var k in a) el.setAttribute(k, a[k]);
    if (v.value != null) el.value = v.value;
    if (v.checked != null) el.checked = v.checked;
    if (v.innerHTML != null) el.innerHTML = v.innerHTML;
    else if (v.children) for (var i = 0; i < v.children.length; i++) el.appendChild(createEl(v.children[i]));
    return el;
  }
  function patchAttrs(dom, oldA, newA) {
    for (var k in oldA) { if (!(k in newA)) dom.removeAttribute(k); }
    for (var k2 in newA) { if (oldA[k2] !== newA[k2]) dom.setAttribute(k2, newA[k2]); }
  }
  function patch(parent, dom, oldV, newV) {
    if (oldV == null) { parent.appendChild(createEl(newV)); return; }
    if (newV == null) { if (dom && dom.parentNode === parent) parent.removeChild(dom); return; }
    if (oldV.t !== newV.t || (newV.t === "el" && oldV.tag !== newV.tag)) { parent.replaceChild(createEl(newV), dom); return; }
    if (newV.t === "tx") { if (oldV.text !== newV.text) dom.textContent = newV.text; return; }
    patchAttrs(dom, oldV.attrs || {}, newV.attrs || {});
    // value/checked 作为“属性属性”：仅当数据驱动值变化时才写回 DOM，且避免覆盖用户正在输入的值
    // —— 这保证输入不被清空、光标不跳动、复选/滑块的用户交互不被回滚。
    if (newV.value != null && oldV.value !== newV.value && dom.value !== newV.value) dom.value = newV.value;
    if (newV.checked != null && oldV.checked !== newV.checked) dom.checked = newV.checked;
    if (newV.innerHTML != null) { if (oldV.innerHTML !== newV.innerHTML) dom.innerHTML = newV.innerHTML; return; }
    patchChildren(dom, oldV.children || [], newV.children || []);
  }
  function patchChildren(parent, oldCh, newCh) {
    var common = Math.min(oldCh.length, newCh.length);
    for (var i = 0; i < common; i++) patch(parent, parent.childNodes[i], oldCh[i], newCh[i]);
    for (var r = oldCh.length - 1; r >= newCh.length; r--) { var d = parent.childNodes[r]; if (d) parent.removeChild(d); }
    for (var k = common; k < newCh.length; k++) parent.appendChild(createEl(newCh[k]));
  }

  // ───────── setData / 挂载 / 更新 ─────────
  var curVTree = null;
  function pageData() { return pageInst ? pageInst.data : {}; }
  function setByPath(obj, path, val) {
    var tokens = path.replace(/\[(\w+)\]/g, ".$1").split(".");
    var o = obj;
    for (var i = 0; i < tokens.length - 1; i++) { var t = tokens[i]; if (o[t] == null) o[t] = {}; o = o[t]; }
    o[tokens[tokens.length - 1]] = val;
  }
  function mountPage() {
    var app = document.getElementById("app");
    if (!app || !AST || !AST.length) return false;
    try {
      var vtree = renderNodesV(AST, pageData());
      app.innerHTML = "";
      for (var i = 0; i < vtree.length; i++) app.appendChild(createEl(vtree[i]));
      curVTree = vtree;
      initWidgets();
      return true;
    } catch (e) { console.error("render error", e); return false; }
  }
  function updatePage() {
    var app = document.getElementById("app");
    if (!app || curVTree == null) { mountPage(); return; }
    try {
      var vtree = renderNodesV(AST, pageData());
      patchChildren(app, curVTree, vtree);
      curVTree = vtree;
      initWidgets();
    } catch (e) { console.error("patch error", e); }
  }
  function setData(patch, cb) {
    if (patch) for (var k in patch) setByPath(pageInst.data, k, patch[k]);
    updatePage();
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
      if (model) { if (composing) { setByPath(pageInst.data, model, val); } else { var p = {}; p[model] = val; setData(p); } }
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
    document.addEventListener("compositionend", function (e) { composing = false; var el = e.target; if (el.getAttribute && el.getAttribute("data-model")) { var p = {}; p[el.getAttribute("data-model")] = el.value; setData(p); } });
  }

  // ───────── 组件初始化（幂等，不重构 DOM，兼容 vdom 增量更新）─────────
  // swiper 的 wrap/dots 已由 vnode 生成；这里只“一次性”为每个 swiper 绑定滚动/自动播放，
  // 并直接操作 dot 的 active 类（不写入 vnode.attrs，故 patch 不会覆盖）。
  function initWidgets() {
    document.querySelectorAll(".wx-radio-group").forEach(function (g, gi) {
      var name = "wxradio_" + gi;
      g.querySelectorAll(".wx-radio").forEach(function (r) { r.name = name; });
    });
    document.querySelectorAll(".wx-swiper").forEach(function (sw) {
      if (sw.__init) return; sw.__init = true;
      var vertical = sw.getAttribute("data-vertical") === "true";
      var dots = sw.parentNode ? sw.parentNode.querySelector(".wx-swiper-dots") : null;
      function upd() {
        var i = Math.round(vertical ? sw.scrollTop / (sw.clientHeight || 1) : sw.scrollLeft / (sw.clientWidth || 1));
        if (dots) for (var j = 0; j < dots.children.length; j++) dots.children[j].className = "wx-swiper-dot" + (j === i ? " active" : "");
      }
      sw.addEventListener("scroll", upd); upd();
      if (sw.getAttribute("data-autoplay") === "true") {
        var iv = parseInt(sw.getAttribute("data-interval"), 10) || 5000;
        setInterval(function () {
          var n = sw.querySelectorAll(".wx-swiper-item").length; if (!n) return;
          var cur = Math.round(vertical ? sw.scrollTop / (sw.clientHeight || 1) : sw.scrollLeft / (sw.clientWidth || 1));
          var nx = (cur + 1) % n, pos = nx * (vertical ? sw.clientHeight : sw.clientWidth);
          sw.scrollTo(vertical ? { top: pos, behavior: "smooth" } : { left: pos, behavior: "smooth" });
        }, iv);
      }
    });
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
      pageInst.setData = function (patch, cb) { setData(patch, cb); };
      pageInst.selectComponent = function () { return null; };
      var q = parseQuery();
      // 先按初始 data 挂载 DOM，再跑生命周期——这样 onLoad 里 wx.createCanvasContext
      // 等依赖真实节点的调用能拿到已存在的元素（canvas 绘制才生效）。
      if (!mountPage()) initWidgets();
      try { if (pageInst.onLoad) pageInst.onLoad(q); } catch (e) { console.error(e); }
      try { if (pageInst.onShow) pageInst.onShow(); } catch (e) { console.error(e); }
      try { if (pageInst.onReady) pageInst.onReady(); } catch (e) { console.error(e); }
    } else {
      initWidgets();
    }
  });
})();
