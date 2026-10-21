  // ───────── scroll-view 的事件与受控位置（与原生端 host::scroll_view 同语义）─────────
  // 浏览器的 overflow:auto 自己滚；这里补上 bindscroll（detail: scrollLeft / scrollTop / scrollHeight /
  // scrollWidth / deltaX / deltaY）、bindscrolltoupper / bindscrolltolower（进入阈值时各发一次）、
  // 以及 scroll-top / scroll-left / scroll-into-view 的受控值（值**变了**才动，与原生端一致；
  // scroll-with-animation 时平滑滚动）。
  var svState = typeof WeakMap === "function" ? new WeakMap() : null;
  function svGet(el) { var s = svState && svState.get(el); if (!s) { s = { top: el.scrollTop, left: el.scrollLeft, upper: true, lower: false }; if (svState) svState.set(el, s); } return s; }
  function isSV(el) { return el && el.classList && el.classList.contains("wx-scroll-view"); }
  function horizontal(el) { return /overflow-x:\s*auto/.test(el.getAttribute("style") || ""); }
  document.addEventListener("scroll", function (e) {
    var el = e.target; if (!isSV(el)) return;
    var p = svGet(el), h = horizontal(el);
    var d = { scrollLeft: el.scrollLeft, scrollTop: el.scrollTop, scrollHeight: el.scrollHeight, scrollWidth: el.scrollWidth,
              deltaX: el.scrollLeft - p.left, deltaY: el.scrollTop - p.top };
    var hs = el.getAttribute("data-scroll"); if (hs) call(hs, evObj(el, d));
    var ut = Number(el.getAttribute("data-sv-upper-threshold") || 50), lt = Number(el.getAttribute("data-sv-lower-threshold") || 50);
    var pos = h ? el.scrollLeft : el.scrollTop, max = h ? el.scrollWidth - el.clientWidth : el.scrollHeight - el.clientHeight;
    var upper = pos <= ut, lower = max - pos <= lt;
    if (upper && !p.upper) { var hu = el.getAttribute("data-scrolltoupper"); if (hu) call(hu, evObj(el, { direction: h ? "left" : "top" })); }
    if (lower && !p.lower) { var hl = el.getAttribute("data-scrolltolower"); if (hl) call(hl, evObj(el, { direction: h ? "right" : "bottom" })); }
    p.top = el.scrollTop; p.left = el.scrollLeft; p.upper = upper; p.lower = lower;
  }, true);
  function syncScrollViews() {
    var list = document.querySelectorAll(".wx-scroll-view");
    for (var i = 0; i < list.length; i++) {
      var el = list[i], p = svGet(el);
      var smooth = el.getAttribute("data-sv-scroll-with-animation");
      var behavior = smooth != null && smooth !== "false" && smooth !== "0" ? "smooth" : "auto";
      var into = el.getAttribute("data-sv-scroll-into-view") || "";
      var top = el.getAttribute("data-sv-scroll-top"), left = el.getAttribute("data-sv-scroll-left");
      if (into && into !== p.into) {
        var t = document.getElementById(into);
        if (t && el.contains(t)) {
          var o = t.getBoundingClientRect(), b = el.getBoundingClientRect();
          el.scrollTo({ top: el.scrollTop + o.top - b.top, left: el.scrollLeft + o.left - b.left, behavior: behavior });
        }
      } else if (top != null && top !== p.topAttr) {
        el.scrollTo({ top: parseFloat(top) || 0, behavior: behavior });
      } else if (left != null && left !== p.leftAttr) {
        el.scrollTo({ left: parseFloat(left) || 0, behavior: behavior });
      }
      p.into = into; p.topAttr = top; p.leftAttr = left;
    }
  }
