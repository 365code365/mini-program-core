const store = require('../../utils/store.js');

const WAY = { wx: '微信支付', bank: '建设银行储蓄卡', card: '易加油电子油卡' };

Page({
  data: { r: {}, wayText: '', timeText: '' },
  onLoad(q) { this.id = q.id; },
  onShow() {
    const r = store.recordById(this.id);
    if (r) this.setData({ r, wayText: WAY[r.way] || r.way, timeText: store.fmt(r.createdAt, true) });
  },
  invoice() { wx.navigateTo({ url: '/pages/invoice/invoice?id=' + this.id }); },
  done() { wx.switchTab({ url: '/pages/records/records' }); }
});
