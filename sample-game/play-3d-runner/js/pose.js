// 角色动作：跑、跳、铲、换道、撞上。模型层级见 js/hero.js。
//
// 为什么 v1 看着僵：相机在正后方偏上，四肢前后摆（绕 x 轴）从背后看几乎全被透视吃掉了，剩下的就是一块板在平移。
// 从背后读得出「在跑」的是：
//   - 重心左右压（root 横移 + 侧滚），每步一次；
//   - 胯与肩反向扭（hips / chest 绕 y 轴），手臂往身后摆时手肘从侧面露出来；
//   - 小腿往后踢（膝盖在腿往前收的半周弯到 ~100°），鞋底朝上翻出来 —— 背后视角最显眼的一个动作；
//   - 一步一颠（两步一周期），背包跟着晚半拍晃（二阶弹簧）；
//   - 手臂稍微张开（绕 z 轴），不贴着身体，剪影才有缝。
// 跑 / 空中 / 铲 / 撞四套姿势每个关节 x、z 两个角各一套，按平滑过的权重混合：起跳、落地、开铲、起身都是几十毫秒的过渡。
// 换道按横向速度 p.vx（game.js 的弹簧给出，从 0 起、到 0 止）侧倾、转头、张手，也是渐入渐出。
function make(H) {
  var J = [H.legL.a, H.legL.b, H.legR.a, H.legR.b, H.armL.a, H.armL.b, H.armR.a, H.armR.b];
  var N = J.length, rx = new Array(N), rz = new Array(N);
  var air = 0, slide = 0, over = 0, land = 0, wasAir = false, bagA = 0, bagV = 0, lastBob = 0;
  function ease(cur, tgt, dt, k) { return cur + (tgt - cur) * (1 - Math.exp(-dt * k)); }
  //            胯L   膝L   胯R   膝R   肩L   肘L   肩R   肘R
  var AIR_X  = [1.25, -1.9, -0.35, -1.2, 0.9, 1.0, -0.7, 0.8];        // 前腿高抬、后腿屈；一手往前上、一手往后
  var AIR_Z  = [0.05, 0, -0.08, 0, -0.75, 0, 0.9, 0];                  // 手臂张开保持平衡
  // 后仰铲：body 往后仰 1.1rad 时，胯落到 0.93·cos1.1 ≈ 0.42m，腿相对角 ≈ 0 时正好朝前下方伸直贴地；后腿收在身下
  var SLD_X  = [-0.05, -0.05, -0.95, -1.7, -2.1, 0.3, 0.7, 0.9];
  var SLD_Z  = [0.1, 0, -0.15, 0, -0.35, 0, 0.3, 0];
  var OVR_X  = [0.7, -0.5, 0.1, -0.7, -0.4, 0.5, -0.3, 0.6];
  var OVR_Z  = [0.15, 0, -0.15, 0, -1.6, 0, 1.5, 0];
  return {
    reset: function () { air = slide = over = land = 0; bagA = bagV = 0; },
    apply: function (p, state, clock, dt) {
      dt = Math.min(dt || 1 / 60, 0.05);
      var ready = state === 'ready', ph = ready ? clock * 6 : p.run, amp = ready ? 0.55 : 1;
      var s = Math.sin(ph), c = Math.cos(ph);
      // ── 跑 ──
      // 大腿：sin 摆；膝：腿往前收（cos > 0）的那半周后踢最深，着地前伸直；手臂反相，肘一直屈 ~80°，往后摆时再屈一点
      rx[0] = s * 0.95 * amp;  rx[1] = -(0.25 + 1.55 * Math.pow(Math.max(0, c), 1.4)) * amp;
      rx[2] = -s * 0.95 * amp; rx[3] = -(0.25 + 1.55 * Math.pow(Math.max(0, -c), 1.4)) * amp;
      rx[4] = -s * 0.9 * amp;  rx[5] = 1.25 + Math.max(0, s) * 0.35;
      rx[6] = s * 0.9 * amp;   rx[7] = 1.25 + Math.max(0, -s) * 0.35;
      rz[0] = 0.04; rz[1] = 0; rz[2] = -0.04; rz[3] = 0;
      rz[4] = -0.32 - Math.max(0, s) * 0.18; rz[5] = 0;                  // 往后摆时手臂外张，手肘从身侧露出来
      rz[6] = 0.32 + Math.max(0, -s) * 0.18; rz[7] = 0;
      // ── 姿势权重 ──
      var inAir = p.y > 0.05;
      if (wasAir && !inAir) land = 1;                                    // 落地缓冲：膝盖一沉再起
      wasAir = inAir;
      land = Math.max(0, land - dt * 5);
      air = ease(air, inAir ? 1 : 0, dt, inAir ? 14 : 24);
      slide = ease(slide, p.slide > 0 ? 1 : 0, dt, 18);
      over = ease(over, state === 'over' ? 1 : 0, dt, 10);
      for (var i = 0; i < N; i++) {
        var x = rx[i] + (AIR_X[i] - rx[i]) * air, z = rz[i] + (AIR_Z[i] - rz[i]) * air;
        x += (SLD_X[i] - x) * slide; z += (SLD_Z[i] - z) * slide;
        x += (OVR_X[i] - x) * over;  z += (OVR_Z[i] - z) * over;
        J[i].rotation.x = x; J[i].rotation.z = z;
      }
      var run = (1 - air) * (1 - slide) * (1 - over) * amp;
      // 落地：两膝一起屈、大腿前收
      var ld = land * land * (1 - slide);
      J[0].rotation.x += ld * 0.5; J[2].rotation.x += ld * 0.5; J[1].rotation.x -= ld * 0.9; J[3].rotation.x -= ld * 0.9;
      // ── 躯干 ──
      var bob = run * s * s * 0.09 - ld * 0.16;                          // 两步一颠：腿交叉（支撑中期）时最低
      var vx = p.vx || 0, turn = Math.max(-1, Math.min(1, vx / 14));
      H.body.position.set(run * c * 0.045, 0.09 * run - bob + slide * 0.04 + Math.abs(turn) * 0.06 * (1 - slide), 0);
      // 前倾跑；空中按竖直速度先仰后俯；铲到后仰；撞上往后倒
      var vy = p.vy || 0;
      H.body.rotation.x = -0.2 * run - (ready ? 0.03 : 0) + air * Math.max(-0.25, Math.min(0.2, -vy * 0.015)) * (1 - slide)
        + slide * 1.1 + over * 0.55;
      H.body.rotation.z = run * c * 0.07;                                 // 重心压到支撑脚那边
      H.hips.rotation.y = run * s * 0.28;
      H.chest.rotation.y = -run * s * 0.38 + slide * 0.25;
      H.chest.rotation.x = run * 0.06 - air * 0.1;
      H.head.rotation.y = run * s * 0.25 - turn * 0.35;                   // 头稳住（抵消肩的扭），换道时看向要去的道
      H.head.rotation.x = 0.12 * (1 - slide) - slide * 0.85 - air * 0.15;
      // 背包：二阶弹簧追着身体的上下颠，晚半拍晃
      var bv = (bob - lastBob) / dt; lastBob = bob;
      bagV += (-160 * bagA - 14 * bagV + bv * 9) * dt; bagA += bagV * dt;
      H.bag.rotation.x = Math.max(-0.4, Math.min(0.4, bagA)) + slide * -0.3;
      // ── 换道：侧倾、转身、双手张开保持平衡 ──
      H.root.position.set(p.x, p.y, 0);
      H.root.rotation.z = -turn * 0.4;
      H.root.rotation.y = -turn * 0.45;
      var bal = Math.abs(turn) * (1 - slide);
      J[4].rotation.z -= bal * 0.7; J[6].rotation.z += bal * 0.7;
      J[0].rotation.z += turn * 0.25 * (1 - slide); J[2].rotation.z += turn * 0.25 * (1 - slide);   // 腿往外蹬
    }
  };
}
module.exports = { make: make };
