  // ───────── hover-class：按住时加类，松手/移出时去掉（小程序官方按压反馈） ─────────
  function bindHoverClass() {
    var active = null;
    function on(e) {
      var el = e.target.closest && e.target.closest("[data-hover-class]");
      if (!el) return;
      off();
      var cls = el.getAttribute("data-hover-class");
      if (!cls) return;
      el.classList.add.apply(el.classList, cls.split(/\s+/));
      active = { el: el, cls: cls };
    }
    function off() {
      if (!active) return;
      active.el.classList.remove.apply(active.el.classList, active.cls.split(/\s+/));
      active = null;
    }
    document.addEventListener("touchstart", on, { passive: true });
    document.addEventListener("mousedown", on);
    ["touchend", "touchcancel", "mouseup", "mouseleave"].forEach(function (n) {
      document.addEventListener(n, off, { passive: true });
    });
  }

  var pullDown = (function () {
    var HEIGHT = 64;             // 指示器区域高度，同时是触发阈值（与原生端一致）
    var enabled = false, app = null, indicator = null;
    var pulling = false, refreshing = false, startY = 0, offset = 0;

    function ensureDom() {
      if (indicator || !app) return;
      indicator = document.createElement("div");
      indicator.className = "wx-pull-indicator";
      indicator.innerHTML = "<i></i><i></i><i></i>";
      app.parentNode.insertBefore(indicator, app);
    }
    function setOffset(v) {
      offset = v;
      if (!app) return;
      app.style.transform = v > 0 ? "translateY(" + v + "px)" : "";
      if (indicator) {
        // 点的不透明度跟随下拉进度；刷新中交给 CSS 动画
        var p = Math.min(1, v / HEIGHT);
        indicator.style.opacity = refreshing ? 1 : p;
        indicator.classList.toggle("is-refreshing", refreshing);
      }
    }
    function atTop() {
      var el = document.scrollingElement || document.documentElement;
      return (el.scrollTop || 0) <= 0 && (window.scrollY || 0) <= 0;
    }
    return {
      init: function () {
        enabled = !!window.__PAGE_PULL_DOWN__;
        app = document.getElementById("app");
        if (!enabled || !app) return;
        ensureDom();
        // 触摸下拉
        app.addEventListener("touchstart", function (e) {
          if (refreshing || !atTop()) return;
          pulling = true; startY = e.touches[0].clientY;
        }, { passive: true });
        app.addEventListener("touchmove", function (e) {
          if (!pulling) return;
          var d = e.touches[0].clientY - startY;
          if (d <= 0) { setOffset(0); return; }
          // 橡皮筋衰减，与原生端同一公式
          setOffset((1 - 1 / (d * 0.55 / window.innerHeight + 1)) * window.innerHeight);
        }, { passive: true });
        app.addEventListener("touchend", function () {
          if (!pulling) return;
          pulling = false;
          if (offset >= HEIGHT) { pullDown.start(); } else { setOffset(0); }
        });
        // 桌面端用滚轮模拟：在顶部继续向上滚就是下拉
        window.addEventListener("wheel", function (e) {
          if (refreshing || e.deltaY >= 0 || !atTop()) return;
          setOffset(Math.min(HEIGHT * 1.5, offset - e.deltaY * 0.5));
          clearTimeout(pullDown._t);
          pullDown._t = setTimeout(function () {
            if (offset >= HEIGHT) { pullDown.start(); } else { setOffset(0); }
          }, 120);
        }, { passive: true });
      },
      start: function () {
        if (!enabled || refreshing) return;
        refreshing = true;
        setOffset(HEIGHT);
        try { if (pageInst && pageInst.onPullDownRefresh) pageInst.onPullDownRefresh(); } catch (err) { console.error(err); }
      },
      stop: function () {
        if (!refreshing) { setOffset(0); return; }
        refreshing = false;
        setOffset(0);
      }
    };
  })();

  // ───────── 启动 ─────────
  document.addEventListener("DOMContentLoaded", function () {
    if (appConfig) { appInst = appConfig; appInst.globalData = appConfig.globalData || {}; try { if (appInst.onLaunch) appInst.onLaunch({}); } catch (e) { console.error(e); } try { if (appInst.onShow) appInst.onShow({}); } catch (e) { console.error(e); } }
    bindDelegation();
    bindHoverClass();
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
      pullDown.init();
    } else {
      initWidgets();
    }
  });
})();
