const store = require('../../utils/store.js');
const MAXH = 280; // 柱子最高 rpx
const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
function whenOf(w) {
  const t = store.today();
  if (w.date === t) return '今天 ' + w.time;
  if (w.date === store.addDays(t, -1)) return '昨天 ' + w.time;
  const d = new Date(store.parseDate(w.date));
  return w.date.slice(5) + ' 周' + WEEK[d.getDay()] + ' ' + w.time;
}
Page({
  data: { mode: 'min', bars: [], sel: 0, avg: 0, avgPos: 0, tip: '', days: 0, totalMin: 0, totalKcal: 0, history: [], total: 0 },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    store.tick();
    this.load();
  },
  load() {
    this.week = store.weekStats();
    const all = store.getWorkouts();
    this.setData({
      days: this.week.trained, totalMin: this.week.totalMin, totalKcal: this.week.totalKcal, sel: this.week.todayIdx, total: all.length,
      history: all.slice(0, 30).map(w => Object.assign({}, w, { when: whenOf(w), group: w.type === '团课' || w.type === '计划' }))
    });
    this.draw();
  },
  draw() {
    const key = this.data.mode;
    const w = this.week.days;
    const max = Math.max(1, Math.max.apply(null, w.map(x => x[key])));
    const bars = w.map(x => ({ d: x.d, v: x[key], h: x[key] ? Math.max(16, Math.round(x[key] / max * MAXH)) : 12 }));
    const active = w.filter(x => x[key] > 0);
    const avg = active.length ? Math.round(active.reduce((a, x) => a + x[key], 0) / active.length) : 0;
    this.setData({ bars, avg, avgPos: Math.round(avg / max * MAXH) });
    this.updateTip();
  },
  updateTip() {
    const b = this.data.bars[this.data.sel];
    const unit = this.data.mode === 'min' ? '分钟' : '千卡';
    const isToday = this.data.sel === this.week.todayIdx;
    const name = isToday ? '今天' : '周' + b.d;
    let tip;
    if (this.data.sel > this.week.todayIdx) tip = name + '还没到，按计划安排训练吧';
    else if (!b.v) tip = name + (isToday ? '还没有训练记录，练完记得记一笔' : '休息日，适当休息有助于肌肉恢复');
    else tip = name + '训练 ' + b.v + ' ' + unit + (b.v >= this.data.avg ? '，高于日均，状态不错！' : '，比日均少一点，加油～');
    this.setData({ tip });
  },
  setMode(e) { this.setData({ mode: e.currentTarget.dataset.m }); this.draw(); },
  pickBar(e) { this.setData({ sel: Number(e.currentTarget.dataset.index) }); this.updateTip(); },
  addWorkout() { wx.navigateTo({ url: '/pages/workout-edit/workout-edit' }); },
  onItem(e) {
    const w = this.data.history.find(x => x.id === e.currentTarget.dataset.id);
    wx.showActionSheet({
      itemList: ['删除这条训练记录'],
      success: () => {
        wx.showModal({
          title: '删除记录',
          content: '删除「' + w.name + '」（' + w.when + '，' + w.min + ' 分钟）？打卡记录会保留',
          confirmText: '删除',
          confirmColor: '#ff3d6e',
          success: r => {
            if (!r.confirm) return;
            store.removeWorkout(w.id);
            this.load();
            wx.showToast({ title: '已删除', icon: 'none' });
          }
        });
      }
    });
  }
});
