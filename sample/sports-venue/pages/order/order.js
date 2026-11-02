const D = require('../../utils/data.js');
const F = require('../../utils/fmt.js');
const store = require('../../utils/store.js');
const ORD = require('../../utils/order.js');
const HEAD = {
  paid: '请在开场前 10 分钟到场，凭入场码核验入场',
  used: '已核验入场，祝你打得尽兴',
  ended: '本场已结束，欢迎再来',
  cancelled: '订单已取消，款项 / 次数已原路退回'
};
Page({
  data: { o: null, v: null, head: '', created: '', left: '' },
  onLoad(q) { this.id = q.id; },
  onShow() { this.load(); },
  load() {
    const raw = store.getOrder(this.id);
    if (!raw) { this.setData({ o: null }); return; }
    const o = ORD.view(raw);
    const ms = ORD.startTs(raw) - Date.now();
    let left = '';
    if (o.st === 'paid') left = ms <= 0 ? '已开场' : (ms < 86400000 ? '距开场 ' + Math.floor(ms / 3600000) + ' 小时 ' + Math.floor(ms / 60000) % 60 + ' 分' : '距开场 ' + Math.floor(ms / 86400000) + ' 天');
    this.setData({ o, v: D.getVenue(raw.venueId), head: HEAD[o.st], created: F.dateTime(raw.createdAt), left });
  },
  toPass() { wx.navigateTo({ url: '/pages/pass/pass?id=' + this.id }); },
  toTeam() { wx.navigateTo({ url: '/pages/team-create/team-create?order=' + this.id }); },
  again() { wx.navigateTo({ url: '/pages/booking/booking?id=' + this.data.o.venueId }); },
  call() {
    const phone = this.data.v.phone;
    wx.makePhoneCall({
      phoneNumber: phone.replace('-', ''),
      fail: () => wx.setClipboardData({ data: phone, success: () => wx.showToast({ title: '已复制场馆电话 ' + phone, icon: 'none' }) })
    });
  },
  nav() {
    const v = this.data.v;
    wx.openLocation({
      latitude: v.lat, longitude: v.lng, name: v.name, address: v.addr,
      fail: () => wx.setClipboardData({ data: v.addr, success: () => wx.showToast({ title: '无法打开地图，地址已复制', icon: 'none' }) })
    });
  },
  cancel() {
    const reasons = ['临时有事去不了', '人没约齐', '订错时间 / 场地', '天气原因', '其他'];
    wx.showActionSheet({
      alertText: '开场前 2 小时以上可免费取消',
      itemList: reasons,
      success: (r) => {
        const raw = store.getOrder(this.id);
        raw.cancelReason = reasons[r.tapIndex];
        const res = ORD.cancel(raw);
        if (!res.ok) { wx.showModal({ title: '无法取消', content: res.msg, showCancel: false }); return; }
        this.load();
        wx.showModal({ title: '已取消', content: res.msg, showCancel: false });
      }
    });
  },
  copy() { wx.setClipboardData({ data: this.id }); },
  toOrders() { wx.navigateBack({ fail: () => wx.redirectTo({ url: '/pages/orders/orders' }) }); }
});
