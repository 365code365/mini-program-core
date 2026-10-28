// 角色与招式表（策划配置）。时间单位一律是「帧」（逻辑固定 60 步 / 秒，与显示刷新率无关），距离是世界单位（站立身高 BODY）。
//
// 招式字段：
//   pose      用图集里哪个动作
//   s / a / r 发生（startup）/ 持续（active）/ 收招（recovery）帧
//   dmg       伤害（满血 1000）；chip 被挡时的削血
//   hs / bs   打中 / 被挡时对手的硬直帧（比自己剩下的收招长 = 有利，可以连段）
//   push      把对手推开的距离（打中按 push，被挡按 0.7 倍）
//   y0 / y1   判定框的上下沿（离脚底的高度）；框的前沿 = 这个动作的图里拳 / 脚伸出去的最远处（fighter.js 按图集算），宽 w
//   lv        'mid' 站蹲都能挡、'low' 只能蹲挡、'high' 只能站挡（跳攻）
//   kd        打中就击倒；launch 把对手打飞的竖直速度
//   cancel    打中 / 被挡后收招里能取消成必杀技（连段的基础）
//   lunge     发招时往前冲的速度；rise 升龙拳往上跳的速度；inv 发生前几帧无敌
//   proj      飞行道具：{ v: 速度, r: 半径, hits: 段数, every: 每段间隔帧, life: 寿命帧, big: 超必杀 }
//   stop      打中时双方定住的帧（打击感：轻 6、重 11、超 14）；shake 镜头震动；spark 火花大小
var BODY = 260;
var HP = 1000;
var METER_MAX = 300, STOCK = 100;          // 能量槽 3 格，超必杀用 1 格

var NORMALS = {
  lp: { pose: 'lp', s: 3, a: 3, r: 7, dmg: 32, hs: 14, bs: 10, push: 26, y0: 170, y1: 225, w: 70, lv: 'mid', cancel: true, stop: 6, shake: 0, spark: 0.7, sfx: 'light', meter: 6, chain: true },
  lk: { pose: 'lk', s: 5, a: 3, r: 10, dmg: 42, hs: 15, bs: 10, push: 34, y0: 95, y1: 150, w: 80, lv: 'mid', cancel: true, stop: 7, shake: 0, spark: 0.8, sfx: 'light', meter: 7 },
  hp: { pose: 'hp', s: 7, a: 4, r: 17, dmg: 85, hs: 22, bs: 15, push: 52, y0: 160, y1: 225, w: 90, lv: 'mid', cancel: true, stop: 11, shake: 5, spark: 1.2, sfx: 'heavy', meter: 12, lunge: 140 },
  hk: { pose: 'hk', s: 9, a: 4, r: 19, dmg: 95, hs: 22, bs: 15, push: 60, y0: 175, y1: 285, w: 95, lv: 'mid', cancel: false, stop: 11, shake: 6, spark: 1.3, sfx: 'heavy', meter: 12, lunge: 90 },
  // 下段：蹲着出拳 / 轻脚是小扫（不倒），蹲着重拳 / 重脚是大扫（击倒）
  cl: { pose: 'ck', s: 4, a: 3, r: 9, dmg: 30, hs: 14, bs: 9, push: 24, y0: 0, y1: 55, w: 90, lv: 'low', cancel: true, stop: 6, shake: 0, spark: 0.7, sfx: 'light', meter: 6, low: true },
  ch: { pose: 'ck', s: 8, a: 4, r: 22, dmg: 70, hs: 0, bs: 15, push: 40, y0: 0, y1: 55, w: 110, lv: 'low', kd: true, stop: 10, shake: 5, spark: 1.1, sfx: 'heavy', meter: 10, low: true, lunge: 120 },
  // 空中：任何攻击键都是飞踢，要站着挡
  jk: { pose: 'jk', s: 4, a: 14, r: 0, dmg: 72, hs: 18, bs: 12, push: 30, y0: -10, y1: 110, w: 100, lv: 'high', stop: 9, shake: 3, spark: 1.0, sfx: 'heavy', meter: 9, air: true }
};

