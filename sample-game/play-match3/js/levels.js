// 关卡表：步数、出现几种水果、目标（收集 N 个某种水果）。星级：过关 1 星，分数到 star2 / star3 再加一星。
// 水果种类下标与 js/fruit.js 一致：0 苹果 1 橙子 2 葡萄 3 柠檬 4 青梨 5 蓝莓
var NAMES = ['苹果', '橙子', '葡萄', '柠檬', '青梨', '蓝莓'];
var L = [
  { moves: 20, colors: 5, goals: [[0, 20]] },
  { moves: 20, colors: 5, goals: [[1, 22], [2, 16]] },
  { moves: 22, colors: 5, goals: [[3, 26], [4, 26]] },
  { moves: 22, colors: 6, goals: [[0, 26], [2, 26]] },
  { moves: 24, colors: 6, goals: [[5, 36]] },
  { moves: 24, colors: 6, goals: [[1, 24], [3, 24], [4, 20]] },
  { moves: 25, colors: 6, goals: [[2, 42]] },
  { moves: 25, colors: 6, goals: [[0, 32], [5, 32]] },
  { moves: 26, colors: 6, goals: [[1, 26], [2, 26], [3, 26]] },
  { moves: 26, colors: 6, goals: [[3, 44], [1, 30]] },
  { moves: 28, colors: 6, goals: [[0, 24], [2, 24], [4, 24], [5, 24]] },
  { moves: 30, colors: 6, goals: [[0, 20], [1, 20], [2, 20], [3, 20], [4, 20], [5, 20]] }
];
// 分数线按步数定：自动试玩（每步挑消得最多的）实测一步平均 ~100 分（三连 30，连锁与特效翻倍），剩余步数结算时每步再给 60。
// 照这个，贪心打法能拿 2 星，3 星要多做特效、多打连锁
L.forEach(function (lv, i) {
  lv.no = i + 1;
  lv.star2 = lv.moves * 110;
  lv.star3 = lv.moves * 150;
});
module.exports = { list: L, NAMES: NAMES };
