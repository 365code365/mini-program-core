/* mini-render 落地页交互：入场动画、吸顶描边、阅读进度、数字滚动、指针光斑、
   帧条、截图跑马灯、代码 Tab、复制按钮。
   没有 JS 也能正常读全文（.js-anim 由 <head> 里的内联脚本加上）。
   这里只造装饰性的东西（帧条的格子、跑马灯里的截图副本），不造文字 —— 文字都在 HTML 里，
   中英切换（assets/i18n.js）才管得到。 */
(function () {
  'use strict';

  var reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var each = function (sel, fn) { Array.prototype.forEach.call(document.querySelectorAll(sel), fn); };

  /* ── 吸顶描边 + 顶部阅读进度（一帧最多算一次） ── */
  var head = document.querySelector('.site-head');
  var bar = document.querySelector('.scroll-progress span');
  var ticking = false;
  var onScroll = function () {
    if (ticking) return;
    ticking = true;
    requestAnimationFrame(function () {
      ticking = false;
      var y = window.scrollY;
      if (head) head.classList.toggle('is-stuck', y > 8);
      if (bar) {
        var max = document.documentElement.scrollHeight - window.innerHeight;
        bar.style.setProperty('--p', max > 0 ? Math.min(1, y / max).toFixed(4) : '0');
      }
    });
  };
  onScroll();
  window.addEventListener('scroll', onScroll, { passive: true });
  window.addEventListener('resize', onScroll, { passive: true });

  /* ── 帧条：90 格 = 90Hz 下的一秒，红格数量按实测的重复比例 ──
     位置用固定种子的伪随机撒开（每次打开都一样，不会一刷新就换一种「数据」） */
  each('.strip', function (strip) {
    var n = Number(strip.dataset.frames) || 90;
    var reps = Math.round((Number(strip.dataset.rate) || 0) * n);
    var seed = Number(strip.dataset.seed) || 1;
    var rnd = function () { seed = (seed * 1103515245 + 12345) % 2147483648; return seed / 2147483648; };
    var marked = {};
    var guard = 0;
    while (Object.keys(marked).length < reps && guard++ < 10000) marked[Math.floor(rnd() * n)] = true;
    var frag = document.createDocumentFragment();
    for (var i = 0; i < n; i++) {
      var c = document.createElement('i');
      if (marked[i]) c.className = 'rep';
      frag.appendChild(c);
    }
    strip.appendChild(frag);
  });

  /* ── 截图跑马灯：从画廊卡片里复制一份（两遍首尾相接，平移一半正好无缝） ── */
  var track = document.querySelector('.marquee-track');
  if (track) {
    var srcs = [];
    each('.shots img', function (img) { srcs.push(img.getAttribute('src')); });
    each('.hero-shots img, .phone-device img', function (img) { srcs.push(img.getAttribute('src')); });
    var frag2 = document.createDocumentFragment();
    for (var k = 0; k < 2; k++) {
      srcs.forEach(function (src) {
        var im = document.createElement('img');
        im.src = src; im.alt = ''; im.loading = 'lazy'; im.decoding = 'async';
        im.width = 120; im.height = 214;
        frag2.appendChild(im);
      });
    }
    track.appendChild(frag2);
  }

  /* ── 指针光斑：卡片里跟着指针走的那团光 ── */
  if (!reduced && window.matchMedia('(hover: hover)').matches) {
    document.addEventListener('pointermove', function (ev) {
      var el = ev.target && ev.target.closest ? ev.target.closest('.spot') : null;
      if (!el) return;
      var r = el.getBoundingClientRect();
      el.style.setProperty('--mx', (ev.clientX - r.left) + 'px');
      el.style.setProperty('--my', (ev.clientY - r.top) + 'px');
    }, { passive: true });
  }

  /* ── 数字滚动：数字条进视口时从 0 数到实测值（小数位数跟原文一样） ── */
  var countUp = function (el) {
    var raw = el.dataset.count || el.textContent;
    var to = Number(raw);
    if (!isFinite(to) || reduced) return;
    var digits = (raw.split('.')[1] || '').length;
    var t0 = null;
    var dur = 1300;
    var step = function (now) {
      if (t0 === null) t0 = now;
      var p = Math.min(1, (now - t0) / dur);
      var e = 1 - Math.pow(1 - p, 3);
      el.textContent = (to * e).toFixed(digits);
      if (p < 1) requestAnimationFrame(step);
      else el.textContent = raw;
    };
    requestAnimationFrame(step);
  };

  /* ── 进入视口再显示（性能条同时开始生长） ── */
  var targets = [];
  ['.sec-head', '.card', '.steps li', '.shot', '.bar-row', '.doc', '.deps li',
   '.stats > div', '.table-scroll', '.callout', '.tabs', '.split > div', '.flow',
   '.metric', '.twin-tile', '.phone-device', '.be-points li']
    .forEach(function (sel) {
      Array.prototype.push.apply(targets, document.querySelectorAll(sel));
    });

  if (!reduced && 'IntersectionObserver' in window) {
    var io = new IntersectionObserver(function (entries) {
      entries.forEach(function (e) {
        if (!e.isIntersecting) return;
        var el = e.target;
        // 同一行内的元素依次亮起，错开 45ms，最多排到 6 个
        var delay = (Number(el.dataset.i) || 0) * 45;
        setTimeout(function () {
          el.classList.add('is-in');
          var num = el.querySelector && el.querySelector('[data-count]');
          if (num) countUp(num);
        }, delay);
        io.unobserve(el);
      });
    }, { rootMargin: '0px 0px -8% 0px', threshold: 0.08 });

    targets.forEach(function (el, i) {
      el.classList.add('reveal');
      el.dataset.i = String(i % 6);
      io.observe(el);
    });
  } else {
    targets.forEach(function (el) { el.classList.add('is-in'); });
  }

  /* ── 代码 Tab（左右键 / Home / End 可切换） ── */
  document.querySelectorAll('[data-tabs]').forEach(function (box) {
    var tabs = Array.prototype.slice.call(box.querySelectorAll('[role="tab"]'));
    if (!tabs.length) return;

    var select = function (tab, focus) {
      tabs.forEach(function (t) {
        var on = t === tab;
        t.setAttribute('aria-selected', String(on));
        t.tabIndex = on ? 0 : -1;
        var panel = document.getElementById(t.getAttribute('aria-controls'));
        if (panel) panel.hidden = !on;
      });
      if (focus) tab.focus();
    };

    tabs.forEach(function (tab, i) {
      tab.addEventListener('click', function () { select(tab); });
      tab.addEventListener('keydown', function (ev) {
        var next = null;
        if (ev.key === 'ArrowRight') next = tabs[(i + 1) % tabs.length];
        else if (ev.key === 'ArrowLeft') next = tabs[(i - 1 + tabs.length) % tabs.length];
        else if (ev.key === 'Home') next = tabs[0];
        else if (ev.key === 'End') next = tabs[tabs.length - 1];
        if (next) { ev.preventDefault(); select(next, true); }
      });
    });
  });

  /* ── 每段代码加一个复制按钮 ── */
  /* 文案走 i18n（assets/i18n.js）：默认英文，切中文时靠 mr:langchange 事件回来重写。 */
  var T = function (key, fallback) {
    return (window.I18N && window.I18N.t(key)) || fallback;
  };
  var copyBtns = [];
  document.querySelectorAll('.tabpanel').forEach(function (panel) {
    var code = panel.querySelector('pre code');
    if (!code || !navigator.clipboard) return;
    var btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'copy';
    btn.textContent = T('ui.copy', 'Copy');
    btn.setAttribute('aria-label', T('ui.copyAria', 'Copy this snippet'));
    btn.addEventListener('click', function () {
      // 按当时的 DOM 取：切过语言之后 <pre> 里的 <code> 换成了另一份
      var now = panel.querySelector('pre code') || code;
      navigator.clipboard.writeText(now.textContent).then(function () {
        btn.textContent = T('ui.copied', 'Copied');
        btn.classList.add('done');
        setTimeout(function () {
          btn.textContent = T('ui.copy', 'Copy');
          btn.classList.remove('done');
        }, 1600);
      }, function () {
        btn.textContent = T('ui.copyFail', 'Copy failed');
        setTimeout(function () { btn.textContent = T('ui.copy', 'Copy'); }, 1600);
      });
    });
    panel.appendChild(btn);
    copyBtns.push(btn);
  });
  /* 切语言时把已经生成的按钮也换掉（正在显示「已复制」的那个不动，1.6s 后自己回位） */
  document.addEventListener('mr:langchange', function () {
    copyBtns.forEach(function (btn) {
      if (!btn.classList.contains('done')) btn.textContent = T('ui.copy', 'Copy');
      btn.setAttribute('aria-label', T('ui.copyAria', 'Copy this snippet'));
    });
  });
})();
