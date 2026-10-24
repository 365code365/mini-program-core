const data = require('../../utils/data.js');
const MAXH = 280; // 柱子最高 rpx

Page({
  data: { mode: 'min', bars: [], sel: 5, avg: 0, avgPos: 0, tip: '', days: 0, totalMin: 0, totalKcal: 0, history: data.history },
  onLoad() {
    const w = data.weekRecords;
    this.setData({
      days: w.filter(x => x.min > 0).length,
      totalMin: w.reduce((a, x) => a + x.min, 0),
      totalKcal: w.reduce((a, x) => a + x.kcal, 0)
    });
    this.draw();
  },
  draw() {
    const key = this.data.mode;
    const w = data.weekRecords;
    const max = Math.max.apply(null, w.map(x => x[key]));
    const bars = w.map(x => ({ d: x.d, v: x[key], h: x[key] ? Math.max(16, Math.round(x[key] / max * MAXH)) : 12 }));
    const active = w.filter(x => x[key] > 0);
    const avg = Math.round(active.reduce((a, x) => a + x[key], 0) / active.length);
    // 柱子底部到 chart 底边有日期标签的 -48rpx 外溢，平均线按柱子同一坐标算
    this.setData({ bars, avg, avgPos: Math.round(avg / max * MAXH) });
    this.updateTip();
  },
  updateTip() {
    const b = this.data.bars[this.data.sel];
    const unit = this.data.mode === 'min' ? '分钟' : '千卡';
    this.setData({ tip: '周' + b.d + (b.v ? ' 训练 ' + b.v + ' ' + unit + (b.v >= this.data.avg ? '，高于日均，状态不错！' : '，比日均少一点，加油～') : ' 休息日，适当休息有助于肌肉恢复') });
  },
  setMode(e) { this.setData({ mode: e.currentTarget.dataset.m }); this.draw(); },
  pickBar(e) { this.setData({ sel: e.currentTarget.dataset.index }); this.updateTip(); }
});
