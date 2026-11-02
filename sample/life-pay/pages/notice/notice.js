// 公告：按城市过滤（只有杭州有演示公告）；看过的记进 lp_notice_read，首页公告条据此显示「新」
const data = require('../../utils/data.js');
const host = require('../../utils/host.js');
Page({
  data: { list: [], city: '' },
  onLoad() {
    const city = wx.getStorageSync('lp_city') || '杭州市';
    const list = data.notices.filter(n => n.city === city);
    this.setData({ city, list });
    wx.setStorageSync('lp_notice_read', list.map(n => n.id));
  },
  call(e) { host.call(String(e.currentTarget.dataset.p)); }
});
