// 行程详情：轨迹地图（起终点 + 折线）、时间地点、费用明细；可对这辆车报修、对费用有疑问联系客服
const fee = require('../../utils/fee.js');
const store = require('../../utils/store.js');

const SERVICE = '4000000999';

Page({
  data: { trip: null, v: null, markers: [], polyline: [], center: null },

  onLoad(q) {
    const t = store.trip(q.id);
    if (!t) return;
    const a = t.path[0], b = t.path[t.path.length - 1];
    this.setData({
      trip: t,
      center: { latitude: (a.latitude + b.latitude) / 2, longitude: (a.longitude + b.longitude) / 2 },
      v: {
        name: fee.PRICING[t.type].name,
        begin: store.fmtTime(t.startAt), end: store.fmtTime(t.endAt),
        dur: fee.fmtMin(t.sec), km: (t.meters / 1000).toFixed(2),
        speed: t.sec ? (t.meters / t.sec * 3.6).toFixed(1) : '0.0',
        start: t.start.toFixed(2), extra: t.extra.toFixed(2), total: t.total.toFixed(2),
        discount: t.discount.toFixed(2), paid: t.paid.toFixed(2),
        carbon: (t.meters / 1000 * 0.15).toFixed(2),
        payBy: { balance: '余额支付', wechat: '微信支付', card: '骑行卡抵扣', free: '免费' }[t.payBy] || '待支付',
        paidAt: t.paidAt ? store.fmtTime(t.paidAt) : '—'
      },
      markers: [
        { id: 1, latitude: a.latitude, longitude: a.longitude, iconPath: '/images/marker-start.png', width: 28, height: 35, anchor: { x: 0.5, y: 1 } },
        { id: 2, latitude: b.latitude, longitude: b.longitude, iconPath: '/images/marker-end.png', width: 28, height: 35, anchor: { x: 0.5, y: 1 } }
      ],
      polyline: [{ points: t.path, color: '#13a868', width: 6, arrowLine: true }]
    });
  },

  onReady() {
    const t = this.data.trip;
    if (!t) return;
    // 让整条轨迹都落在视野里；宿主不支持就保持中点 + 默认缩放
    wx.createMapContext('tripMap', this).includePoints({ points: t.path, padding: [40, 40, 40, 40], fail() {} });
  },

  onFault() { wx.navigateTo({ url: '/pages/fault/fault?no=' + this.data.trip.no }); },

  onService() {
    wx.makePhoneCall({
      phoneNumber: SERVICE,
      fail: () => wx.setClipboardData({
        data: '400-000-0999',
        success: () => wx.showToast({ title: '客服电话已复制，可粘贴拨号', icon: 'none' })
      })
    });
  },

  goHome() { wx.reLaunch({ url: '/pages/index/index' }); }
});
