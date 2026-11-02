// 洗牌用 wx.getRandomValues 的密码学随机数（题目顺序、干扰项），拿不到时退回 Math.random
function bytes(n) {
  return new Promise((resolve) => {
    wx.getRandomValues({
      length: n,
      success: (res) => resolve(new Uint8Array(res.randomValues)),
      fail: () => {
        const a = new Uint8Array(n);
        for (let i = 0; i < n; i++) a[i] = Math.floor(Math.random() * 256);
        resolve(a);
      }
    });
  });
}
// 用一串随机字节做 Fisher–Yates（每步取 2 字节，偏差可忽略）
function shuffleWith(arr, r) {
  const a = arr.slice();
  let k = 0;
  for (let i = a.length - 1; i > 0; i--) {
    const v = (r[k++ % r.length] << 8) | r[k++ % r.length];
    const j = v % (i + 1);
    const t = a[i]; a[i] = a[j]; a[j] = t;
  }
  return a;
}
async function shuffle(arr) { return shuffleWith(arr, await bytes(Math.max(2, arr.length * 2))); }

module.exports = { bytes, shuffle, shuffleWith };
