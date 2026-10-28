// 文字贴图：WebGL 里画字 = 在离屏 2D 画布上写好字，传成纹理，再当矩形贴上去。
// 每个 key 一块画布 + 一个纹理；文字 / 样式没变就不重画不重传（得分、最高分一局里才变几十次）。
// 画布按设备像素比放大，贴回逻辑尺寸，字边才清楚。
var FONT = 'sans-serif';

function make(gl, dpr) {
  var cache = {};
  function get(key, text, size, color, weight, bg) {
    var sig = text + '|' + size + '|' + color + '|' + (weight || '') + '|' + (bg ? bg.join(',') : '');
    var e = cache[key];
    if (e && e.sig === sig) return e;
    if (!e) {
      e = cache[key] = { cv: wx.createCanvas(), tex: gl.createTexture() };
      gl.bindTexture(gl.TEXTURE_2D, e.tex);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);   // 非 2 的幂尺寸在 WebGL 1 里必须 CLAMP + 不用 mipmap
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    }
    e.sig = sig;
    var c = e.cv.getContext('2d'), font = (weight || 'normal') + ' ' + Math.round(size * dpr) + 'px ' + FONT;
    c.font = font;
    var padX = bg ? bg[2] : 2, padY = bg ? bg[3] : 2;
    var w = Math.ceil(c.measureText(String(text)).width / dpr) + padX * 2, h = Math.ceil(size * 1.3) + padY * 2;
    e.cv.width = Math.max(1, Math.ceil(w * dpr)); e.cv.height = Math.max(1, Math.ceil(h * dpr));
    c = e.cv.getContext('2d');                         // 改尺寸会清空状态：重设
    c.clearRect(0, 0, e.cv.width, e.cv.height);
    if (bg) {                                          // 胶囊底（按钮）：bg = [颜色, _, padX, padY]
      var r = e.cv.height / 2;
      c.fillStyle = bg[0];
      c.beginPath(); c.moveTo(r, 0); c.lineTo(e.cv.width - r, 0); c.arc(e.cv.width - r, r, r, -Math.PI / 2, Math.PI / 2);
      c.lineTo(r, e.cv.height); c.arc(r, r, r, Math.PI / 2, Math.PI * 1.5); c.closePath(); c.fill();
    }
    c.font = font; c.fillStyle = color; c.textAlign = 'center'; c.textBaseline = 'middle';
    c.fillText(String(text), e.cv.width / 2, e.cv.height / 2 + dpr);
    gl.bindTexture(gl.TEXTURE_2D, e.tex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, e.cv);
    e.w = w; e.h = h;
    return e;
  }
  return { get: get };
}

module.exports = { make: make };
