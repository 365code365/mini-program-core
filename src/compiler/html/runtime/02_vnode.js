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
  // 页面根（模板作用域栈为空）进来时先立一个根作用域；其余递归调用沿用栈顶。
  function renderNodesV(nodes, scope) {
    if (tplFrames.length) return renderListV(nodes, scope);
    tplBudget = TPL_MAX_EXPANSIONS;
    return inFrame({ t: tplTableOf(nodes), r: scope, d: 0, w: wxsOf(nodes) }, function () { return renderListV(nodes, scope); });
  }
  // 进一个作用域帧：模板表与 wxs 模块一起切换，出来时恢复
  function inFrame(fr, body) {
    var savedW = curWxs;
    tplFrames.push(fr);
    curWxs = fr.w;
    try { return body(); } finally { tplFrames.pop(); curWxs = savedW; }
  }
  function renderListV(nodes, scope) {
    var out = [], branch = false;
    for (var i = 0; i < nodes.length; i++) {
      var node = nodes[i];
      // 文本节点：与原生渲染器一致地去掉首尾空白（WXML 缩进换行不应产生可见空行；
      // 数据里的 \n 由 .wx-text{white-space:pre-line} 保留为换行）
      if (node.t === "tx") { out.push({ t: "tx", text: interp(node.x, scope).replace(/^[\s\u3000]+|[\s\u3000]+$/g, "") }); branch = false; continue; }
      // 模板定义 / import / wxs：编译期的东西，不出节点，也不打断 if/else 链
      if (isDeclV(node)) continue;
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
    if (tag === "block") {
      var kids = renderListV(node.c || [], scope);
      for (var i = 0; i < kids.length; i++) out.push(kids[i]);
      return;
    }
    // 有 name 的是定义，不渲染；有 is 的查表展开
    if (tag === "template") { if (!("name" in (node.a || {}))) renderTemplateV(out, node, scope); return; }
    if (tag === "include") { renderIncludeV(out, node); return; }
    if (tag === "import" || tag === "wxs") return;
    out.push(vnodeEl(node, scope));
  }

  // ───────── 模板片段 / include ─────────
  // 与原生 parser/template/{scope,fragment}.rs 同一套规则。编译期（parser::wxml_file）已经把
  // include 的文件内容放进 include 节点的子节点、把被 import 文件的模板挂在 import 节点下、
  // 把 <template name> 上提到各自文件顶层 —— 这里只按「作用域根」建表、查表展开。
  //  · 模板只看得见 data 传入的对象（官方 wxml/template.html）；
  //  · include 的内容只认数据根（页面 data 或所在模板的 data），不认 wx:for 的 item/index
  //    （glass-easel 语义，Skyline 只支持 glass-easel）；
  //  · import 不传递：只收 import 节点的直接 <template name> 子节点。
  var TPL_MAX_DEPTH = 16, TPL_MAX_EXPANSIONS = 50000;
  var tplFrames = [], tplBudget = 0, tplWarned = {};
  var tplCache = typeof WeakMap === "function" ? new WeakMap() : null;
  function isDeclV(n) {
    if (n.t !== "el") return false;
    return (n.n === "template" && n.a && ("name" in n.a)) || n.n === "import" || n.n === "wxs";
  }
  function tplName(n) {
    return (n && n.t === "el" && n.n === "template" && n.a && n.a.name != null) ? String(n.a.name).trim() : null;
  }
  function tplTableOf(list) {
    var hit = tplCache && tplCache.get(list);
    if (hit) return hit;
    var t = Object.create(null), i, j, k, n, kids;
    for (i = 0; i < list.length; i++) {
      n = list[i];
      if (n.t !== "el" || n.n !== "import") continue;
      kids = n.c || [];
      // 后写的 import 覆盖先写的
      for (j = 0; j < kids.length; j++) { k = tplName(kids[j]); if (k != null) t[k] = { b: kids[j].c || [], f: kids }; }
    }
    // 本文件的定义覆盖 import 进来的
    for (i = 0; i < list.length; i++) { k = tplName(list[i]); if (k != null) t[k] = { b: list[i].c || [], f: list }; }
    if (tplCache) tplCache.set(list, t);
    return t;
  }
  function tplWarn(msg) { if (!tplWarned[msg]) { tplWarned[msg] = 1; if (window.console) console.warn(msg); } }
  // data="{{ … }}" 的花括号里是对象字面量的内容：{{...item, a: 1}}、{{a, b}}、{{a: x}}
  function tplData(raw, scope) {
    var m = raw == null ? null : /^\s*\{\{([\s\S]*)\}\}\s*$/.exec("" + raw);
    var body = m ? m[1].trim() : "";
    if (!body) return {};
    var v = evalExpr("{" + body + "}", scope);
    if (v == null || typeof v !== "object") v = evalExpr(body, scope);
    return (v != null && typeof v === "object") ? v : {};
  }
  function renderTemplateV(out, node, scope) {
    var a = node.a || {}, fr = tplFrames[tplFrames.length - 1];
    var name = String(interp(a.is, scope) || "").trim();
    if (!name || !fr) return;
    var def = fr.t[name];
    if (!def) { tplWarn('<template is="' + name + '">：当前文件里没有这个模板，什么都不渲染'); return; }
    if (fr.d >= TPL_MAX_DEPTH || tplBudget <= 0) { tplWarn('<template is="' + name + '">：嵌套过深或展开过多，已截断'); return; }
    tplBudget--;
    var data = tplData(a.data, scope);
    // 模板里只能用定义模板的文件里的 wxs 模块
    var kids = inFrame({ t: tplTableOf(def.f), r: data, d: fr.d + 1, w: wxsOf(def.f) }, function () { return renderListV(def.b, data); });
    for (var i = 0; i < kids.length; i++) out.push(kids[i]);
  }
  function renderIncludeV(out, node) {
    var kids = node.c || [], fr = tplFrames[tplFrames.length - 1];
    if (!kids.length || !fr) return;
    // 被引入文件自己的模板优先，其次仍能用引入方的
    var own = tplTableOf(kids), t = fr.t, k;
    if (Object.keys(own).length) {
      t = Object.create(null);
      for (k in fr.t) t[k] = fr.t[k];
      for (k in own) t[k] = own[k];
    }
    // 被引入文件的内容用它自己文件里的 wxs 模块（include 不传递模块）
    var v = inFrame({ t: t, r: fr.r, d: fr.d, w: wxsOf(kids) }, function () { return renderListV(kids, fr.r); });
    for (var i = 0; i < v.length; i++) out.push(v[i]);
  }

  // ───────── wxs 模块 ─────────
  // 与原生 parser::template::wxs_scope + js/prelude/wxs.js 同一套规则：模块只在定义它的文件里可见
  // （不进 include / import 的子节点）；src 与 require 的文件编译期已打包进 <wxs> 的子文本
  // （带 __wxs_bundle 标记），内联且不 require 别人的现包一个。模块按包内容做单例。
  var wxsCache = typeof WeakMap === "function" ? new WeakMap() : null, wxsMods = {};
  function wxsOf(list) {
    var hit = wxsCache && wxsCache.get(list);
    if (hit !== undefined) return hit;
    var m = null;
    (function walk(ns) {
      for (var i = 0; i < ns.length; i++) {
        var n = ns[i];
        if (!n || n.t !== "el") continue;
        if (n.n === "wxs") {
          var name = n.a && n.a.module ? String(n.a.module).trim() : "";
          if (!name || (m && name in m)) continue;
          var text = (n.c || []).map(function (c) { return c.t === "tx" ? c.x : ""; }).join("");
          var bundle;
          if (n.a && ("__wxs_bundle" in n.a)) { try { bundle = JSON.parse(text); } catch (e) { bundle = null; } }
          else { var f = {}; f["inline:" + name] = text; bundle = { entry: "inline:" + name, files: f }; }
          (m || (m = {}))[name] = wxsLoad(bundle, text);
        } else if (n.n !== "include" && n.n !== "import") walk(n.c || []);
      }
    })(list);
    if (wxsCache) wxsCache.set(list, m);
    return m;
  }
  function wxsLoad(bundle, key) {
    if (wxsMods.hasOwnProperty(key)) return wxsMods[key];
    var files = (bundle && bundle.files) || {}, cache = {};
    function resolve(from, rel) {
      var base = String(from).split("#")[0].split("/"); base.pop();
      String(rel).split("/").forEach(function (p) { if (p === "" || p === ".") return; if (p === "..") { if (base.length > 1) base.pop(); return; } base.push(p); });
      var out = base.join("/"); return out.charAt(0) === "/" ? out : "/" + out;
    }
    function getDate() { var a = arguments; if (!a.length) return new Date(); if (a.length === 1) return new Date(a[0]); return new Date(a[0], a[1], a.length > 2 ? a[2] : 1, a[3] || 0, a[4] || 0, a[5] || 0, a[6] || 0); }
    function getRegExp(p, f) { return new RegExp(p, f || ""); }
    function load(path) {
      if (cache.hasOwnProperty(path)) return cache[path].exports;
      var code = files[path];
      if (code === undefined) throw new Error("找不到 wxs 模块 " + path);
      var mod = { exports: {} }; cache[path] = mod;
      new Function("module", "exports", "require", "getDate", "getRegExp", code)(mod, mod.exports, function (r) { return load(resolve(path, r)); }, getDate, getRegExp);
      return mod.exports;
    }
    var exp;
    try { exp = bundle ? load(bundle.entry) : {}; } catch (e) { tplWarn("[wxs] " + (bundle && bundle.entry) + ": " + e.message); exp = {}; }
    return (wxsMods[key] = exp == null ? {} : exp);
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
    else if (tag === "scroll-view") {
      // 受控位置与阈值交给运行时（08f_scroll.js）：浏览器的 overflow:auto 自己滚，事件与受控值另接
      ["scroll-top", "scroll-left", "scroll-into-view", "upper-threshold", "lower-threshold", "scroll-with-animation"].forEach(function (k) {
        if (a[k] != null) attrs["data-sv-" + k] = "" + interp(a[k], scope);
      });
    }
    else if (tag === "video") {
      // 包内绝对路径同图片一样要换成相对路径（file:// 打开导出工程时 `/assets/x.mp4` 指到磁盘根上）
      if (a["src"]) attrs["src"] = assetUrl(interp(a["src"], scope));
      if (a["poster"]) attrs["poster"] = assetUrl(interp(a["poster"], scope));
      // 布尔属性与原生端同一套：写了且不是 false 就算开
      var vflag = function (k, dflt) { if (a[k] == null) return dflt; var s = String(interp(a[k], scope)).trim(); return s !== "false" && s !== "0"; };
      if (vflag("controls", true)) attrs["controls"] = "controls";
      if (vflag("autoplay", false)) attrs["autoplay"] = "autoplay";
      if (vflag("loop", false)) attrs["loop"] = "loop";
      if (vflag("muted", false)) attrs["muted"] = "muted";
      var of = a["object-fit"] ? interp(a["object-fit"], scope) : "";
      if (of === "fill" || of === "cover") { var vs = attrs["style"] ? attrs["style"].replace(/;?\s*$/, ";") : ""; attrs["style"] = vs + "object-fit:" + of; }
      attrs["playsinline"] = "playsinline";
    }
    else if (tag === "audio") { if (a["src"]) attrs["src"] = assetUrl(interp(a["src"], scope)); attrs["controls"] = "controls"; }
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
    if (a["name"] && /^(input|textarea|checkbox|radio|switch|slider|picker|checkbox-group|radio-group)$/.test(tag)) attrs["name"] = interp(a["name"], scope);
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

