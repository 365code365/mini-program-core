const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const flows = require('../../utils/flows.js');
// 按钮文案：自己的预约状态优先，其次是这节课本身（开课了 / 满员）
const BTN = { booked: '已预约', wait: '候补中', done: '已上课', miss: '已爽约', open: '预约', full: '排队', started: '已开课', ended: '已结束' };
Page({
  data: { days: [], dayIdx: 0, types: ['全部', '燃脂', '单车', '力量', '瑜伽', '普拉提'], type: '全部', list: [], storeName: '' },
  onLoad() { this.setData({ days: data.nextDays(7) }); },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    store.tick();
    this.setData({ storeName: store.currentStore().short });
    this.refresh();
  },
  refresh() {
    const { dayIdx, type } = this.data;
    const now = Date.now();
    const list = data.scheduleFor(dayIdx).filter(c => type === '全部' || c.type === type).map(c => {
      const b = store.bookingOf(c.id, c.date);
      // 课表里的 booked 是别人约的：自己约上了要算进去
      const mine = b && (b.status === 'booked' || b.status === 'done') ? 1 : 0;
      const left = c.left - mine;
      const started = now >= store.startOf(c);
      const state = b ? b.status : (started ? (now >= store.endOf(c) ? 'ended' : 'started') : (left > 0 ? 'open' : 'full'));
      return Object.assign(c, { booked: c.booked + mine, left, pct: Math.round((c.booked + mine) / c.cap * 100), state, btn: BTN[state], bid: b ? b.bid : '' });
    });
    this.setData({ list });
  },
  pickDay(e) { this.setData({ dayIdx: Number(e.currentTarget.dataset.index) }); this.refresh(); },
  pickType(e) { this.setData({ type: e.currentTarget.dataset.t }); this.refresh(); },
  book(e) {
    const c = this.data.list[Number(e.currentTarget.dataset.index)];
    if (c.state === 'booked' || c.state === 'wait') return flows.cancel(store.findBooking(c.bid)).then(() => this.refresh());
    if (c.state === 'done' || c.state === 'miss') return this.openCourse(c.id);
    if (c.state === 'started' || c.state === 'ended') return wx.showToast({ title: c.state === 'started' ? '已开课，下次早点来' : '这节课已经结束了', icon: 'none' });
    // 课表里的 left 不含自己：交给 flows 之前换回原始的余位
    flows.book(data.scheduleFor(this.data.dayIdx).find(x => x.id === c.id)).then(() => this.refresh());
  },
  goCourse(e) { this.openCourse(e.currentTarget.dataset.id); },
  openCourse(id) { wx.navigateTo({ url: '/pages/course/course?id=' + id + '&day=' + this.data.dayIdx }); }
});
