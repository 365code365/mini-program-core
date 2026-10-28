// AI 美术（tools/gen-fighter-art.py 用豆包 Seedream 4.5 生成、抠图、装箱，放在 art/）：
//   yan.webp / shuang.webp  每个角色 16 个动作的图集；manifest.json 记每个动作的矩形与脚底锚点
//   stage_*.jpg 场景、cover.jpg 封面
// 图都转成 WebGL 贴图（gl2d.texture）。某张没加载出来时那一项是 null，画的地方退回代码画的剪影 / 渐变，游戏照常跑。
var D = require('./data.js');

function loadImage(src) {
  return new Promise(function (resolve) {
    var img = wx.createImage();
    img.onload = function () { resolve(img); };
    img.onerror = function () { console.warn('[art] 加载失败：' + src); resolve(null); };
    img.src = src;
  });
}

function manifest() {
  try { return JSON.parse(wx.getFileSystemManager().readFileSync('art/manifest.json', 'utf8')); }
  catch (e) { console.warn('[art] 没有 art/manifest.json'); return null; }
}

// 返回 { chars: { yan: { tex, poses, k, head }, … }, stages: { id: tex }, cover: tex }
// k = 图集像素 → 世界单位（站立身高 = D.BODY）
function load(g, progress) {
  var m = manifest() || { chars: {}, standH: 500 };
  var jobs = [], out = { chars: {}, stages: {}, cover: null }, done = 0, total;
  D.ORDER.forEach(function (id) {
    var c = m.chars[id];
    if (!c) return;
    jobs.push(loadImage('art/' + c.atlas).then(function (img) {
      if (img) out.chars[id] = { tex: g.texture(img), poses: c.poses, head: c.head, k: D.BODY / (m.standH * (c.scale || 1)) };
    }));
  });
  D.STAGES.forEach(function (s) {
    jobs.push(loadImage('art/' + s.id + '.jpg').then(function (img) { if (img) out.stages[s.id] = g.texture(img); }));
  });
  jobs.push(loadImage('art/cover.jpg').then(function (img) { if (img) out.cover = g.texture(img); }));
  total = jobs.length;
  jobs = jobs.map(function (p) { return p.then(function () { done++; if (progress) progress(done / total); }); });
  return Promise.all(jobs).then(function () { return out; });
}

module.exports = { load: load };
