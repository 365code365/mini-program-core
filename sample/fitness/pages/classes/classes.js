const data = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { days: [], dayIdx: 0, types: ['全部', '燃脂', '单车', '力量', '瑜伽', '普拉提'], type: '全部', list: [] },
  onLoad() { this.setData({ days: data.nextDays(7) }); },
  onShow() { this.refresh(); },
  refresh() {
    const { dayIdx, type, days } = this.data;
    const list = data.scheduleFor(dayIdx)
      .filter(c => type === '全部' || c.type === type)
      .map(c => Object.assign(c, { mine: store.isBooked(c.id, dayIdx), dateText: days[dayIdx].key }));
    this.setData({ list });
  },
  pickDay(e) { this.setData({ dayIdx: e.currentTarget.dataset.index }); this.refresh(); },
  pickType(e) { this.setData({ type: e.currentTarget.dataset.t }); this.refresh(); },
  book(e) {
    const c = this.data.list[e.currentTarget.dataset.index];
    if (c.mine) {
      wx.showModal({
        title: '取消预约', content: '开课前 2 小时可免费取消，确定取消「' + c.name + '」吗？',
        success: (r) => { if (r.confirm) { store.toggleBooking(c); this.refresh(); wx.showToast({ title: '已取消', icon: 'none' }); } }
      });
      return;
    }
    if (!c.left) {
      wx.showToast({ title: '已加入候补，有空位将通知你', icon: 'none' });
      return;
    }
    store.toggleBooking(c);
    this.refresh();
    wx.showToast({ title: '预约成功', icon: 'success' });
  },
  goCourse(e) {
    wx.navigateTo({ url: '/pages/course/course?id=' + e.currentTarget.dataset.id + '&day=' + this.data.dayIdx });
  }
});
