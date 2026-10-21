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

  // 表单字段的值，类型与原生端（host/form.rs）一致：name 写在 checkbox-group / radio-group 上，
  // 前者给选中项 value 的数组、后者给选中项的 value（没选是空串）；switch 布尔、slider 数字、
  // picker 与它 bindchange 的 detail.value 同型。单个 checkbox / radio 上的 name 不算字段（微信语义）。
  function pickerFormValue(el) {
    var mode = el.getAttribute("data-picker-mode") || "selector", raw = el.getAttribute("data-picker-value") || "";
    if (mode === "multiSelector" || mode === "region") { try { var a = JSON.parse(raw); return Array.isArray(a) ? a : []; } catch (e) { return []; } }
    if (mode === "time" || mode === "date") return raw;
    return parseInt(raw, 10) || 0;
  }
  function collectForm(form) {
    var v = {};
    form.querySelectorAll("[name]").forEach(function (el) {
      var n = el.getAttribute("name"); if (!n) return;
      var cl = el.classList || { contains: function () { return false; } };
      if (cl.contains("wx-checkbox-group")) { var arr = []; el.querySelectorAll(".wx-checkbox:checked").forEach(function (c) { arr.push(c.value); }); v[n] = arr; }
      else if (cl.contains("wx-radio-group")) { var r = el.querySelector(".wx-radio:checked"); v[n] = r ? r.value : ""; }
      else if (el.type === "checkbox" || el.type === "radio") return;
      else if (cl.contains("wx-switch")) v[n] = cl.contains("wx-switch-on");
      else if (el.hasAttribute("data-picker-mode")) v[n] = pickerFormValue(el);
      else if (cl.contains("wx-slider")) v[n] = Number(el.value);
      else if ("value" in el) v[n] = el.value;
    });
    return v;
  }
  // 重置：输入框清空、勾选与开关取消、滑块回到 min（与原生端一致）；picker 不留状态、不动
  function resetForm(form) {
    form.querySelectorAll("input,textarea").forEach(function (el) {
      if (el.type === "checkbox" || el.type === "radio") el.checked = false;
      else if (el.classList && el.classList.contains("wx-slider")) { el.value = el.min || "0"; if (typeof refreshSlider === "function") refreshSlider(el); }
      else el.value = "";
    });
    form.querySelectorAll(".wx-switch").forEach(function (sw) { sw.classList.remove("wx-switch-on"); });
  }

