// H5 落地页：进来就试一次拉起 App（URL Scheme）。浏览器要求用户手势时这一下会被拦，按钮兜底
'use strict';
(function () {
  var a = document.getElementById('open');
  if (!a) return;
  var href = a.getAttribute('data-href');
  if (href && href.indexOf('minirender://') === 0) {
    setTimeout(function () { location.href = href; }, 300);
  }
})();
