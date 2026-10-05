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
    applyHover(tag, a, scope, attrs);

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
    else if (tag === "picker") {
      // picker 的「点开弹面板」是宿主行为，这里把配置塞进 data 属性，运行时读它弹出选择器
      attrs["data-picker-mode"] = interp(a["mode"] || "selector", scope) || "selector";
      var rng = evalWhole(a["range"], scope);
      attrs["data-picker-range"] = (rng != null && typeof rng === "object") ? JSON.stringify(rng) : "[]";
      if (a["value"] != null) { var pv = evalWhole(a["value"], scope); attrs["data-picker-value"] = (pv != null && typeof pv === "object") ? JSON.stringify(pv) : ("" + (pv == null ? "" : pv)); }
      if (a["range-key"]) attrs["data-picker-rangekey"] = interp(a["range-key"], scope);
      if (a["fields"]) attrs["data-picker-fields"] = interp(a["fields"], scope);
      if (a["start"]) attrs["data-picker-start"] = interp(a["start"], scope);
      if (a["end"]) attrs["data-picker-end"] = interp(a["end"], scope);
      if (truthy(evalWhole(a["disabled"], scope))) attrs["data-picker-disabled"] = "1";
    }
    if (a["name"] && /^(input|textarea|checkbox|radio|switch|slider|picker)$/.test(tag)) attrs["name"] = interp(a["name"], scope);
  }

  // 微信点击态：button 默认 button-hover（20ms 后出现，松手再留 70ms）；
  // 普通元素默认没有，写了 hover-class 才有（50ms / 400ms）。"none" 关掉。
  function applyHover(tag, a, scope, attrs) {
    if (truthy(evalWhole(a["disabled"], scope))) return;
    var raw = a["hover-class"];
    var hc = raw != null ? String(interp(raw, scope) || "").trim() : (tag === "button" ? "button-hover" : "");
    if (!hc || hc === "none") return;
    attrs["data-hover-class"] = hc;
    var start = a["hover-start-time"] != null ? String(interp(a["hover-start-time"], scope) || "").trim() : "";
    var stay = a["hover-stay-time"] != null ? String(interp(a["hover-stay-time"], scope) || "").trim() : "";
    attrs["data-hover-start"] = start || (tag === "button" ? "20" : "50");
    attrs["data-hover-stay"] = stay || (tag === "button" ? "70" : "400");
    if (truthy(evalWhole(a["hover-stop-propagation"], scope))) attrs["data-hover-stop"] = "1";
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
    // animation：wx.createAnimation 的 export 载荷，载荷变化时由 patchAttrs 触发重播
    if (a["animation"]) {
      var av = evalWhole(a["animation"], scope);
      if (av && typeof av === "object") attrs["data-animation"] = JSON.stringify(av);
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
  // 内置图标：直接查编译期生成的 __WXICON 表（数据源同原生渲染器，见 icon_data.rs）。
  // 以前这里手抄了一份和编译期并行的 switch，改一处漏一处。
  function iconSvg(t) {
    if (typeof __WXICON !== "undefined" && __WXICON[t]) return __WXICON[t];
    return "";
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

