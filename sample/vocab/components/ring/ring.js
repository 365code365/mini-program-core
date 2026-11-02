// 进度环：canvas type="2d" 自绘（引擎不支持 conic-gradient），中间的数字用普通 text 叠在上面
Component({
  properties: {
    value: { type: Number, value: 0, observer: 'draw' },
    total: { type: Number, value: 1, observer: 'draw' },
    size: { type: Number, value: 120 },
    color: { type: String, value: '#4f6bed' },
    label: { type: String, value: '今日新词' }
  },
  lifetimes: {
    ready() {
      this.createSelectorQuery().select('#ring').fields({ node: true, size: true }).exec((res) => {
        const r = res && res[0];
        if (!r || !r.node) return;
        const dpr = wx.getWindowInfo ? wx.getWindowInfo().pixelRatio : 2;
        r.node.width = r.width * dpr;
        r.node.height = r.height * dpr;
        this._ctx = r.node.getContext('2d');
        this._ctx.scale(dpr, dpr);
        this._w = r.width;
        this.draw();
      });
    }
  },
  methods: {
    draw() {
      const ctx = this._ctx;
      if (!ctx) return;
      const s = this._w, c = s / 2, lw = s * 0.09, rad = c - lw / 2 - 1;
      const total = Math.max(1, this.data.total);
      const p = Math.max(0, Math.min(1, this.data.value / total));
      ctx.clearRect(0, 0, s, s);
      ctx.lineWidth = lw;
      ctx.lineCap = 'round';
      ctx.strokeStyle = '#e6eaf7';
      ctx.beginPath();
      ctx.arc(c, c, rad, 0, Math.PI * 2);
      ctx.stroke();
      if (p > 0) {
        const g = ctx.createLinearGradient(0, 0, s, s);
        g.addColorStop(0, '#7b8cff');
        g.addColorStop(1, this.data.color);
        ctx.strokeStyle = g;
        ctx.beginPath();
        ctx.arc(c, c, rad, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * p);
        ctx.stroke();
      }
    }
  }
});
