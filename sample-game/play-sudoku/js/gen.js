// 出题：随机终盘 → 按中心对称成对挖洞，每挖一对用求解器数解（数到 2 就停），不唯一就填回去。
// 挖到目标提示数或者所有格子都试过就停。分帧做：step(ms) 每帧最多算 ms 毫秒，出题时画面照常走
// （困难题要试满 41 对，每次数解在 QuickJS 里几毫秒到几十毫秒，一口气做完会卡住一两秒）。
var solver = require('./solver.js');

var DIFFS = [
  { key: 'intro', name: '入门', clues: 46 },
  { key: 'easy', name: '简单', clues: 38 },
  { key: 'mid', name: '中等', clues: 32 },
  { key: 'hard', name: '困难', clues: 25 }
];

function Gen(diff) {
  this.diff = diff;
  this.target = DIFFS[diff].clues;
  this.sol = new Uint8Array(81);
  solver.fill(this.sol);
  this.grid = new Uint8Array(this.sol);
  this.clues = 81;
  // 40 对（i, 80-i）+ 正中间那一格，顺序打乱
  var pairs = [];
  for (var i = 0; i <= 40; i++) pairs.push(i);
  for (var k = pairs.length - 1; k > 0; k--) { var j = Math.floor(Math.random() * (k + 1)), t = pairs[k]; pairs[k] = pairs[j]; pairs[j] = t; }
  this.pairs = pairs; this.pos = 0; this.checks = 0; this.ms = 0;
  this.done = false;
}
Gen.prototype.progress = function () {
  return Math.min(1, Math.max(this.pos / this.pairs.length, (81 - this.clues) / (81 - this.target)));
};
Gen.prototype.step = function (budget) {
  var t0 = Date.now();
  while (!this.done && Date.now() - t0 < budget) {
    if (this.pos >= this.pairs.length || this.clues <= this.target) { this.done = true; break; }
    var a = this.pairs[this.pos++], z = 80 - a, va = this.grid[a], vz = this.grid[z];
    this.grid[a] = 0; this.grid[z] = 0;
    this.checks++;
    if (solver.count(this.grid, 2) !== 1) { this.grid[a] = va; this.grid[z] = vz; }
    else this.clues -= a === z ? 1 : 2;
  }
  this.ms += Date.now() - t0;
  return this.done;
};

Gen.DIFFS = DIFFS;
module.exports = Gen;
