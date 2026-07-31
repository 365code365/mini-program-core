/* mini-render 落地页交互：入场动画、吸顶描边、代码 Tab、复制按钮。
   没有 JS 也能正常读全文（.js-anim 由 <head> 里的内联脚本加上）。 */
(function () {
  'use strict';

  var reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  /* ── 吸顶时给头部加一条描边 ── */
  var head = document.querySelector('.site-head');
  if (head) {
    var onScroll = function () {
      head.classList.toggle('is-stuck', window.scrollY > 8);
    };
    onScroll();
    window.addEventListener('scroll', onScroll, { passive: true });
  }

  /* ── 进入视口再显示（性能条同时开始生长） ── */
  var targets = [];
  ['.sec-head', '.card', '.steps li', '.shot', '.bar-row', '.doc', '.deps li',
   '.stats > div', '.table-scroll', '.callout', '.tabs', '.split > div', '.flow']
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
        setTimeout(function () { el.classList.add('is-in'); }, delay);
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
  document.querySelectorAll('.tabpanel').forEach(function (panel) {
    var code = panel.querySelector('pre code');
    if (!code || !navigator.clipboard) return;

    var btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'copy';
    btn.textContent = '复制';
    btn.setAttribute('aria-label', '复制这段代码');

    btn.addEventListener('click', function () {
      navigator.clipboard.writeText(code.textContent).then(function () {
        btn.textContent = '已复制';
        btn.classList.add('done');
        setTimeout(function () {
          btn.textContent = '复制';
          btn.classList.remove('done');
        }, 1600);
      }, function () {
        btn.textContent = '复制失败';
        setTimeout(function () { btn.textContent = '复制'; }, 1600);
      });
    });

    panel.appendChild(btn);
  });
})();
