// 可注入种子的随机数发生器（线性同余 LCG，Numerical Recipes 参数 a=1664525, c=1013904223, m=2^32）。
// 同一个种子永远出同一串数：测试用 ?seed=7 打开游戏页，就能断言滑动之后棋盘上每一格的具体数值。
// 不用 Math.imul：a*s 最大约 7.1e15，仍在 2^53 以内，普通乘法是精确的。
const M = 4294967296

function create(seed) {
  let s = Math.floor(Math.abs(Number(seed))) % M
  if (!s) s = 1
  return {
    // [0, 1) 的浮点数
    next() {
      s = (1664525 * s + 1013904223) % M
      return s / M
    },
    // [0, n) 的整数
    int(n) {
      return Math.floor(this.next() * n)
    },
    state() {
      return s
    }
  }
}

module.exports = { create }
