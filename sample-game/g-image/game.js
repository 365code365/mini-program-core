// g-image（game-doc/05 §2、GT-I1~I7）：各种来源 / 格式的图依次加载，记 onload 前后尺寸与 onerror 参数；
// 最后把图画到主画布上，像素由 e2e 断言（9 参 drawImage、最近邻 / 双线性、Canvas 源、图案）。
// query：http=<端口>（e2e 起的本地服务：/px.png 正常、/404 不存在）
var T = require('_lib/t.js');
var c = wx.createCanvas(); var ctx = c.getContext('2d');
var DATA = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAQAAAACCAYAAAB/qH1jAAAAHElEQVR4nAXBAQ0AAAzDIJbcv+UeRBJVOgLFxgP5cwv7Sg3VMwAAAABJRU5ErkJggg==';
var srcs = [['png', 'img/px4x2.png'], ['png-abs', '/img/px4x2.png'], ['jpg', 'img/blue16x8.jpg'], ['webp', 'img/green5.webp'],
  ['gif', 'img/anim6.gif'], ['data', DATA], ['missing', 'img/nope.png'], ['broken', 'img/broken.png'], ['escape', '../../etc/passwd']];
if (T.q.http) { srcs.push(['http', 'http://127.0.0.1:' + T.q.http + '/px.png']); srcs.push(['http404', 'http://127.0.0.1:' + T.q.http + '/404']); }
// wxfile：把包里那张 png 拷进用户目录再读（与「下载后显示」同一条路）
try {
  var fs = wx.getFileSystemManager();
  fs.copyFileSync('img/px4x2.png', wx.env.USER_DATA_PATH + '/copy.png');
  srcs.push(['wxfile', wx.env.USER_DATA_PATH + '/copy.png']);
} catch (e) { T.ev('wxfile-setup-fail', { e: String(e) }); }

var imgs = {}, left = srcs.length;
srcs.forEach(function (s) {
  var img = wx.createImage();
  var before = [img.width, img.height, img.complete];
  img.onload = function (e) { T.ev('load', { k: s[0], before: before, after: [img.width, img.height], complete: img.complete, arg: e && e.type, self: this === img }); finish(); };
  img.onerror = function (e) { T.ev('error', { k: s[0], arg: e, w: img.width, complete: img.complete }); finish(); };
  img.src = s[1];
  T.ev('set', { k: s[0], w: img.width, complete: img.complete });
  imgs[s[0]] = img;
});
// I4：同一 src 并发设置 5 个 Image
var same = 0;
for (var i = 0; i < 5; i++) { var m = wx.createImage(); m.onload = function () { if (++same === 5) T.ev('same5', {}); }; m.src = 'img/edge4.png'; }

function finish() {
  if (--left > 0) return;
  var p = imgs.png;
  ctx.fillStyle = '#ffffff'; ctx.fillRect(0, 0, c.width, c.height);
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(p, 0, 0);                              // 3 参：(0,0) 起 4×2 原尺寸
  ctx.drawImage(p, 0, 10, 40, 20);                     // 5 参：放大 10 倍、最近邻
  ctx.drawImage(p, 1, 1, 1, 1, 60, 10, 8, 8);          // 9 参：只取 (1,1)（青）
  ctx.drawImage(imgs.gif, 0, 60, 12, 12);              // GIF：只画第一帧（绿）
  ctx.imageSmoothingEnabled = true;
  // Canvas 源：离屏画布画一块绿再贴上来
  var off = wx.createCanvas(); off.width = 10; off.height = 10;
  var oc = off.getContext('2d'); oc.fillStyle = '#00ff00'; oc.fillRect(0, 0, 10, 10);
  ctx.drawImage(off, 100, 0);
  // 图案：px4x2 平铺 12×4
  ctx.fillStyle = ctx.createPattern(p, 'repeat'); ctx.fillRect(120, 0, 12, 4);
  // 旋转：绕 (200,20) 转 90°，图 4×2 放大成 40×20 → 竖着 20 宽 40 高
  ctx.save(); ctx.translate(200, 20); ctx.rotate(Math.PI / 2); ctx.imageSmoothingEnabled = false; ctx.drawImage(p, 0, 0, 40, 20); ctx.restore();
  // I5：同一张图 10 000 次
  for (var k = 0; k < 10000; k++) ctx.drawImage(p, 300, 0);
  T.ev('drawn', { pattern: ctx.createPattern(p, 'no-repeat') !== null });
  T.done({});
}
T.ready('g-image');
