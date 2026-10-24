const store = require('../../utils/store.js');
const STATUS = { paid: '花艺制作中', shipping: '配送中', done: '已签收', cancel: '已取消' };
Page({
  data: {
    tabs: [{ k: 'all', n: '全部' }, { k: 'paid', n: '制作中' }, { k: 'shipping', n: '配送中' }, { k: 'done', n: '已签收' }],
    tab: 'all', list: []
  },
  onShow() { this.refresh(); },
  refresh() {
    const list = store.getOrders().filter(o => this.data.tab === 'all' || o.status === this.data.tab)
      .map(o => {
        const items = o.items.map(store.expand);
        return Object.assign({}, o, { items, st: STATUS[o.status], qty: items.reduce((s, x) => s + x.qty, 0) });
      });
    this.setData({ list });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.refresh(); },
  track(e) {
    const o = this.data.list.find(x => x.no === e.currentTarget.dataset.no);
    const steps = o.status === 'done' ? '已签收（' + o.date + ' 10:42）\n配送员 李师傅 已送达' :
      (o.status === 'shipping' ? '配送员 李师傅 正在配送\n预计 ' + o.slot.split('-')[1] + ' 前送达' : '花艺师正在为您精心制作\n预计 ' + o.date + ' ' + o.slot + ' 送达');
    wx.showModal({ title: '订单进度', content: steps, showCancel: false, confirmText: '知道了' });
  },
  cancel(e) {
    const no = e.currentTarget.dataset.no;
    wx.showModal({ title: '取消订单', content: '花艺师可能已开始制作，确定要取消吗？款项将原路退回。', confirmColor: '#ef4f76', success: r => {
      if (!r.confirm) return;
      const all = store.getOrders();
      const o = all.find(x => x.no === no);
      if (o) o.status = 'cancel';
      store.saveOrders(all);
      this.refresh();
      wx.showToast({ title: '已取消，退款 1-3 天到账', icon: 'none' });
    } });
  },
  again(e) {
    const o = store.getOrders().find(x => x.no === e.currentTarget.dataset.no);
    o.items.forEach(x => store.addToCart({ id: x.id, stems: x.stems, pack: x.pack, qty: x.qty, date: store.dayAfter(1), slot: o.slot, card: '' }));
    wx.switchTab({ url: '/pages/cart/cart' });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); }
});
