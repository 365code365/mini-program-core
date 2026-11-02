// 数值表：塔、敌人、波次、关卡倍率。距离单位都是「格」，时间是秒。
// 塔每级一行：dmg 伤害、range 射程、rate 每秒几发；upg 是升到这一级的价钱（第 1 级是建造价 cost）。

var TOWERS = {
  arrow: { name: '箭塔', color: '#c98b4b', air: true, cost: 50, levels: [
    { dmg: 9, range: 2.6, rate: 2.2 }, { dmg: 15, range: 2.9, rate: 2.5, upg: 60 }, { dmg: 24, range: 3.2, rate: 2.8, upg: 100 }],
    desc: '射速快，能打飞行' },
  cannon: { name: '炮塔', color: '#5d6d7e', air: false, cost: 80, levels: [
    { dmg: 22, range: 2.4, rate: 0.8, splash: 1.0 }, { dmg: 38, range: 2.6, rate: 0.85, splash: 1.15, upg: 90 }, { dmg: 60, range: 2.8, rate: 0.9, splash: 1.3, upg: 140 }],
    desc: '范围伤害，打不到飞行' },
  ice: { name: '冰塔', color: '#4fc3f7', air: true, cost: 70, levels: [
    { dmg: 4, range: 2.2, rate: 1.2, splash: 0.8, slow: 0.35 }, { dmg: 7, range: 2.4, rate: 1.2, splash: 0.9, slow: 0.45, upg: 70 }, { dmg: 11, range: 2.6, rate: 1.3, splash: 1.0, slow: 0.55, upg: 110 }],
    desc: '减速一片，能打飞行' },
  tesla: { name: '电塔', color: '#ab47bc', air: true, cost: 110, levels: [
    { dmg: 18, range: 2.3, rate: 0.9, chain: 3 }, { dmg: 30, range: 2.5, rate: 1.0, chain: 4, upg: 120 }, { dmg: 46, range: 2.7, rate: 1.1, chain: 5, upg: 180 }],
    desc: '连锁闪电，无视护甲' }
};
var ORDER = ['arrow', 'cannon', 'ice', 'tesla'];

// r：画出来的半径（格）；armor：每次伤害减去的点数（至少留 25%，见 combat.dmg）
var ENEMIES = {
  grunt: { name: '步兵', hp: 40, speed: 1.0, armor: 0, gold: 6, r: 0.22, color: '#e57373' },
  runner: { name: '快兵', hp: 28, speed: 1.8, armor: 0, gold: 7, r: 0.18, color: '#ffb74d' },
  tank: { name: '甲兵', hp: 110, speed: 0.7, armor: 6, gold: 12, r: 0.27, color: '#8d6e63' },
  bat: { name: '飞龙', hp: 45, speed: 1.25, armor: 0, gold: 10, r: 0.22, color: '#7e57c2', air: true },
  chief: { name: '督军', hp: 450, speed: 0.6, armor: 8, gold: 60, r: 0.34, color: '#c62828', boss: true },
  boss: { name: '巨魔王', hp: 1500, speed: 0.5, armor: 10, gold: 150, r: 0.42, color: '#4a148c', boss: true }
};

// 每波若干组：[类型, 数量, 间隔秒]；组与组接着出
var WAVES = [
  [['grunt', 10, 0.9]],
  [['grunt', 8, 0.8], ['runner', 6, 0.6]],
  [['runner', 12, 0.5], ['grunt', 6, 0.8]],
  [['tank', 5, 1.4], ['grunt', 8, 0.7]],
  [['bat', 8, 0.9], ['chief', 1, 1]],
  [['tank', 8, 1.1], ['runner', 10, 0.45]],
  [['bat', 12, 0.7], ['grunt', 12, 0.5]],
  [['tank', 10, 0.9], ['bat', 8, 0.8]],
  [['runner', 20, 0.35], ['tank', 8, 0.9]],
  [['boss', 1, 1], ['bat', 10, 0.8], ['tank', 8, 0.9]]
];

// 血量倍率：关卡基础 × 每波递增
function hpScale(map, wave) { return map.hp * (1 + 0.12 * wave); }
function waveText(w) {
  return (WAVES[w] || []).map(function (g) { return ENEMIES[g[0]].name + '×' + g[1]; }).join('  ');
}
function hasAir(w) { return (WAVES[w] || []).some(function (g) { return ENEMIES[g[0]].air; }); }

module.exports = { TOWERS: TOWERS, ORDER: ORDER, ENEMIES: ENEMIES, WAVES: WAVES, hpScale: hpScale, waveText: waveText, hasAir: hasAir,
  START_GOLD: 180, LIVES: 20, SELL: 0.7 };
