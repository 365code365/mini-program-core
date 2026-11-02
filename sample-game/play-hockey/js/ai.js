// 电脑球拍：球在自己半场就进攻（绕到球的后面、对准对方球门推过去），在对方半场就回防
// （站在球与自家球门之间；球朝自己来时按墙面反弹预测它到防线的落点）。
// 三档难度差在：球拍最高速、多久重新看一次局面（反应）、瞄准的随机偏差。两边都能当电脑（自动试玩时左右对打）。
var P = require('./physics.js');
var LEVELS = [
  { name: '简单', speed: 560, react: 0.16, err: 60 },
  { name: '普通', speed: 900, react: 0.08, err: 32 },
  { name: '困难', speed: 1400, react: 0.03, err: 12 }
];

function make(world, mallet, level) {
  var rink = world.rink, lv = LEVELS[level], wait = 0, aimY = rink.cy, phase = 'idle', defErr = 0, wasToward = false;
  var s = mallet.side, ownX = s < 0 ? rink.L : rink.R, oppX = s < 0 ? rink.R : rink.L;
  var reach = P.PUCK_R + P.MALLET_R;
  mallet.maxSpeed = lv.speed;

  // 球在 x 处的 y（直线 + 上下墙镜面反射）
  function predictY(px, py, vx, vy, x) {
    if (Math.abs(vx) < 1) return py;
    var t = (x - px) / vx; if (t < 0) return py;
    var y = py + vy * t, lo = rink.T + P.PUCK_R, hi = rink.B - P.PUCK_R, span = hi - lo;
    var u = ((y - lo) % (2 * span) + 2 * span) % (2 * span);
    return lo + (u > span ? 2 * span - u : u);
  }
  function think(dt) {
    wait -= dt;
    if (wait > 0) return;
    wait = lv.react;
    var p = world.puck, inMine = s < 0 ? p.x < rink.mid : p.x > rink.mid;
    var toward = p.vx * s > 60;                         // 球正朝自家球门来
    if (inMine && !(toward && Math.abs(p.vx) > 700)) {
      // 球在自家球门与球拍之间：先绕回去（从球的上 / 下侧绕，别把球往自家门里捅）
      var behind = (p.x - mallet.x) * s > -reach * 0.3;
      if (behind) {
        phase = 'recover';
        mallet.tx = p.x + s * (reach + 18);
        mallet.ty = p.y + (p.y < rink.cy ? reach + 10 : -(reach + 10));
        return;
      }
      if (phase !== 'attack') aimY = rink.cy + (Math.random() * 2 - 1) * (rink.goalH / 2 - 10) + (Math.random() * 2 - 1) * lv.err;
      phase = 'attack';
      // 也会打边墙反弹：目标点关于上墙或下墙镜像
      var ay = aimY, bank = Math.abs(p.y - rink.cy) > rink.H * 0.28;
      if (bank) ay = p.y < rink.cy ? 2 * rink.T - aimY : 2 * rink.B - aimY;
      var dx = oppX - p.x, dy = ay - p.y, d = Math.sqrt(dx * dx + dy * dy) || 1;
      dx /= d; dy /= d;
      var ax = p.x - dx * (reach + 4), ay2 = p.y - dy * (reach + 4);
      var off = Math.sqrt((mallet.x - ax) * (mallet.x - ax) + (mallet.y - ay2) * (mallet.y - ay2));
      if (off < 22) { mallet.tx = p.x + dx * 60; mallet.ty = p.y + dy * 60; }     // 对准了：推过去
      else { mallet.tx = ax; mallet.ty = ay2; }
    } else {
      // 防守偏差进入防守时定一次（每次反应都重抽的话球拍会原地抖）
      // 每来一记新的射门重抽一次：球越快读得越不准（不然两个电脑对打谁也进不了球）
      var sp = Math.sqrt(p.vx * p.vx + p.vy * p.vy);
      if (phase !== 'defend' || (toward && !wasToward)) defErr = (Math.random() * 2 - 1) * lv.err * (0.5 + sp / 600);
      wasToward = toward;
      phase = 'defend';
      var lineX = ownX - s * rink.W * 0.1;
      var y = toward ? predictY(p.x, p.y, p.vx, p.vy, lineX) : rink.cy + (p.y - rink.cy) * 0.45;
      var g = rink.goalH / 2 + 6;
      mallet.tx = lineX;
      mallet.ty = Math.max(rink.cy - g, Math.min(rink.cy + g, y + defErr));
    }
  }
  return { think: think, level: lv, phase: function () { return phase; } };
}

module.exports = { make: make, LEVELS: LEVELS };
