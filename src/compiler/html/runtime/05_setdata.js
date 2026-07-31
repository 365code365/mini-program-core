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

