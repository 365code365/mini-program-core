const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const start = require('../../utils/start.js');
Page({
  data: { st: null, cur: null, curTotal: '', periods: [], segs: ['全部', '快充', '慢充'], sIdx: 0, terms: [], count: {}, fav: false },
  onLoad(q) {
    this.id = q.id;
    const cur = data.currentPeriod();
    const periods = data.PERIODS.map(p => Object.assign({}, p, { elec: p.elec.toFixed(2), service: p.service.toFixed(2), total: (p.elec + p.service).toFixed(2) }));
    this.setData({ cur: Object.assign({}, cur, { elec: cur.elec.toFixed(2), service: cur.service.toFixed(2) }), curTotal: (cur.elec + cur.service).toFixed(2), periods });
  },
  // 每次回到这页重读：刚开始 / 结束的充电会改变终端占用
  onShow() {
    const st = store.station(this.id);
    this.setData({ st, fav: store.getFavs().indexOf(st.id) >= 0 });
    wx.setNavigationBarTitle({ title: st.brand + '充电站' });
    this.applySeg();
  },
  applySeg() {
    const s = this.data.sIdx;
    const terms = this.data.st.list
      .filter(t => s === 0 || (s === 1 && t.power > 7) || (s === 2 && t.power <= 7))
      .map(t => Object.assign({}, t, { short: t.no.split('-')[1] + ' 号桩' }));
    const count = { idle: 0, busy: 0, fault: 0 };
    terms.forEach(t => { count[t.status]++; });
    this.setData({ terms, count });
  },
  onSeg(e) { this.setData({ sIdx: Number(e.currentTarget.dataset.idx) }); this.applySeg(); },
  onTerm(e) {
    const t = this.data.st.list.find(x => x.no === String(e.currentTarget.dataset.no));
    if (t.mine) { wx.navigateTo({ url: '/pages/charging/charging' }); return; }
    if (t.status === 'fault') {
      wx.showModal({ title: t.no + ' 维护中', content: '该终端暂时不能使用，请换一个空闲终端。发现新的故障可以报修，运维会尽快处理。', confirmText: '我要报修', cancelText: '换一个', confirmColor: '#00B87A',
        success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/repair/repair?no=' + t.no }); } });
      return;
    }
    if (t.status === 'busy') { wx.showToast({ title: '该终端正在使用', icon: 'none' }); return; }
    start.confirm(t.no, this.data.curTotal);
  },
  onFav() {
    const fav = store.toggleFav(this.data.st.id);
    this.setData({ fav });
    wx.showToast({ title: fav ? '已收藏电站' : '已取消收藏', icon: 'none' });
  },
  onNav() { host.openMap(this.data.st); },
  onCall() { host.call(this.data.st.phone); },
  onScan() { wx.navigateTo({ url: '/pages/scan/scan' }); }
});
