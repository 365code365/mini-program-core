/* 「小游戏」这一节的 3D 转盘（样式在 assets/games.css）：
   - 自己慢慢转（每秒 10°）；指针在转盘上时减速到停、跟着指针倾斜；按住左右拖可以手动转，松手带惯性；
   - 只在进视口时跑 rAF，滚出去就停（不白耗电）；
   - 不要动画 / 窄屏：不接管（CSS 已经拆成静态一排）。
   只造动效，不造文字（文字都在 HTML 里，i18n 才管得到）。 */
(function () {
  'use strict';
  var stage = document.querySelector('[data-gm]');
  if (!stage) return;
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  var narrow = window.matchMedia('(max-width: 720px)');
  var ring = stage.querySelector('.gm-ring');
  var ry = 0, vy = -10, target = -10, rx = -8, trx = -8, hovering = false, dragging = false, lastX = 0, lastT = 0, visible = false, raf = 0, prev = 0;

  function frame(t) {
    raf = 0;
    if (!visible || narrow.matches) return;
    var dt = prev ? Math.min(0.05, (t - prev) / 1000) : 0;
    prev = t;
    if (!dragging) {
      // 悬停时速度降到 0，离开后回到自转；拖完的惯性也在这里慢慢收回到自转速度
      var want = hovering ? 0 : target;
      vy += (want - vy) * Math.min(1, dt * (hovering ? 3.5 : 1.2));
      ry += vy * dt;
    }
    rx += (trx - rx) * Math.min(1, dt * 6);
    ring.style.transform = 'rotateX(' + rx.toFixed(2) + 'deg) rotateY(' + ry.toFixed(2) + 'deg)';
    raf = requestAnimationFrame(frame);
  }
  function kick() { if (!raf && visible && !narrow.matches) { prev = 0; raf = requestAnimationFrame(frame); } }

  stage.setAttribute('data-gm-live', '');
  ring.style.transition = 'none';

  stage.addEventListener('pointerenter', function () { hovering = true; });
  stage.addEventListener('pointerleave', function () { hovering = false; dragging = false; trx = -8; });
  stage.addEventListener('pointermove', function (e) {
    var r = stage.getBoundingClientRect();
    trx = -8 + ((e.clientY - r.top) / r.height - 0.5) * -14;
    if (dragging) {
      var now = performance.now(), dx = e.clientX - lastX;
      ry += dx * 0.35;
      if (now > lastT) vy = vy * 0.6 + (dx * 0.35) / ((now - lastT) / 1000) * 0.4;
      lastX = e.clientX; lastT = now;
    }
  });
  stage.addEventListener('pointerdown', function (e) {
    if (e.pointerType === 'mouse' && e.button !== 0) return;
    dragging = true; lastX = e.clientX; lastT = performance.now(); vy = 0;
  });
  window.addEventListener('pointerup', function () {
    if (!dragging) return;
    dragging = false;
    vy = Math.max(-240, Math.min(240, vy));
    hovering = false;   // 松手先带惯性转起来，悬停减速等下一次移动再接上
    setTimeout(function () { hovering = stage.matches(':hover'); }, 600);
  });

  if ('IntersectionObserver' in window) {
    new IntersectionObserver(function (es) {
      visible = es[0].isIntersecting;
      if (visible) kick();
    }, { rootMargin: '120px 0px' }).observe(stage);
  } else { visible = true; kick(); }
  narrow.addEventListener && narrow.addEventListener('change', function () {
    if (narrow.matches) ring.style.transform = ''; else kick();
  });
})();
