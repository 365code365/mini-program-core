  // ───────── wx.createAnimation 的播放：把 actions 逐步映射成 CSS transition ─────────
  // 与原生端同一份载荷、同一套语义（每步绝对目标值、按 delay/duration 顺序播）。
  function playJsAnimation(el, payload) {
    var data;
    try { data = typeof payload === "string" ? JSON.parse(payload.replace(/'/g, '"')) : payload; } catch (e) { return; }
    if (!data || !data.actions || !data.actions.length) return;
    if (el.__animTimers) el.__animTimers.forEach(clearTimeout);
    el.__animTimers = [];
    // 累积状态：未提及的属性沿用上一步
    var st = { tx: 0, ty: 0, rotate: 0, sx: 1, sy: 1, skx: 0, sky: 0, opacity: null, bg: null };
    var at = 0;
    data.actions.forEach(function (action) {
      var tr = (action.option && action.option.transition) || {};
      var dur = tr.duration === undefined ? 400 : tr.duration;
      var delay = tr.delay || 0;
      var tf = tr.timingFunction || "linear";
      (action.animates || []).forEach(function (a) {
        var v = a.args || [];
        switch (a.type) {
          case "translate": st.tx = v[0] || 0; st.ty = v[1] || 0; break;
          case "translateX": st.tx = v[0] || 0; break;
          case "translateY": st.ty = v[0] || 0; break;
          case "rotate": st.rotate = v[0] || 0; break;
          case "scale": st.sx = v[0] === undefined ? 1 : v[0]; st.sy = v[1] === undefined ? st.sx : v[1]; break;
          case "scaleX": st.sx = v[0] === undefined ? 1 : v[0]; break;
          case "scaleY": st.sy = v[0] === undefined ? 1 : v[0]; break;
          case "skew": st.skx = v[0] || 0; st.sky = v[1] || 0; break;
          case "opacity": st.opacity = v[0]; break;
          case "backgroundColor": st.bg = v[0]; break;
        }
      });
      var snapshot = JSON.parse(JSON.stringify(st));
      el.__animTimers.push(setTimeout(function () {
        el.style.transition = "transform " + dur + "ms " + tf + ", opacity " + dur + "ms " + tf + ", background-color " + dur + "ms " + tf;
        el.style.transform = "translate(" + snapshot.tx + "px," + snapshot.ty + "px) rotate(" + snapshot.rotate
          + "deg) scale(" + snapshot.sx + "," + snapshot.sy + ") skew(" + snapshot.skx + "deg," + snapshot.sky + "deg)";
        if (snapshot.opacity !== null) el.style.opacity = snapshot.opacity;
        if (snapshot.bg !== null) el.style.backgroundColor = snapshot.bg;
      }, at + delay));
      at += delay + dur;
    });
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

