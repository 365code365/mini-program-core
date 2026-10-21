  // ───────── SelectorQuery / IntersectionObserver（与原生端 host/selector_query.rs、host/observers.rs 同一套字段） ─────────
  // 选择器里的标签名换成运行时给每个组件加的 `wx-<标签>` 类（view 渲染成 div，按标签直接找不到）；
  // `>>>` 当后代。`exec` 异步回调（setTimeout 0），与微信一致。
  (function () {
    function toCss(sel) {
      return String(sel).replace(/>>>/g, " ").replace(/(^|[\s>,+~])([a-zA-Z][\w-]*)/g, "$1.wx-$2");
    }
    function rectOf(el) { var r = el.getBoundingClientRect(); return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height }; }
    function kebab(s) { return s.replace(/[A-Z]/g, function (c) { return "-" + c.toLowerCase(); }); }
    function typed(v) { if (v == null) return null; try { return JSON.parse(v); } catch (e) { return v; } }
    function scrollCtx(el) {
      return {
        scrollEnabled: true,
        scrollTo: function (o) { o = o || {}; el.scrollTo({ top: o.top, left: o.left, behavior: o.animated === false ? "auto" : "smooth" }); },
        scrollIntoView: function (sel, o) {
          o = o || {}; var t = el.querySelector(toCss(sel)); if (!t) return;
          var top = t.getBoundingClientRect().top - el.getBoundingClientRect().top + el.scrollTop + (Number(o.offset) || 0);
          el.scrollTo({ top: top, behavior: o.animated === false ? "auto" : "smooth" });
        },
        triggerRefresh: function () {}, closeRefresh: function () {}
      };
    }
    function fieldsOf(el, f) {
      var o = {};
      if (f.id) o.id = el.id || "";
      if (f.dataset) o.dataset = datasetOf(el);
      if (f.mark) o.mark = {};
      if (f.rect) { var r = rectOf(el); o.left = r.left; o.top = r.top; o.right = r.right; o.bottom = r.bottom; }
      if (f.size) { var s = rectOf(el); o.width = s.width; o.height = s.height; }
      if (f.scrollOffset) { o.scrollLeft = el.scrollLeft; o.scrollTop = el.scrollTop; o.scrollWidth = el.scrollWidth; o.scrollHeight = el.scrollHeight; }
      if (f.properties) f.properties.forEach(function (p) { o[p] = typed(el.getAttribute(p) != null ? el.getAttribute(p) : el.getAttribute(kebab(p))); });
      if (f.computedStyle) { var cs = getComputedStyle(el); f.computedStyle.forEach(function (p) { o[p] = cs.getPropertyValue(kebab(p)); }); }
      if (f.node) {
        if (el.tagName === "CANVAS") o.node = el;
        else if (el.classList.contains("wx-scroll-view")) o.node = scrollCtx(el);
        else o.node = null;
      }
      if (f.context) {
        if (el.tagName === "VIDEO") o.context = wx.createVideoContext(el.id);
        else if (el.tagName === "CANVAS") o.context = wx.createCanvasContext(el.getAttribute("data-canvas-id") || el.id);
        else o.context = null;
      }
      return o;
    }
    function viewportFields(f) {
      var o = {}, d = document.documentElement;
      if (f.id) o.id = "";
      if (f.dataset) o.dataset = {};
      if (f.rect) { o.left = 0; o.top = 0; o.right = d.clientWidth; o.bottom = d.clientHeight; }
      if (f.size) { o.width = d.clientWidth; o.height = d.clientHeight; }
      if (f.scrollOffset) { o.scrollLeft = window.scrollX; o.scrollTop = window.scrollY; o.scrollWidth = d.scrollWidth; o.scrollHeight = d.scrollHeight; }
      return o;
    }
    function Query() { this._q = []; }
    function Ref(q, sel, all, vp) { this._q = q; this._sel = sel; this._all = all; this._vp = vp; }
    Ref.prototype._push = function (f, cb) { this._q._q.push({ sel: this._sel, all: this._all, vp: this._vp, f: f, cb: cb }); return this._q; };
    Ref.prototype.boundingClientRect = function (cb) { return this._push({ id: true, dataset: true, rect: true, size: true }, cb); };
    Ref.prototype.scrollOffset = function (cb) { return this._push({ id: true, dataset: true, scrollOffset: true }, cb); };
    Ref.prototype.fields = function (f, cb) { return this._push(f || {}, cb); };
    Ref.prototype.node = function (cb) { return this._push({ node: true }, cb); };
    Ref.prototype.context = function (cb) { return this._push({ context: true }, cb); };
    Query.prototype.in = function () { return this; };
    Query.prototype.select = function (s) { return new Ref(this, s, false, false); };
    Query.prototype.selectAll = function (s) { return new Ref(this, s, true, false); };
    Query.prototype.selectViewport = function () { return new Ref(this, "", false, true); };
    Query.prototype.exec = function (cb) {
      var qs = this._q;
      setTimeout(function () {
        var res = qs.map(function (q) {
          if (q.vp) return viewportFields(q.f);
          var css = toCss(q.sel);
          if (q.all) return Array.prototype.map.call(document.querySelectorAll(css), function (el) { return fieldsOf(el, q.f); });
          var el = document.querySelector(css);
          return el ? fieldsOf(el, q.f) : null;
        });
        qs.forEach(function (q, i) { if (typeof q.cb === "function") q.cb(res[i]); });
        if (typeof cb === "function") cb(res);
      }, 0);
      return this;
    };
    wx.createSelectorQuery = function () { return new Query(); };

    // IntersectionObserver：用浏览器自己的，回调按「跨过阈值区间」过滤（与原生端同一套判定）
    function bucket(r, ts) { return ts.filter(function (t) { return t <= 0 ? r > 0 : r >= t - 1e-6; }).length; }
    function Obs(opt) {
      opt = opt || {};
      this._ts = (opt.thresholds && opt.thresholds.length ? opt.thresholds : [0]).map(Number).sort(function (a, b) { return a - b; });
      this._init = bucket(Number(opt.initialRatio) || 0, this._ts);
      this._all = !!opt.observeAll; this._root = null; this._margin = "0px"; this._ios = [];
    }
    function marginStr(m) { m = m || {}; return [m.top || 0, m.right || 0, m.bottom || 0, m.left || 0].map(function (v) { return v + "px"; }).join(" "); }
    Obs.prototype.relativeTo = function (sel, m) { this._root = document.querySelector(toCss(sel)); this._margin = marginStr(m); return this; };
    Obs.prototype.relativeToViewport = function (m) { this._root = null; this._margin = marginStr(m); return this; };
    Obs.prototype.observe = function (sel, cb) {
      var self = this, last = new Map();
      var io = new IntersectionObserver(function (entries) {
        entries.forEach(function (en) {
          var b = bucket(en.intersectionRatio, self._ts), prev = last.has(en.target) ? last.get(en.target) : self._init;
          last.set(en.target, b);
          if (b === prev) return;
          var rr = en.rootBounds || { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 };
          cb({ id: en.target.id || "", dataset: datasetOf(en.target), time: Date.now(), intersectionRatio: en.intersectionRatio,
               intersectionRect: rectPlain(en.intersectionRect), boundingClientRect: rectPlain(en.boundingClientRect), relativeRect: rectPlain(rr) });
        });
      }, { root: this._root, rootMargin: this._margin, threshold: this._ts });
      var css = toCss(sel), els = this._all ? Array.prototype.slice.call(document.querySelectorAll(css)) : [document.querySelector(css)];
      els.forEach(function (el) { if (el) io.observe(el); });
      this._ios.push(io);
    };
    function rectPlain(r) { return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height }; }
    Obs.prototype.disconnect = function () { this._ios.forEach(function (io) { io.disconnect(); }); this._ios = []; };
    wx.createIntersectionObserver = function (comp, opt) {
      if (comp && !opt && (comp.thresholds || comp.initialRatio !== undefined || comp.observeAll !== undefined)) opt = comp;
      return new Obs(opt);
    };
    wx.nextTick = function (fn) { if (typeof fn === "function") Promise.resolve().then(fn); };
  })();
