const app = getApp();
Page({
  data: { log: [], imgInfo: '' },
  onReady() {
    this.createSelectorQuery().select('#cv').fields({ node: true, size: true }).exec((res) => {
      const r = res && res[0];
      if (!r || !r.node) return;
      const dpr = wx.getSystemInfoSync().pixelRatio;
      const c = r.node;
      c.width = r.width * dpr; c.height = r.height * dpr;
      const ctx = c.getContext('2d');
      ctx.scale(dpr, dpr);
      this.cv = { c, ctx, w: r.width, h: r.height };
      const img = c.createImage();
      img.onload = () => { this.img = img; app.note(this, `createImage onload ${img.width}×${img.height}`); this.draw(); };
      img.onerror = (e) => app.note(this, `createImage onerror ${e.errMsg}`);
      img.src = '/images/photo.jpg';
    });
  },
  draw() {
    const { ctx, w, h } = this.cv;
    ctx.clearRect(0, 0, w, h);
    // drawImage 9 参数：只裁中间那颗太阳
    if (this.img) ctx.drawImage(this.img, 80, 40, 80, 80, 12, 12, 80, 80);
    // 3 参数：按原图尺寸（240×160）画一半宽的缩略
    if (this.img) ctx.drawImage(this.img, 104, 12, 120, 80);
    ctx.beginPath();
    ctx.roundRect(236, 12, 80, 80, [16, 4]);
    ctx.fillStyle = '#07c160'; ctx.fill();
    // evenodd：中间挖空
    ctx.beginPath();
    ctx.rect(12, 104, 90, 60); ctx.rect(32, 119, 50, 30);
    ctx.fillStyle = '#fa9d3b'; ctx.fill('evenodd');
    ctx.font = '16px sans-serif'; ctx.fillStyle = '#1f2329';
    ctx.fillText('maxWidth 压缩到 120px 宽的一行很长的字', 112, 128, 120);
    ctx.strokeStyle = '#e64340'; ctx.lineWidth = 1;
    ctx.font = 'bold 24px sans-serif';
    ctx.strokeText('描边', 250, 150);
  },
  onCvTap(e) {
    if (!this.cv) return;
    const t = e.touches && e.touches[0] || e.changedTouches && e.changedTouches[0];
    if (!t) return;
    const { ctx } = this.cv;
    ctx.beginPath(); ctx.roundRect(236, 12, 80, 80, [16, 4]);
    const dpr = wx.getSystemInfoSync().pixelRatio;
    const hit = ctx.isPointInPath(t.x * dpr, t.y * dpr);
    app.note(this, `canvas 触点 (${Math.round(t.x)}, ${Math.round(t.y)}) 在绿色圆角矩形里：${hit}`);
  },
  onImgLoad(e) { this.setData({ imgInfo: `${e.detail.width}×${e.detail.height}` }); app.note(this, `image bindload ${e.detail.width}×${e.detail.height}`); },
  onImgErr(e) { app.note(this, `image binderror ${e.detail.errMsg}`); },
  onCover() { app.note(this, '点到视频上的 cover-view 控件'); }
});
