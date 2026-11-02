// 一个页面里的多个倒计时共用一个 setInterval：订阅者数从 0 变 1 时开表，变回 0 时关表。
// 页面在 onUnload（tab 页在 onHide）里调 on() 返回的函数退订，最后一个退订时定时器就清掉了。
const subs = [];
let timer = null;

function on(fn) {
  subs.push(fn);
  if (!timer) timer = setInterval(() => { const now = Date.now(); subs.slice().forEach((f) => f(now)); }, 1000);
  return function off() {
    const i = subs.indexOf(fn);
    if (i >= 0) subs.splice(i, 1);
    if (!subs.length && timer) { clearInterval(timer); timer = null; }
  };
}

function running() { return !!timer; }

// 页面用法：onShow 里 attach、onHide / onUnload 里 detach。
// 另加一道保险：页面被 navigateTo 盖住时（引擎目前不给被盖住的页派 onHide），回调发现自己不在栈顶就自行退订，
// 回到这页时 onShow 再 attach。
function attach(page, fn) {
  if (page._tickOff) return;
  page._tickOff = on((now) => {
    const top = getCurrentPages().slice(-1)[0];
    if (top && top !== page) { detach(page); return; }
    fn(now);
  });
}
function detach(page) {
  if (page._tickOff) { page._tickOff(); page._tickOff = null; }
}

// 剩余毫秒 → { h, m, s, text }，过期返回 null
function left(endAt, now) {
  const ms = endAt - (now || Date.now());
  if (ms <= 0) return null;
  const t = Math.floor(ms / 1000);
  const pad = (n) => (n < 10 ? '0' + n : '' + n);
  const h = pad(Math.floor(t / 3600)), m = pad(Math.floor((t % 3600) / 60)), s = pad(t % 60);
  return { h, m, s, text: `${h}:${m}:${s}` };
}

module.exports = { on, attach, detach, left, running, count: () => subs.length };
