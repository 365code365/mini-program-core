// g-overlays（game-doc/05 GT-U1 / U3）：系统覆盖层盖在游戏上。游戏画一块满屏绿底（#208040），
// 触摸记 T:EV gameTouch（覆盖层吃掉的那串不该出现）；go(step) 由测试驱动，结果记 T:EV ui。
var T = require('_lib/t.js');
var c = wx.createCanvas(), g = c.getContext('2d');
function draw() { g.fillStyle = '#208040'; g.fillRect(0, 0, c.width, c.height); }
draw();
wx.onTouchStart(function (e) { T.ev('gameTouch', { x: e.changedTouches[0].clientX, y: e.changedTouches[0].clientY }); });
function cb(tag) {
  return {
    success: function (r) { T.ev('ui', { tag: tag, ok: true, r: r }); },
    fail: function (r) { T.ev('ui', { tag: tag, ok: false, r: r }); }
  };
}
globalThis.go = function (step) {
  if (step === 'toast') wx.showToast(Object.assign({ title: '成功', icon: 'success', duration: 1000 }, cb('toast')));
  if (step === 'loading') wx.showLoading(Object.assign({ title: '加载中', mask: true }, cb('loading')));
  if (step === 'hideLoading') wx.hideLoading();
  if (step === 'modal') wx.showModal(Object.assign({ title: '提示', content: '确定要继续吗？', confirmText: '好的' }, cb('modal')));
  if (step === 'sheet') wx.showActionSheet(Object.assign({ itemList: ['选项 A', '选项 B', '选项 C'] }, cb('sheet')));
};
// auto=1：真机上依次弹出，肉眼看 + 截图
if (T.q.auto) {
  var seq = ['toast', 'loading', 'hideLoading', 'modal'];
  seq.forEach(function (s, i) { setTimeout(function () { go(s); }, 800 + i * 1500); });
}
(function tick() { requestAnimationFrame(tick); })();
T.ready('g-overlays');
