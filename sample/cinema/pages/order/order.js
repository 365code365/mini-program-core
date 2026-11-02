// 订单详情 / 电子票：取票码与二维码、场次座位、费用明细、退票 / 改签 / 导航 / 联系影院 / 写影评 / 再次购票
const store = require('../../utils/store.js');
const ops = require('../../utils/ops.js');
const host = require('../../utils/host.js');
const { qrMatrix } = require('../../utils/qr.js');
Page({
  data: { t: null, qr: [] },
  onLoad(q) { this.id = q.id; },
  onShow() { this.load(); },
  load() {
    const t = store.ticket(this.id);
    this.setData({ t, qr: t && t.status === 'unused' ? qrMatrix(t.code) : [] });
  },
  onCopy() { host.copy(this.data.t.code); },
  onRefund() { ops.refund(this.data.t, () => this.load()); },
  onChange() { ops.change(this.data.t, () => this.load()); },
  onNav() { ops.nav(this.data.t); },
  onCall() { ops.call(this.data.t); },
  onReview() {
    const r = store.reviewOf(this.data.t.id);
    wx.navigateTo({ url: '/pages/review/review?mid=' + this.data.t.mid + '&tid=' + this.data.t.id + (r ? '&id=' + r.id : '') });
  },
  onRebuy() { wx.navigateTo({ url: '/pages/movie/movie?id=' + this.data.t.mid + '&buy=1' }); },
  onTickets() { wx.switchTab({ url: '/pages/tickets/tickets' }); }
});
