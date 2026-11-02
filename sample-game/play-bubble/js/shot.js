// 发射轨迹：从炮口沿角度 ang（弧度，0 = 向右，π/2 = 正上）走小步，碰左右墙镜面反弹，碰到泡泡或顶边就停，
// 再交给 grid.snap 找落点。瞄准线与真正的发射**用同一个函数**：瞄准线画出来的就是泡泡真会走的路、真会停的格子。
// 步长取半径的 0.5 倍：两颗泡泡相碰的判定距离是 1.76r，步长远小于它，斜着擦过去也不会穿模。
function simulate(grid, x, y, ang, maxBounces) {
  var r = grid.r, L = grid.left + r, Rx = grid.left + grid.cols * 2 * r - r;
  var dx = Math.cos(ang), dy = -Math.sin(ang), step = r * 0.5, reach = r * 1.76;
  var pts = [[x, y]], bounces = 0;
  for (var n = 0; n < 4000; n++) {
    x += dx * step; y += dy * step;
    if (x < L) { x = 2 * L - x; dx = -dx; pts.push([L, y]); bounces++; }
    else if (x > Rx) { x = 2 * Rx - x; dx = -dx; pts.push([Rx, y]); bounces++; }
    if (maxBounces !== undefined && bounces > maxBounces) break;
    if (y <= grid.top + r || grid.collides(x, y, reach)) break;
    if (y > grid.top + 40 * grid.rowH) break;
  }
  pts.push([x, y]);
  return { pts: pts, cell: grid.snap(x, y), bounces: bounces };
}
// 沿折线走 s 像素后的位置（发射动画用）；out 复用，不在帧里分配
function along(pts, s, out) {
  for (var i = 0; i + 1 < pts.length; i++) {
    var a = pts[i], b = pts[i + 1], dx = b[0] - a[0], dy = b[1] - a[1], l = Math.sqrt(dx * dx + dy * dy);
    if (s <= l) { var k = l ? s / l : 0; out[0] = a[0] + dx * k; out[1] = a[1] + dy * k; out[2] = 0; return out; }
    s -= l;
  }
  var e = pts[pts.length - 1]; out[0] = e[0]; out[1] = e[1]; out[2] = 1;
  return out;
}
module.exports = { simulate: simulate, along: along };
