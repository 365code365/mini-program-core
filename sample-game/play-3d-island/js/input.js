// 触摸分发：按下那一刻判归属 —— 落在界面上的整串（移动、抬起）都给界面；落在 3D 上的给镜头（单指平移、双指捏 / 拧）。
// 3D 上的「点」：一根手指、移动不到 10 像素、按住不到 0.45 秒，且期间没有第二根手指 —— 才当点击（点建筑 / 地块）。
function make(G) {
  var touches = {}, world = {}, tap = null;
  function list() { return Object.keys(world).map(function (id) { return world[id]; }); }
  wx.onTouchStart(function (e) {
    e.changedTouches.forEach(function (t) {
      var p = { id: t.identifier, x: t.clientX, y: t.clientY, x0: t.clientX, y0: t.clientY, t0: Date.now() };
      if (G.ui.down(p.x, p.y)) { touches[p.id] = 'ui'; return; }
      touches[p.id] = 'world'; world[p.id] = p;
      var l = list();
      tap = l.length === 1 ? p : null;
      G.camera.begin(l);
    });
  });
  wx.onTouchMove(function (e) {
    var moved = false;
    e.changedTouches.forEach(function (t) {
      var who = touches[t.identifier];
      if (who === 'ui') G.ui.move(t.clientX, t.clientY);
      else if (who === 'world') { var p = world[t.identifier]; p.x = t.clientX; p.y = t.clientY; moved = true;
        if (tap === p && Math.abs(p.x - p.x0) + Math.abs(p.y - p.y0) > 10) tap = null; }
    });
    if (moved) G.camera.move(list());
  });
  function end(e, cancel) {
    e.changedTouches.forEach(function (t) {
      var who = touches[t.identifier]; delete touches[t.identifier];
      if (who === 'ui') { if (!cancel) G.ui.up(t.clientX, t.clientY); else G.ui.up(-1e6, -1e6); return; }
      if (who !== 'world') return;
      var p = world[t.identifier]; delete world[t.identifier];
      if (!cancel && tap === p && Date.now() - p.t0 < 450) G.act.tapWorld(p.x, p.y);
      tap = null;
      G.camera.end(list());
    });
  }
  wx.onTouchEnd(function (e) { end(e, false); });
  wx.onTouchCancel(function (e) { end(e, true); });
  // PC：滚轮缩放、方向键平移
  if (wx.onWheel) wx.onWheel(function (e) { G.camera.zoomBy(e.deltaY > 0 ? 1.1 : 0.9); });
  return {};
}
module.exports = { make: make };
