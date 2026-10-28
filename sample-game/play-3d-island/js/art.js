// AI 美术资源（tools/gen-island-art.py 用豆包 Seedream 生成、加工好的图集，放在 art/）：
// textures.jpg（材质 4×4）、icons.webp（图标 8×8）、portraits.webp（宠物 / 建筑立绘）、cover.jpg（封面）、manifest.json（各图在图集里的位置）。
// 全部是可选的：哪张没加载出来（包里没有、解码失败），对应的地方退回代码画的版本，游戏照常跑。
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
  catch (e) { console.warn('[art] 没有 art/manifest.json：用代码画的美术'); return null; }
}
// progress(k)：0~1，封面上的进度条用
function load(progress) {
  var m = manifest();
  if (!m) return Promise.resolve({ manifest: null });
  var names = ['textures.jpg', 'icons.webp', 'portraits.webp'], done = 0, out = { manifest: m };
  return Promise.all(names.map(function (n) {
    return loadImage('art/' + n).then(function (img) { out[n.split('.')[0]] = img; done++; if (progress) progress(done / names.length); });
  })).then(function () { return out; });
}
module.exports = { load: load, loadImage: loadImage, manifest: manifest };
