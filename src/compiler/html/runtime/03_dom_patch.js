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
    // animation="{{animData}}"（wx.createAnimation 的 export）：载荷变化就重播
    if (newA["data-animation"] && oldA["data-animation"] !== newA["data-animation"]) {
      playJsAnimation(dom, newA["data-animation"]);
    }
  }

