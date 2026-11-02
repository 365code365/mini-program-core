// 课程表：周视图 7 列 × 12 节，display:grid 摆放（grid-row 跨节次）；swiper 左右滑切周（current 受控）。
// 列宽按屏宽算（wx.getWindowInfo），只渲染当前周与相邻两周，其余 swiper-item 留空
const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { weeks: [], cur: 0, thisWeek: 1, colW: 46, rowH: 50, sections: D.SECTIONS, detail: null, todayDay: 1 },
  onLoad() {
    const win = wx.getWindowInfo ? wx.getWindowInfo() : wx.getSystemInfoSync();
    const colW = Math.floor((win.windowWidth - 34) / 7);
    const w = D.weekOf();
    this.setData({ colW, thisWeek: w, cur: w - 1, todayDay: new Date().getDay() || 7 });
    this.build(w - 1);
  },
  onShow() { store.syncTab(this, 1); },
  build(cur) {
    const weeks = [];
    for (let i = 0; i < D.WEEKS; i++) {
      const w = i + 1, near = Math.abs(i - cur) <= 1;
      weeks.push({
        w, near, dates: near ? D.datesOf(w) : [],
        courses: near ? D.COURSES.filter(c => D.inWeek(c, w)) : []
      });
    }
    this.setData({ weeks, cur });
  },
  onSwiper(e) {
    if (e.detail.source && e.detail.source !== 'touch' && e.detail.source !== 'autoplay' && e.detail.source !== '') return;
    this.build(e.detail.current);
  },
  prev() { if (this.data.cur > 0) this.build(this.data.cur - 1); },
  next() { if (this.data.cur < D.WEEKS - 1) this.build(this.data.cur + 1); },
  backToday() { this.build(this.data.thisWeek - 1); },
  open(e) {
    const c = D.COURSES.find(x => x.id === e.currentTarget.dataset.id);
    this.setData({ detail: Object.assign({}, c, { wd: D.WD[c.day], sec: D.sectionText(c.start, c.len) }) });
  },
  close() { this.setData({ detail: null }); }
});