// 每个角色的必杀技 / 超必杀（数值不同，手感不同：焰慢而重，霜快而轻）
var CHARS = {
  yan: {
    name: '焰', full: '炎城 烈', title: '烈焰拳士', color: '#ff6a2a', glow: [1, 0.45, 0.12], spark: [1, 0.7, 0.25],
    walk: 3.6, back: 2.9, jumpV: 19, jumpX: 4.6, dash: 9.5,
    s1: { name: '炎龙波', pose: 'sp', s: 13, a: 1, r: 30, dmg: 80, chip: 16, hs: 22, bs: 18, push: 40, lv: 'mid', stop: 10, shake: 4, spark: 1.3, sfx: 'fire', meter: 15,
      proj: { v: 9.5, r: 46, hits: 1, life: 150, y: 165 } },
    s2: { name: '升炎拳', pose: 'up', s: 4, a: 12, r: 32, dmg: 125, chip: 20, hs: 0, bs: 20, push: 30, y0: 80, y1: 330, w: 85, lv: 'mid', kd: true, launch: 15, inv: 8,
      stop: 12, shake: 7, spark: 1.6, sfx: 'rise', meter: 15, rise: 17, riseX: 2.5, fire: true },
    su: { name: '天焰灭世', pose: 'sp', s: 28, a: 1, r: 40, dmg: 52, chip: 10, hs: 26, bs: 20, push: 22, lv: 'mid', stop: 7, shake: 9, spark: 1.8, sfx: 'super', meter: 0,
      proj: { v: 12, r: 95, hits: 5, every: 4, life: 120, y: 150, big: true } }
  },
  shuang: {
    name: '霜', full: '白 霜华', title: '寒霆女侠', color: '#58c8ff', glow: [0.3, 0.75, 1], spark: [0.55, 0.9, 1],
    walk: 4.1, back: 3.3, jumpV: 19.5, jumpX: 5.0, dash: 10.5,
    s1: { name: '霜刃波', pose: 'sp', s: 11, a: 1, r: 27, dmg: 70, chip: 14, hs: 20, bs: 16, push: 36, lv: 'mid', stop: 9, shake: 3, spark: 1.2, sfx: 'ice', meter: 15,
      proj: { v: 12.5, r: 40, hits: 1, life: 130, y: 170 } },
    s2: { name: '寒凰升', pose: 'up', s: 3, a: 12, r: 30, dmg: 110, chip: 18, hs: 0, bs: 20, push: 30, y0: 80, y1: 330, w: 80, lv: 'mid', kd: true, launch: 15, inv: 7,
      stop: 12, shake: 6, spark: 1.5, sfx: 'rise', meter: 15, rise: 17.5, riseX: 3, fire: true },
    su: { name: '寒霆天降', pose: 'sp', s: 26, a: 1, r: 38, dmg: 50, chip: 10, hs: 26, bs: 20, push: 22, lv: 'mid', stop: 7, shake: 9, spark: 1.8, sfx: 'super', meter: 0,
      proj: { v: 14, r: 90, hits: 5, every: 4, life: 110, y: 150, big: true } }
  }
};
var ORDER = ['yan', 'shuang'];

var STAGES = [
  { id: 'stage_city', name: '霓虹老街', fx: 'rain', floor: 790, reflect: 0.22, tint: '#1b1030' },
  { id: 'stage_temple', name: '落樱古寺', fx: 'petals', floor: 790, reflect: 0.1, tint: '#3a1c10' }
];

var DIFFS = [
  { name: '简单', react: 24, block: 0.25, aa: 0.15, aggro: 0.35, combo: 0.2 },
  { name: '普通', react: 14, block: 0.55, aa: 0.45, aggro: 0.55, combo: 0.5 },
  { name: '困难', react: 7, block: 0.85, aa: 0.8, aggro: 0.7, combo: 0.85 }
];

module.exports = { BODY: BODY, HP: HP, METER_MAX: METER_MAX, STOCK: STOCK, NORMALS: NORMALS, CHARS: CHARS, ORDER: ORDER, STAGES: STAGES, DIFFS: DIFFS };
