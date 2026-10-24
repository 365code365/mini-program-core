const ALL = [
  { id: 'c1', amount: 15, cond: '满 40 可用', name: '快车立减券', date: '2026.10.24 - 2026.11.07', scope: '快车' },
  { id: 'c2', amount: 8, cond: '无门槛', name: '新客专享券', date: '2026.10.20 - 2026.11.20', scope: '全部车型' },
  { id: 'c3', amount: 20, cond: '满 80 可用', name: '专车商务券', date: '2026.10.01 - 2026.12.31', scope: '专车' },
  { id: 'c4', amount: 5, cond: '无门槛', name: '拼车出行券', date: '2026.10.24 - 2026.10.31', scope: '拼车' },
  { id: 'c5', amount: 10, cond: '满 30 可用', name: '机场接送券', date: '2026.10.15 - 2026.11.15', scope: '机场订单' },
  { id: 'c6', amount: 6, cond: '满 20 可用', name: '夜间出行券', date: '2026.10.24 - 2026.11.24', scope: '22:00 后' }
];
const EXPIRED = [
  { id: 'e1', amount: 12, cond: '满 35 可用', name: '国庆出行券', date: '2026.10.01 - 2026.10.07', scope: '快车' },
  { id: 'e2', amount: 3, cond: '无门槛', name: '早高峰券', date: '2026.09.01 - 2026.09.30', scope: '全部车型' }
];
Page({
  data: { tabs: [{ key: 'ok', name: '可使用' }, { key: 'used', name: '已使用' }, { key: 'expired', name: '已过期' }], tab: 'ok', list: [], code: '' },
  onLoad() { this.refresh(); },
  refresh() {
    const used = wx.getStorageSync('ride_coupon_used') || [];
    const t = this.data.tab;
    const list = t === 'ok' ? ALL.filter(c => used.indexOf(c.id) < 0) : t === 'used' ? ALL.filter(c => used.indexOf(c.id) >= 0) : EXPIRED;
    this.setData({ list });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.refresh(); },
  onCode(e) { this.setData({ code: e.detail.value }); },
  onExchange() {
    if (!this.data.code) { wx.showToast({ title: '请输入兑换码', icon: 'none' }); return; }
    wx.showToast({ title: '兑换码无效或已过期', icon: 'none' });
  },
  onUse(e) {
    const id = e.currentTarget.dataset.id;
    const used = wx.getStorageSync('ride_coupon_used') || [];
    used.push(id);
    wx.setStorageSync('ride_coupon_used', used);
    wx.showToast({ title: '已选用，下单自动抵扣', icon: 'none' });
    this.refresh();
  }
});
