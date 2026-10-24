/* 落地页的背景主题：深色（默认）/ 午夜蓝 / 浅色 / 跟随系统。
   <head> 里的内联脚本已经按上次的选择设好了 <html data-theme>（不闪），这里只管：
   点色块切换、记住选择（localStorage `mr-theme`；地址里 `?theme=light` 优先）、跟随系统时听系统变化、改地址栏颜色（theme-color）。
   窄屏上只露出当前那个色块（style.css），点它换下一个。 */
(function () {
  'use strict';
  var KEY = 'mr-theme';
  var ORDER = ['dark', 'midnight', 'light', 'auto'];
  var BAR = { dark: '#07090d', midnight: '#060a1a', light: '#f7f8fb' };
  var root = document.documentElement;
  var box = document.querySelector('.theme-switch');
  var btns = box ? Array.prototype.slice.call(box.querySelectorAll('[data-theme-pick]')) : [];
  var sys = window.matchMedia ? window.matchMedia('(prefers-color-scheme: light)') : null;
  var narrow = window.matchMedia ? window.matchMedia('(max-width: 560px)') : null;

  // `?theme=light` 优先（截图、分享某个主题的链接用），不写进 localStorage
  var stored = function () {
    var q = (location.search.match(/[?&]theme=(\w+)/) || [])[1];
    if (q && ORDER.indexOf(q) >= 0) return q;
    try { return localStorage.getItem(KEY) || 'dark'; } catch (e) { return 'dark'; }
  };
  var resolve = function (pick) {
    if (pick === 'auto') return sys && sys.matches ? 'light' : 'dark';
    return BAR[pick] ? pick : 'dark';
  };
  var meta = document.querySelector('meta[name="theme-color"]');

  var apply = function (pick, animate) {
    var t = resolve(pick);
    if (animate) {
      // 只在切换那一下挂过渡（平时每个元素都带 transition 会拖慢滚动和入场动画）
      root.classList.add('theme-fade');
      setTimeout(function () { root.classList.remove('theme-fade'); }, 350);
    }
    root.setAttribute('data-theme', t);
    if (meta) meta.setAttribute('content', BAR[t]);
    btns.forEach(function (b) {
      b.setAttribute('aria-pressed', String(b.getAttribute('data-theme-pick') === pick));
      b.title = b.getAttribute('aria-label') || '';
    });
  };

  var current = stored();
  apply(current, false);

  btns.forEach(function (b) {
    b.addEventListener('click', function () {
      var pick = b.getAttribute('data-theme-pick');
      // 窄屏上只看得见当前这一个：点它就换下一个
      if (narrow && narrow.matches && pick === current) pick = ORDER[(ORDER.indexOf(current) + 1) % ORDER.length];
      current = pick;
      try { localStorage.setItem(KEY, pick); } catch (e) {}
      apply(pick, true);
    });
  });
  if (sys) {
    var onSys = function () { if (current === 'auto') apply('auto', true); };
    if (sys.addEventListener) sys.addEventListener('change', onSys); else if (sys.addListener) sys.addListener(onSys);
  }
  // 切了语言以后 aria-label 换了，提示文字跟着换
  document.addEventListener('mr:langchange', function () {
    btns.forEach(function (b) { b.title = b.getAttribute('aria-label') || ''; });
  });
})();
