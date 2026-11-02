// 水果造型：整果（在自身坐标里画，外面 save / translate / rotate 转起来）、切开的半个（平的切面在 x 轴上、果肉朝 +y），炸弹。
// 精灵（R.sprite）不跟着旋转，水果要转，所以直接用形状画；一个整果 3~6 个形状、约 150~250 个顶点。
var PI = Math.PI;

// juice：果汁粒子 / 果渍的颜色（0~1），score：切开得分
var TYPES = [
  { name: '西瓜', r: 34, rind: '#2e7d32', stripe: '#66bb6a', flesh: '#ef5350', inner: '#e53935', seed: '#212121', juice: [0.9, 0.15, 0.2] },
  { name: '橙子', r: 25, rind: '#fb8c00', stripe: '#ffa726', flesh: '#ffb74d', inner: '#ffcc80', seed: '#fff3e0', juice: [1, 0.6, 0.1] },
  { name: '苹果', r: 25, rind: '#d32f2f', stripe: '#ef5350', flesh: '#fff3e0', inner: '#ffe0b2', seed: '#5d4037', juice: [1, 0.95, 0.75] },
  { name: '柠檬', r: 23, rind: '#fdd835', stripe: '#fff176', flesh: '#fff59d', inner: '#fffde7', seed: '#f9a825', juice: [1, 0.95, 0.3] },
  { name: '猕猴桃', r: 22, rind: '#8d6e63', stripe: '#6d4c41', flesh: '#9ccc65', inner: '#f1f8e9', seed: '#1b1b1b', juice: [0.55, 0.8, 0.25] },
  { name: '桃子', r: 25, rind: '#f48fb1', stripe: '#f8bbd0', flesh: '#ffcc80', inner: '#ffe0b2', seed: '#8d4f2b', juice: [1, 0.7, 0.45] }
];
var BOMB_R = 24;
var RED_GLOW = [1, 0.15, 0.1, 0.35], SPARK = [1, 0.85, 0.3, 1];   // stamp 染色：常量数组，帧里不新建

function make(R) {
  function highlight(r) { var a = R.getAlpha(); R.setAlpha(a * 0.3); R.ellipse(-r * 0.38, -r * 0.4, r * 0.32, r * 0.2, -0.6, '#ffffff'); R.setAlpha(a); }
  function leaf(x, y, r) { R.rect(x - 1.2, y - r * 0.28, 2.4, r * 0.3, '#5d4037'); R.ellipse(x + r * 0.2, y - r * 0.2, r * 0.22, r * 0.1, -0.5, '#43a047'); }

  function whole(k) {
    var T = TYPES[k], r = T.r;
    if (k === 0) {                                        // 西瓜：深绿底 + 三道浅绿条纹
      R.circle(0, 0, r, T.rind);
      R.ellipse(0, 0, r * 0.13, r * 0.95, 0, T.stripe);
      R.ellipse(-r * 0.55, 0, r * 0.11, r * 0.78, 0, T.stripe);
      R.ellipse(r * 0.55, 0, r * 0.11, r * 0.78, 0, T.stripe);
    } else if (k === 3) {                                 // 柠檬：两头尖
      R.ellipse(0, 0, r, r * 0.8, 0, T.rind);
      R.circle(-r * 0.95, 0, r * 0.16, T.rind); R.circle(r * 0.95, 0, r * 0.16, T.rind);
    } else if (k === 4) {                                 // 猕猴桃：棕色椭圆 + 绒点
      R.ellipse(0, 0, r, r * 0.86, 0, T.rind);
      R.circle(-r * 0.4, r * 0.2, 1.6, T.stripe); R.circle(r * 0.3, -r * 0.3, 1.6, T.stripe); R.circle(r * 0.45, r * 0.35, 1.6, T.stripe);
    } else {
      R.circle(0, 0, r, T.rind);
      if (k === 5) R.ellipse(r * 0.18, 0, r * 0.06, r * 0.8, 0.15, T.stripe);   // 桃子的缝
      leaf(0, -r * 0.88, r);
    }
    highlight(r);
  }
  // 半个：rind 半圆 + 果肉半圆 + 内芯 / 籽，切面在 y=0
  function half(k) {
    var T = TYPES[k], r = T.r, ry = k === 3 ? 0.8 : k === 4 ? 0.86 : 1;
    R.ellipse(0, 0, r, r * ry, 0, T.rind, 0, PI);
    R.ellipse(0, 0, r * 0.86, r * ry * 0.84, 0, T.flesh, 0, PI);
    if (k === 0) {
      R.ellipse(0, 0, r * 0.7, r * 0.66, 0, T.inner, 0, PI);
      R.ellipse(-r * 0.35, r * 0.3, 2, 3.2, 0.3, T.seed); R.ellipse(0, r * 0.42, 2, 3.2, 0, T.seed); R.ellipse(r * 0.35, r * 0.3, 2, 3.2, -0.3, T.seed);
    } else if (k === 1 || k === 3) {                      // 柑橘：浅色瓣心
      R.ellipse(0, 0, r * 0.3, r * ry * 0.3, 0, T.inner, 0, PI);
    } else if (k === 4) {                                 // 猕猴桃：白芯 + 一圈黑籽
      R.ellipse(0, 0, r * 0.28, r * 0.24, 0, T.inner, 0, PI);
      R.circle(-r * 0.45, r * 0.12, 1.4, T.seed); R.circle(-r * 0.3, r * 0.38, 1.4, T.seed); R.circle(0, r * 0.48, 1.4, T.seed);
      R.circle(r * 0.3, r * 0.38, 1.4, T.seed); R.circle(r * 0.45, r * 0.12, 1.4, T.seed);
    } else if (k === 2) {                                 // 苹果：两粒籽
      R.ellipse(-r * 0.15, r * 0.3, 1.8, 3, 0.3, T.seed); R.ellipse(r * 0.15, r * 0.3, 1.8, 3, -0.3, T.seed);
    } else R.ellipse(0, 0, r * 0.32, r * 0.32, 0, T.seed, 0, PI);   // 桃核
    R.rect(-r * 0.86, 0, r * 1.72, 1.5, T.inner);         // 切面亮边
  }
  function bomb(t, sparkTex) {
    var r = BOMB_R;
    R.stamp(sparkTex, 0, 0, 2.6 + Math.sin(t * 10) * 0.25, RED_GLOW, true);
    R.circle(0, 0, r, '#263238');
    R.rect(-r * 0.3, -r - 4, r * 0.6, 7, '#546e7a');
    R.rect(-1.5, -r - 13, 3, 10, '#8d6e63');
    var a = R.getAlpha(); R.setAlpha(a * 0.35); R.ellipse(-r * 0.35, -r * 0.35, r * 0.3, r * 0.18, -0.6, '#ffffff'); R.setAlpha(a);
    R.text('炸', 0, 2, 16, '#ff5252', 'center', 'bold');
    R.stamp(sparkTex, 0, -r - 14, 0.9 + Math.sin(t * 37) * 0.3, SPARK, true);
  }
  return { whole: whole, half: half, bomb: bomb };
}

module.exports = { TYPES: TYPES, BOMB_R: BOMB_R, make: make };
