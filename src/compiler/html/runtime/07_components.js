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

