/* mini-render 落地页的 3D 动效（样式在 assets/fx.css）：
   1. 演示视频：进视口时纸鹤从远处飞来 → 展开成视频 → 落定后跟着指针倾斜；点一下播放。
   2. 首屏手机组跟着指针在 3D 里转。
   3. 卡片悬停微倾（写 --tilt，style.css 的 .reveal.is-in 用它当 transform）。
   只造装饰，不造文字（文字都在 HTML 里，i18n 才管得到）。
   不要动画的人（prefers-reduced-motion）：直接落在终态，不飞、不倾。 */
(function () {
  'use strict';

  var reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  var hover = window.matchMedia('(hover: hover) and (pointer: fine)').matches;
  var raf = window.requestAnimationFrame.bind(window);

  /* ─────────────  演示视频  ───────────── */
  var band = document.querySelector('.showcase-band');
  var stage = band && band.querySelector('.sc-stage');
  if (stage) {
    var video = stage.querySelector('.sc-video');
    var card = stage.querySelector('.sc-card');
    var flight = stage.querySelector('.sc-flight');
    var tilt = stage.querySelector('.sc-tilt');
    var playBtn = stage.querySelector('.sc-play');
    var state = function (s) { stage.setAttribute('data-sc', s); };

    // 有 JS 才摘掉原生控件，换成自己的播放按钮；真开始播再把控件还回去
    video.removeAttribute('controls');
    var fallback = stage.querySelector('.sc-fallback');
    // 放不出来（浏览器不认 H.264 / VP9，比如没带专有解码器的 Electron 内嵌浏览器；或者网络断了）：
    // 别让人对着一张静止的海报干点，给一个直接打开视频文件的链接
    var broken = function () { if (fallback) fallback.hidden = false; };
    // <source> 的 error 不冒泡，捕获阶段才收得到；一个源失败浏览器会去试下一个（mp4 → webm），
    // 只有全部失败（NETWORK_NO_SOURCE）或解码出错（video.error）才算放不了
    video.addEventListener('error', function () {
      setTimeout(function () { if (video.error || video.networkState === 3) broken(); }, 0);
    }, true);
    var play = function () {
      if (stage.classList.contains('is-playing')) return;
      stage.classList.add('is-playing');
      stage.classList.remove('is-tilting');
      state('landed');
      video.controls = true;
      // preload="none" 时 Safari 有时不会因为 play() 去拉数据：先显式 load 一次
      if (video.readyState === 0 && video.networkState !== 2) video.load();
      var p = video.play();
      if (p && p.catch) p.catch(function (e) {
        // 被自动播放策略拦下（NotAllowedError）：控件已经在，让用户自己点；别的错误多半是放不了
        if (e && e.name !== 'NotAllowedError' && (video.error || video.networkState === 3)) broken();
      });
    };
    // 点舞台里任何地方都算：卡片在 3D 里倾斜，有的浏览器（WebKit）把点击命中到旁边的装饰层上，
    // 只挂在按钮上的话就成了「按钮在那儿、点了没反应」。播放之后交给原生控件，不再拦
    stage.addEventListener('click', function (ev) {
      if (stage.classList.contains('is-playing')) return;
      if (ev.target === fallback) return;
      ev.preventDefault();
      play();
    });

    var land = function () { state('landed'); };
    var unfold = function () {
      state('unfold');
      setTimeout(land, 1200);
    };

    /* 飞行路上撒星尘：每 34ms 在纸鹤当前（投影后的）中心放一颗，往随机方向散开 */
    var trail = function () {
      var n = 0;
      var timer = setInterval(function () {
        if (stage.getAttribute('data-sc') !== 'fly' || n++ > 80) { clearInterval(timer); return; }
        var s = stage.getBoundingClientRect();
        var r = card.getBoundingClientRect();
        var sp = document.createElement('i');
        sp.className = 'sc-spark';
        sp.style.left = (r.left - s.left + r.width * 0.5 + (Math.random() - 0.5) * r.width * 0.25) + 'px';
        sp.style.top = (r.top - s.top + r.height * 0.5 + (Math.random() - 0.5) * r.height * 0.25) + 'px';
        sp.style.setProperty('--sx', ((Math.random() - 0.5) * 60).toFixed(1) + 'px');
        sp.style.setProperty('--sy', (Math.random() * 40 + 10).toFixed(1) + 'px');
        sp.addEventListener('animationend', function () { sp.remove(); });
        stage.appendChild(sp);
      }, 34);
    };

    var fly = function () {
      state('fly');
      trail();
      var done = false;
      var next = function () { if (!done) { done = true; unfold(); } };
      flight.addEventListener('animationend', function (e) { if (e.animationName === 'sc-flight') next(); });
      setTimeout(next, 2700); // animationend 万一没来（标签页在后台被节流），也别卡在半空
    };

    if (reduced || !('IntersectionObserver' in window)) {
      land();
    } else {
      state('armed');
      var started = false;
      new IntersectionObserver(function (es) {
        es.forEach(function (e) {
          band.setAttribute('data-vis', e.isIntersecting ? '1' : '0');
          if (e.isIntersecting && e.intersectionRatio >= 0.2 && !started) { started = true; fly(); }
          // 滚走了就暂停（回来不自动续播：有声视频突然响起来很吓人）
          if (!e.isIntersecting && !video.paused) video.pause();
        });
      }, { threshold: [0, 0.2] }).observe(band.querySelector('.sc-stage'));
      // 兜底：滚到了却一直没起飞（嵌入式浏览器的 IntersectionObserver 不可靠），8 秒后直接落定
      setTimeout(function () { if (stage.getAttribute('data-sc') === 'armed' && band.getBoundingClientRect().top < innerHeight) land(); }, 8000);
    }

    /* 落定后的 3D 倾斜 + 高光跟着指针 */
    if (hover && !reduced) {
      var pend = null;
      stage.addEventListener('pointermove', function (ev) {
        if (stage.getAttribute('data-sc') !== 'landed' || stage.classList.contains('is-playing')) return;
        var first = pend === null;
        pend = ev;
        if (!first) return;
        raf(function () {
          var e = pend; pend = null;
          if (!e) return;
          var r = stage.getBoundingClientRect();
          var nx = (e.clientX - r.left) / r.width * 2 - 1;
          var ny = (e.clientY - r.top) / r.height * 2 - 1;
          stage.classList.add('is-tilting');
          tilt.style.setProperty('--ry', (nx * 6).toFixed(2) + 'deg');
          tilt.style.setProperty('--rx', (-ny * 4.5).toFixed(2) + 'deg');
          card.style.setProperty('--gx', ((nx + 1) * 50).toFixed(1) + '%');
          card.style.setProperty('--gy', ((ny + 1) * 50).toFixed(1) + '%');
        });
      }, { passive: true });
      stage.addEventListener('pointerleave', function () {
        stage.classList.remove('is-tilting');
        tilt.style.removeProperty('--rx');
        tilt.style.removeProperty('--ry');
      });
    }
  }

  if (reduced || !hover) return;

  /* ─────────────  首屏手机组：跟着指针转（静止时停在微微侧着的角度，见 fx.css）  ───────────── */
  var hero = document.querySelector('.hero');
  var shots = hero && hero.querySelector('.hero-shots');
  if (shots) {
    var hq = null;
    hero.addEventListener('pointermove', function (ev) {
      var first = hq === null;
      hq = ev;
      if (!first) return;
      raf(function () {
        var r = hero.getBoundingClientRect();
        var nx = (hq.clientX - r.left) / r.width * 2 - 1;
        var ny = (hq.clientY - r.top) / r.height * 2 - 1;
        hq = null;
        shots.style.setProperty('--hx', (-9 + nx * 10).toFixed(2) + 'deg');
        shots.style.setProperty('--hy', (4 - ny * 7).toFixed(2) + 'deg');
      });
    }, { passive: true });
    hero.addEventListener('pointerleave', function () {
      shots.style.removeProperty('--hx');
      shots.style.removeProperty('--hy');
    });
  }

  /* ─────────────  卡片悬停微倾  ─────────────
     只给已经入场的（.reveal.is-in）写 --tilt：入场前它的 transform 是入场动画的起点，别去抢。
     角度按卡片宽度缩：大卡片同样的角度两边高低差太大，看着像要翻倒。 */
  var cur = null;
  var tq = null;
  var apply = function () {
    var e = tq; tq = null;
    if (!e || !cur) return;
    var r = cur.getBoundingClientRect();
    var nx = (e.clientX - r.left) / r.width * 2 - 1;
    var ny = (e.clientY - r.top) / r.height * 2 - 1;
    var k = Math.min(1, 360 / Math.max(1, r.width));
    cur.style.setProperty('--tilt',
      'perspective(900px) rotateX(' + (-ny * 5 * k).toFixed(2) + 'deg) rotateY(' + (nx * 6 * k).toFixed(2) + 'deg) translateZ(6px)');
  };
  var release = function (el) {
    el.classList.remove('tilting');
    el.style.removeProperty('--tilt');
  };
  document.addEventListener('pointermove', function (ev) {
    var el = ev.target && ev.target.closest ? ev.target.closest('.card.reveal.is-in, .plat.reveal.is-in, .metric.reveal.is-in, .twin-tile.reveal.is-in') : null;
    if (el !== cur) {
      if (cur) release(cur);
      cur = el;
      if (cur) cur.classList.add('tilting');
    }
    if (!cur) return;
    var first = tq === null;
    tq = ev;
    if (first) raf(apply);
  }, { passive: true });
  document.addEventListener('pointerleave', function () { if (cur) { release(cur); cur = null; } });
})();
