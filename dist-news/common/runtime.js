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
      // 文本节点：与原生渲染器一致地去掉首尾空白（WXML 缩进换行不应产生可见空行；
      // 数据里的 \n 由 .wx-text{white-space:pre-line} 保留为换行）
      if (node.t === "tx") { out.push({ t: "tx", text: interp(node.x, scope).replace(/^[\s\u3000]+|[\s\u3000]+$/g, "") }); branch = false; continue; }
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
    if (tag === "switch" && truthy(evalWhole(a["disabled"], scope))) cls += " wx-switch-disabled";
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
      // size 决定图标直径，color 作为 CSS color 供内联 SVG 的 currentColor 使用
      if (a["size"]) { var sz = parseFloat(interp(a["size"], scope)) || 23; if (style && !/;\s*$/.test(style)) style += ";"; style += "width:" + sz + "px;height:" + sz + "px"; }
      if (a["color"]) { if (style && !/;\s*$/.test(style)) style += ";"; style += "color:" + interp(a["color"], scope); }
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

  // 绝对资源路径（以 / 开头）→ 相对当前页面的路径（导出工程用 file:// 打开也能加载）
  var PAGE_UP = (function () { var seg = (window.__PAGE_ROUTE__ || "").split("/"); var d = seg.length - 1; return d > 0 ? new Array(d + 1).join("../") : ""; })();
  function assetUrl(src) { return (src && src.charAt(0) === "/") ? PAGE_UP + src.slice(1) : src; }

  function applyElAttrs(tag, a, scope, attrs, v) {
    if (tag === "image") { if (a["src"]) attrs["src"] = assetUrl(interp(a["src"], scope)); }
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
    // JS 接管后标记 live，恢复多屏横向滚动（无 JS 静态首屏仅显示第一屏）
    var swCls = "wx-swiper wx-swiper-live"; if (a["class"]) swCls += " " + interp(a["class"], scope);
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
    if (tag === "icon") return iconSvg(interp(a["type"] || "success", scope));
    if (tag === "slider") return sliderInner(a, scope);
    return null;
  }
  // slider：range input（已滑过部分着色）+ 可选数值标签，与编译期产物一致
  function sliderInner(a, scope) {
    var num = function (k, d) { var v = parseFloat(interp(a[k], scope)); return isNaN(v) ? d : v; };
    var min = num("min", 0), max = num("max", 100), val = num("value", min);
    var step = interp(a["step"], scope) || "1";
    var span = (max - min) || 1;
    var pct = Math.max(0, Math.min(100, ((val - min) / span) * 100));
    var active = interp(a["activeColor"] || a["active-color"] || "#09bb07", scope);
    var bg = interp(a["backgroundColor"] || a["background-color"] || "#e5e5e5", scope);
    var dis = truthy(evalWhole(a["disabled"], scope)) ? " disabled" : "";
    var h = '<input class="wx-slider" type="range" min="' + min + '" max="' + max + '" step="' + step +
      '" value="' + val + '"' + dis + ' style="background:linear-gradient(to right,' + active + ' 0%,' +
      active + ' ' + pct + '%,' + bg + ' ' + pct + '%,' + bg + ' 100%)">';
    if (truthy(evalWhole(a["show-value"], scope)) || truthy(evalWhole(a["show-info"], scope))) {
      h += '<span class="wx-slider-value">' + (Math.round(val) === val ? val : val.toFixed(1)) + "</span>";
    }
    return h;
  }
  // 拖动时即时更新已滑过轨道与数值（避免等 setData 回流）
  function refreshSlider(el) {
    if (!el || !el.classList || !el.classList.contains("wx-slider")) return;
    var min = parseFloat(el.min || "0"), max = parseFloat(el.max || "100"), val = parseFloat(el.value || "0");
    var span = (max - min) || 1;
    var pct = Math.max(0, Math.min(100, ((val - min) / span) * 100));
    var bgm = /linear-gradient\(to right,([^ ]+) 0%,[^,]+,([^ ]+) [\d.]+%/.exec(el.style.background || "");
    var active = bgm ? bgm[1] : "#09bb07", bg = bgm ? bgm[2] : "#e5e5e5";
    el.style.background = "linear-gradient(to right," + active + " 0%," + active + " " + pct + "%," + bg + " " + pct + "%," + bg + " 100%)";
    var label = el.parentNode && el.parentNode.querySelector(".wx-slider-value");
    if (label) label.textContent = (Math.round(val) === val ? val : val.toFixed(1));
  }
  // 与编译期 icon_svg 保持一致的矢量图标（圆底 currentColor + 白色标记）
  function iconSvg(t) {
    var circle = '<circle cx="12" cy="12" r="12" fill="currentColor"/>';
    var check = '<path d="M5.8 12.4 10 16.4 18.2 7.6" fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>';
    var body;
    switch (t) {
      case "success_no_circle": body = '<path d="M3.5 12.5 9 18 20.5 5.5" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>'; break;
      case "back": case "arrow_left": case "arrow-left": body = '<path d="M15.5 4 7.5 12 15.5 20" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>'; break;
      case "arrow": case "arrow_right": case "arrow-right": body = '<path d="M8.5 4 16.5 12 8.5 20" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>'; break;
      case "arrow_up": case "arrow-up": body = '<path d="M4 15.5 12 7.5 20 15.5" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>'; break;
      case "arrow_down": case "arrow-down": body = '<path d="M4 8.5 12 16.5 20 8.5" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>'; break;
      case "plus": body = '<path d="M12 4.5V19.5M4.5 12H19.5" fill="none" stroke="currentColor" stroke-width="2.8" stroke-linecap="round"/>'; break;
      case "minus": body = '<path d="M4.5 12H19.5" fill="none" stroke="currentColor" stroke-width="2.8" stroke-linecap="round"/>'; break;
      case "info": case "info_circle": body = circle + '<circle cx="12" cy="7.6" r="1.7" fill="#fff"/><rect x="10.9" y="10.8" width="2.2" height="7" rx="1.1" fill="#fff"/>'; break;
      case "warn": body = circle + '<rect x="10.9" y="5.4" width="2.2" height="7.2" rx="1.1" fill="#fff"/><circle cx="12" cy="16.6" r="1.7" fill="#fff"/>'; break;
      case "waiting": case "waiting_circle": body = circle + '<rect x="10.9" y="6.2" width="2.2" height="6.6" rx="1.1" fill="#fff"/><rect x="12" y="10.9" width="5.2" height="2.2" rx="1.1" fill="#fff"/><circle cx="12" cy="12" r="1.6" fill="#fff"/>'; break;
      case "info_no_circle": body = '<circle cx="12" cy="5.6" r="2" fill="currentColor"/><rect x="10.4" y="9.8" width="3.2" height="9.4" rx="1.6" fill="currentColor"/>'; break;
      case "warn_no_circle": body = '<rect x="10.4" y="3" width="3.2" height="11.2" rx="1.6" fill="currentColor"/><circle cx="12" cy="18.8" r="2" fill="currentColor"/>'; break;
      case "waiting_no_circle": case "clock": body = '<circle cx="12" cy="12" r="10.4" fill="none" stroke="currentColor" stroke-width="2.2"/><path d="M12 5.6V12h5.2" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/>'; break;
      case "close": case "cancel_no_circle": body = '<path d="M5 5 19 19M19 5 5 19" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round"/>'; break;
      case "cancel": case "clear": body = circle + '<path d="M7.6 7.6 16.4 16.4M16.4 7.6 7.6 16.4" fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round"/>'; break;
      case "download": body = '<circle cx="12" cy="12" r="10.8" fill="none" stroke="currentColor" stroke-width="2.2"/><path d="M12 6v7" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/><path d="M8 12.4 12 16.6 16 12.4Z" fill="currentColor"/><rect x="7" y="17.4" width="10" height="2.2" rx="1.1" fill="currentColor"/>'; break;
      case "search": body = '<circle cx="10.4" cy="10.4" r="6.4" fill="none" stroke="currentColor" stroke-width="2.4"/><path d="M15.2 15.2 21 21" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"/>'; break;
      case "circle": body = '<circle cx="12" cy="12" r="10.8" fill="none" stroke="currentColor" stroke-width="2.2"/>'; break;
      case "star": body = '<path d="M12 1.6 15.2 8.6 22.8 9.5 17.2 14.6 18.7 22 12 18.3 5.3 22 6.8 14.6 1.2 9.5 8.8 8.6Z" fill="currentColor"/>'; break;
      case "star-o": case "star_o": body = '<path d="M12 1.6 15.2 8.6 22.8 9.5 17.2 14.6 18.7 22 12 18.3 5.3 22 6.8 14.6 1.2 9.5 8.8 8.6Z" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"/>'; break;
      case "heart": body = '<path d="M12 21C6 16.5 2.6 13.4 2.6 9.6 2.6 6.5 5 4.2 8 4.2c1.8 0 3.2.9 4 2.2.8-1.3 2.2-2.2 4-2.2 3 0 5.4 2.3 5.4 5.4 0 3.8-3.4 6.9-9.4 11.4Z" fill="currentColor"/>'; break;
      default: body = circle + check;
    }
    return '<svg viewBox="0 0 24 24" aria-hidden="true">' + body + "</svg>";
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
      if (el.classList && el.classList.contains("wx-slider")) {
        refreshSlider(el);
        var slWrap = el.closest ? el.closest(".wx-slider-wrap") : null;
        var sc = (slWrap && (slWrap.getAttribute("data-change") || slWrap.getAttribute("data-changing")))
          || el.getAttribute("data-change") || el.getAttribute("data-changing");
        if (sc) call(sc, evObj(slWrap || el, { value: Number(val) }));
      }
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
    hideToast: hideToastNow, hideLoading: hideToastNow, stopPullDownRefresh: function () {},
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
