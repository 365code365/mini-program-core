// AI 美术（tools/gen-runner-art.py 用豆包 Seedream 生成、加工好放在 art/）：材质、楼房立面、车厢、金币、远景、封面。
// 全部可选：manifest 没有、哪张没加载出来（包里没有、解码失败、3 秒没回来），对应的地方退回代码画的颜色，游戏照常跑。
function manifest() {
  try { return JSON.parse(wx.getFileSystemManager().readFileSync('art/manifest.json', 'utf8')); }
  catch (e) { console.warn('[art] 没有 art/manifest.json：用纯色'); return null; }
}
function loadImage(src) {
  return new Promise(function (resolve) {
    var img = wx.createImage(), done = false;
    function fin(v) { if (!done) { done = true; resolve(v); } }
    img.onload = function () { fin(img); };
    img.onerror = function () { console.warn('[art] 加载失败：' + src); fin(null); };
    setTimeout(function () { fin(null); }, 3000);
    img.src = src;
  });
}
// 返回 { manifest, img: { 名字 → Image } }
function load() {
  var m = manifest();
  if (!m) return Promise.resolve({ manifest: null, img: {} });
  var out = { manifest: m, img: {} };
  return Promise.all((m.files || []).map(function (n) {
    return loadImage('art/' + n + '.jpg').then(function (img) { if (img) out.img[n] = img; });
  })).then(function () { return out; });
}
module.exports = { load: load };
