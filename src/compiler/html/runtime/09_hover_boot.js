  // ───────── hover-class：与微信同一套计时 ─────────
  // 按下等 data-hover-start 毫秒才加类；松手后再留 data-hover-stay 毫秒。
  // 手指移出元素立刻取消。hover-stop-propagation 挡住更外层的祖先。
  function bindHoverClass() {
    var pending = [];
    var holding = false;
    var fromTouch = false;

    function classesOf(el) {
      return (el.getAttribute("data-hover-class") || "").split(/\s+/).filter(Boolean);
    }
    function show(el) {
      if (el.__hoverOn) return;
      var cls = classesOf(el);
      if (!cls.length) return;
      el.classList.add.apply(el.classList, cls);
      el.__hoverOn = cls;
    }
    function hide(el) {
      if (!el || !el.__hoverOn) return;
      el.classList.remove.apply(el.classList, el.__hoverOn);
      el.__hoverOn = null;
    }
    function chain(target) {
      var list = [];
      var el = target && target.closest && target.closest("[data-hover-class]");
      while (el) {
        var blocked = el.hasAttribute("disabled") || el.classList.contains("wx-button-disabled");
        if (!blocked) list.push(el);
        if (blocked || el.getAttribute("data-hover-stop") === "1") break;
        el = el.parentElement && el.parentElement.closest("[data-hover-class]");
      }
      return list;
    }
    function pointOf(e) {
      var t = (e.touches && e.touches[0]) || (e.changedTouches && e.changedTouches[0]) || e;
      return { x: t.clientX, y: t.clientY };
    }
    function inside(el, p) {
      var r = el.getBoundingClientRect();
      return p.x >= r.left && p.x <= r.right && p.y >= r.top && p.y <= r.bottom;
    }
    function reset() {
      pending.forEach(function (item) {
        if (item.timer) clearTimeout(item.timer);
        hide(item.el);
      });
      pending = [];
      holding = false;
    }
    function onDown(e) {
      if (e.type === "mousedown" && fromTouch) return;
      if (e.type === "touchstart") fromTouch = true;
      reset();
      var list = chain(e.target);
      if (!list.length) return;
      holding = true;
      list.forEach(function (el) {
        var start = parseInt(el.getAttribute("data-hover-start"), 10);
        var stay = parseInt(el.getAttribute("data-hover-stay"), 10);
        if (isNaN(start)) start = 50;
        if (isNaN(stay)) stay = 400;
        var item = { el: el, stay: stay, shown: false, timer: null };
        item.timer = setTimeout(function () {
          item.timer = null;
          if (!holding) return;
          item.shown = true;
          show(el);
        }, start);
        pending.push(item);
      });
    }
    function onMove(e) {
      if (!holding) return;
      var p = pointOf(e);
      pending = pending.filter(function (item) {
        if (inside(item.el, p)) return true;
        if (item.timer) clearTimeout(item.timer);
        hide(item.el);
        return false;
      });
    }
    function onUp(e) {
      if (e.type === "mouseup" && fromTouch) return;
      if (e.type === "touchend" || e.type === "touchcancel") {
        setTimeout(function () { fromTouch = false; }, 700);
      }
      if (!holding) return;
      holding = false;
      pending.forEach(function (item) {
        if (item.timer) clearTimeout(item.timer);
        if (!item.shown) { hide(item.el); return; }
        setTimeout(function () { hide(item.el); }, item.stay);
      });
      pending = [];
    }
    document.addEventListener("touchstart", onDown, { passive: true });
    document.addEventListener("mousedown", onDown);
    document.addEventListener("touchmove", onMove, { passive: true });
    document.addEventListener("mousemove", function (e) { if (!fromTouch) onMove(e); });
    document.addEventListener("touchend", onUp, { passive: true });
    document.addEventListener("touchcancel", onUp, { passive: true });
    document.addEventListener("mouseup", onUp);
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
