const D = require('../../utils/date');
const store = require('../../utils/store');
const order = require('../../utils/order');
const cal = require('../../utils/calendar');
// 「途栖酒店民宿」（sample/hotel）：支付完顺手订目的地的酒店，wx.navigateToMiniProgram 跳过去
const HOTEL = { appId: 'wxe590495f6258b3e6', name: '途栖酒店民宿' };

Page({
  data: { o: null, paid: false, legs: [], pax: [], dest: '' },
  onLoad(q) {
    const o = store.order(q.id) || store.orders()[0];
    if (!o) return;
    this.o = o;
    this.setData({
      o: { id: o.id, total: o.total, method: o.payMethod || '', status: order.STATUS[o.status] },
      paid: o.status === 'paid',
      legs: o.legs.map((l, i) => ({ k: i, text: D.md(l.flight.date) + ' ' + D.weekLabel(l.flight.date) + ' ' + l.flight.depText + ' 起飞',
        route: l.flight.fromName + ' → ' + l.flight.toName, no: l.flight.airlineName + l.flight.no + ' · ' + l.cabin.name, ap: l.flight.depAirport })),
      pax: o.pax.map(p => ({ name: p.name, ticket: p.ticket || '出票中' })),
      dest: o.legs[0].flight.toName
    });
  },
  // 城市与日期放 extraData（对方在 getEnterOptionsSync().referrerInfo.extraData 里读）：不放进 path 的 query，
  // 中文进 URL 要编解码，两边各解一次就乱了。往返票的回程日就是离店日
  goHotel() {
    const go = this.o.legs[0].flight;
    const back = this.o.legs[1];
    wx.navigateToMiniProgram({
      appId: HOTEL.appId,
      path: 'pages/list/list',
      extraData: { from: 'flight', fromName: '云翼机票', city: go.toName, checkin: go.date, checkout: back ? back.flight.date : '' },
      fail(err) {
        const msg = (err && err.errMsg) || 'navigateToMiniProgram:fail';
        // 用户在确认框里点了取消：不再弹一次
        if (msg.indexOf('cancel') < 0) wx.showModal({ title: '没能打开「' + HOTEL.name + '」', content: msg, showCancel: false });
      }
    });
  },
  goDetail() { wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + this.o.id }); },
  addCal() { cal.addToCalendar(this.o); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
