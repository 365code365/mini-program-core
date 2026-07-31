  // ───────── picker 底部选择面板 ─────────
  function pkPad2(n) { return (n < 10 ? "0" : "") + n; }
  function pkDaysInMonth(y, m) { var d = [31, ((y % 4 === 0 && y % 100 !== 0) || y % 400 === 0) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]; return d[(m - 1 + 12) % 12]; }
  function pkColLabels(arr, rangeKey) {
    return (arr || []).map(function (it) {
      return (rangeKey && it && typeof it === "object") ? ("" + it[rangeKey]) : ("" + it);
    });
  }
  // 依 mode 组装 { cols:[[label..]..], sel:[idx..], toValue:fn(sel)->detailValue }
  function pkBuild(mode, range, valAttr, rangeKey, fields, startS, endS) {
    function parseIdxList(s) { try { var a = JSON.parse(s); return a.map(function (x) { return x | 0; }); } catch (e) { return []; } }
    if (mode === "multiSelector") {
      var cols = (range || []).map(function (c) { return pkColLabels(c, rangeKey); });
      var picked = parseIdxList(valAttr);
      var sel = cols.map(function (c, i) { return Math.min(Math.max(picked[i] || 0, 0), Math.max(c.length - 1, 0)); });
      return { cols: cols, sel: sel, toValue: function (s) { return s.slice(); } };
    }
    if (mode === "time") {
      var hrs = [], mins = [], i;
      for (i = 0; i < 24; i++) hrs.push(pkPad2(i) + "时");
      for (i = 0; i < 60; i++) mins.push(pkPad2(i) + "分");
      var hm = /^(\d{1,2}):(\d{1,2})$/.exec(valAttr || "");
      var h = hm ? +hm[1] : new Date().getHours(), mi = hm ? +hm[2] : new Date().getMinutes();
      return { cols: [hrs, mins], sel: [h, mi], toValue: function (s) { return pkPad2(s[0]) + ":" + pkPad2(s[1]); } };
    }
    if (mode === "date") {
      var now = new Date();
      var dm = /^(\d{4})-?(\d{1,2})?-?(\d{1,2})?$/.exec(valAttr || "");
      var y = dm ? +dm[1] : now.getFullYear(), mo = dm && dm[2] ? +dm[2] : now.getMonth() + 1, da = dm && dm[3] ? +dm[3] : now.getDate();
      var sy = /^(\d{4})/.exec(startS || ""); var ey = /^(\d{4})/.exec(endS || "");
      var y0 = sy ? +sy[1] : Math.min(y - 60, now.getFullYear() - 60);
      var y1 = ey ? +ey[1] : Math.max(y + 60, now.getFullYear() + 60); if (y1 < y0) y1 = y0;
      var years = []; for (var yy = y0; yy <= y1; yy++) years.push(yy + "年");
      var cols = [years], sel = [Math.min(Math.max(y - y0, 0), years.length - 1)];
      var months = []; for (var mm = 1; mm <= 12; mm++) months.push(mm + "月");
      var days = []; for (var dd = 1; dd <= pkDaysInMonth(y, mo); dd++) days.push(dd + "日");
      if (fields !== "year") { cols.push(months); sel.push(Math.min(Math.max(mo - 1, 0), 11)); }
      if (fields !== "year" && fields !== "month") { cols.push(days); sel.push(Math.min(Math.max(da - 1, 0), days.length - 1)); }
      return {
        cols: cols, sel: sel, dateFields: fields || "day",
        toValue: function (s) {
          var yr = y0 + s[0];
          if (fields === "year") return "" + yr;
          if (fields === "month") return yr + "-" + pkPad2(s[1] + 1);
          return yr + "-" + pkPad2(s[1] + 1) + "-" + pkPad2(s[2] + 1);
        }
      };
    }
    if (mode === "region") {
      // H5 无内置行政区划表：range 传了二维数组就用它，否则退化为空（原生端有内置表）
      var rcols = (range && range.length && Array.isArray(range[0])) ? range.map(function (c) { return pkColLabels(c, rangeKey); }) : [];
      var rsel = rcols.map(function () { return 0; });
      return { cols: rcols, sel: rsel, toValue: function (s) { return s.map(function (i, ci) { return rcols[ci][i]; }); } };
    }
    // selector
    var col = pkColLabels(range, rangeKey);
    var idx = Math.min(Math.max((valAttr | 0) || 0, 0), Math.max(col.length - 1, 0));
    return { cols: [col], sel: [idx], toValue: function (s) { return s[0]; } };
  }

  var ITEM_H = 44;
  function showPicker(el) {
    var mode = el.getAttribute("data-picker-mode") || "selector";
    var range; try { range = JSON.parse(el.getAttribute("data-picker-range") || "[]"); } catch (e) { range = []; }
    var st = pkBuild(mode, range,
      el.getAttribute("data-picker-value"), el.getAttribute("data-picker-rangekey"),
      el.getAttribute("data-picker-fields"), el.getAttribute("data-picker-start"), el.getAttribute("data-picker-end"));
    if (!st.cols.length || !st.cols[0].length) return;

    var mask = document.createElement("div"); mask.className = "wxpk-mask";
    var sheet = document.createElement("div"); sheet.className = "wxpk-sheet";
    var head = document.createElement("div"); head.className = "wxpk-head";
    var cancel = document.createElement("div"); cancel.className = "wxpk-cancel"; cancel.textContent = "取消";
    var confirm = document.createElement("div"); confirm.className = "wxpk-confirm"; confirm.textContent = "确定";
    head.appendChild(cancel); head.appendChild(confirm);
    var colsWrap = document.createElement("div"); colsWrap.className = "wxpk-cols";
    var colEls = st.cols.map(function (items, ci) {
      var c = document.createElement("div"); c.className = "wxpk-col";
      var top = document.createElement("div"); top.className = "wxpk-col-pad"; c.appendChild(top);
      items.forEach(function (label) { var it = document.createElement("div"); it.className = "wxpk-item"; it.textContent = label; c.appendChild(it); });
      var bot = document.createElement("div"); bot.className = "wxpk-col-pad"; c.appendChild(bot);
      colsWrap.appendChild(c);
      return c;
    });
    sheet.appendChild(head); sheet.appendChild(colsWrap);
    document.body.appendChild(mask); document.body.appendChild(sheet);
    // 初始定位到当前选中项
    colEls.forEach(function (c, i) { c.scrollTop = (st.sel[i] || 0) * ITEM_H; });
    // 入场动画（下一帧加 show 类触发过渡）
    requestAnimationFrame(function () { mask.classList.add("show"); sheet.classList.add("show"); });

    function nearestIdx(c, len) { return Math.min(Math.max(Math.round(c.scrollTop / ITEM_H), 0), Math.max(len - 1, 0)); }
    function close() {
      mask.classList.remove("show"); sheet.classList.remove("show");
      setTimeout(function () { if (mask.parentNode) mask.parentNode.removeChild(mask); if (sheet.parentNode) sheet.parentNode.removeChild(sheet); }, 240);
    }
    mask.addEventListener("click", close);
    cancel.addEventListener("click", close);
    confirm.addEventListener("click", function () {
      var sel = colEls.map(function (c, i) { return nearestIdx(c, st.cols[i].length); });
      var handler = el.getAttribute("data-change");
      if (handler) call(handler, evObj(el, { value: st.toValue(sel) }));
      close();
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
      var pk = e.target.closest("[data-picker-mode]");
      if (pk) { if (!pk.getAttribute("data-picker-disabled")) showPicker(pk); return; }
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

