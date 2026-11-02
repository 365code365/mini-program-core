const D = require('../../utils/date');
const store = require('../../utils/store');
const order = require('../../utils/order');
const cal = require('../../utils/calendar');

Page({
  data: { o: null, paid: false, legs: [], pax: [] },
  onLoad(q) {
    const o = store.order(q.id) || store.orders()[0];
    if (!o) return;
    this.o = o;
    this.setData({
      o: { id: o.id, total: o.total, method: o.payMethod || '', status: order.STATUS[o.status] },
      paid: o.status === 'paid',
      legs: o.legs.map((l, i) => ({ k: i, text: D.md(l.flight.date) + ' ' + D.weekLabel(l.flight.date) + ' ' + l.flight.depText + ' 起飞',
        route: l.flight.fromName + ' → ' + l.flight.toName, no: l.flight.airlineName + l.flight.no + ' · ' + l.cabin.name, ap: l.flight.depAirport })),
      pax: o.pax.map(p => ({ name: p.name, ticket: p.ticket || '出票中' }))
    });
  },
  goDetail() { wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + this.o.id }); },
  addCal() { cal.addToCalendar(this.o); },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
