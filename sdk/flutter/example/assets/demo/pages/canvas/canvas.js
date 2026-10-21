// 画布宽度跟着屏幕走：.cv 是 100% 宽（卡片左右各 20rpx 外边距 + 24rpx 内边距）、360rpx 高
function cssSize() {
  const w = wx.getSystemInfoSync().windowWidth;
  const rpx = w / 750;
  return { width: w - (20 + 24) * 2 * rpx, height: 360 * rpx };
}

Page({
  data: {},
  onReady() {
    this.drawPoster();
    this.initChart();
  },
  onUnload() {
    clearTimeout(this.animTimer);
  },

  // ── 旧接口 ──
  drawPoster() {
    const { width, height } = cssSize();
    const ctx = wx.createCanvasContext('poster');
    ctx.setFillStyle('#f5f7fa');
    ctx.fillRect(0, 0, width, height);
    // 左边一张图
    ctx.drawImage('/images/coffee.jpg', 12, 12, 120, 80);
    ctx.setStrokeStyle('#07c160');
    ctx.setLineWidth(2);
    ctx.strokeRect(12, 12, 120, 80);
    // 文字
    ctx.setFillStyle('#222222');
    ctx.setFontSize(16);
    ctx.fillText('手冲咖啡', 146, 34);
    ctx.setFillStyle('#e64340');
    ctx.setFontSize(20);
    ctx.fillText('¥ 28.00', 146, 64);
    ctx.setFillStyle('#999999');
    ctx.setFontSize(11);
    ctx.fillText('canvas-id="poster"', 146, 86);
    // 圆与圆弧
    const cy = height - 50;
    ctx.beginPath();
    ctx.arc(46, cy, 30, 0, 2 * Math.PI);
    ctx.setFillStyle('#10aeff');
    ctx.fill();
    ctx.beginPath();
    ctx.arc(120, cy, 30, 0, 1.5 * Math.PI);
    ctx.setStrokeStyle('#fa9d3b');
    ctx.setLineWidth(6);
    ctx.setLineCap('round');
    ctx.stroke();
    // 贝塞尔
    ctx.beginPath();
    ctx.moveTo(170, cy + 20);
    ctx.bezierCurveTo(200, cy - 50, 240, cy + 50, 280, cy - 20);
    ctx.setStrokeStyle('#8a5cf5');
    ctx.setLineWidth(3);
    ctx.stroke();
    // 旋转的小方块（save / translate / rotate / restore）
    ctx.save();
    ctx.translate(width - 40, cy);
    ctx.rotate(Math.PI / 6);
    ctx.setFillStyle('#07c160');
    ctx.fillRect(-18, -18, 36, 36);
    ctx.restore();
    ctx.draw();
  },

  // ── 新接口 ──
  initChart() {
    wx.createSelectorQuery().select('#chart').fields({ node: true, size: true }).exec((res) => {
      const r = res && res[0];
      if (!r || !r.node) {
        console.warn('拿不到 canvas 节点');
        return;
      }
      this.canvas = r.node;
      this.size = { width: r.width, height: r.height };
      this.values = [42, 68, 30, 88, 55, 73];
      this.drawChart(1);
    });
  },
  resetCanvas() {
    const dpr = wx.getSystemInfoSync().pixelRatio;
    const c = this.canvas;
    c.width = Math.round(this.size.width * dpr);
    c.height = Math.round(this.size.height * dpr);
    const ctx = c.getContext('2d');
    ctx.scale(dpr, dpr);
    return ctx;
  },
  drawChart(progress) {
    if (!this.canvas) return;
    const ctx = this.resetCanvas();
    const { width, height } = this.size;
    const pad = 24;
    const n = this.values.length;
    const slot = (width - pad * 2) / n;
    const base = height - 28;
    // 网格
    ctx.strokeStyle = '#eeeeee';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
      const y = base - (base - 20) * i / 4;
      ctx.beginPath();
      ctx.moveTo(pad, y);
      ctx.lineTo(width - pad, y);
      ctx.stroke();
    }
    const colors = ['#07c160', '#10aeff', '#fa9d3b', '#e64340', '#8a5cf5', '#576b95'];
    ctx.textAlign = 'center';
    ctx.font = '11px sans-serif';
    for (let i = 0; i < n; i++) {
      const h = (base - 20) * this.values[i] / 100 * progress;
      const x = pad + slot * i + slot * 0.2;
      ctx.fillStyle = colors[i % colors.length];
      ctx.fillRect(x, base - h, slot * 0.6, h);
      ctx.fillStyle = '#666666';
      ctx.fillText(String(Math.round(this.values[i] * progress)), x + slot * 0.3, base - h - 10);
      ctx.fillText('周' + '一二三四五六'[i], x + slot * 0.3, height - 10);
    }
    // 折线
    ctx.strokeStyle = '#222222';
    ctx.lineWidth = 2;
    ctx.beginPath();
    for (let i = 0; i < n; i++) {
      const x = pad + slot * i + slot * 0.5;
      const y = base - (base - 20) * this.values[i] / 100 * progress;
      if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y);
    }
    ctx.stroke();
  },
  shuffle() {
    this.values = this.values.map(() => 20 + Math.round(Math.random() * 75));
    this.drawChart(1);
  },
  animate() {
    // 每帧改 width 清空再画（2d 画布的指令是追加的，不清空会越攒越多）
    clearTimeout(this.animTimer);
    let frame = 0;
    const total = 30;
    const step = () => {
      frame++;
      const t = frame / total;
      this.drawChart(1 - Math.pow(1 - t, 3));
      if (frame < total) this.animTimer = setTimeout(step, 16);
    };
    step();
  }
});
