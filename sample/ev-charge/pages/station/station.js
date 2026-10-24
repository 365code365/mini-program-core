const data = require('../../utils/data.js');

Page({
  data: { st: null, cur: null, curTotal: '', periods: [], segs: ['全部', '快充', '慢充'], sIdx: 0, terms: [], count: {}, fav: false },
  onLoad(q) {
    const st = data.getStation(q.id);
    const cur = data.currentPeriod();
    const periods = data.PERIODS.map(p => Object.assign({}, p, { elec: p.elec.toFixed(2), service: p.service.toFixed(2), total: (p.elec + p.service).toFixed(2) }));
    const favs = wx.getStorageSync('ev_fav') || [];
    this.setData({ st, cur: Object.assign({}, cur, { elec: cur.elec.toFixed(2), service: cur.service.toFixed(2) }), curTotal: (cur.elec + cur.service).toFixed(2), periods, fav: favs.indexOf(st.id) >= 0 });
    wx.setNavigationBarTitle({ title: st.brand + '充电站' });
    this.applySeg();
  },
  applySeg() {
    const s = this.data.sIdx;
    const all = this.data.st.list;
    const terms = all
      .filter(t => s === 0 || (s === 1 && t.power > 7) || (s === 2 && t.power <= 7))
      .map(t => Object.assign({}, t, { short: t.no.split('-')[1] + ' 号桩' }));
    const count = { idle: 0, busy: 0, fault: 0 };
    terms.forEach(t => { count[t.status]++; });
    this.setData({ terms, count });
  },
  onSeg(e) { this.setData({ sIdx: e.currentTarget.dataset.idx }); this.applySeg(); },
  onTerm(e) {
    const t = this.data.st.list.find(x => x.no === e.currentTarget.dataset.no);
    if (t.status === 'fault') { wx.showToast({ title: '该终端维护中，请换一个', icon: 'none' }); return; }
    if (t.status === 'busy') { wx.showToast({ title: '该终端正在使用', icon: 'none' }); return; }
    wx.showModal({
      title: t.no + '（' + t.power + 'kW）',
      content: '请确认已插好充电枪，当前电价 ¥' + this.data.curTotal + '/度',
      confirmText: '开始充电',
      confirmColor: '#00B87A',
      success: (r) => { if (r.confirm) wx.navigateTo({ url: '/pages/charging/charging?no=' + t.no }); }
    });
  },
  onFav() {
    let favs = wx.getStorageSync('ev_fav') || [];
    const fav = !this.data.fav;
    favs = fav ? favs.concat([this.data.st.id]) : favs.filter(x => x !== this.data.st.id);
    wx.setStorageSync('ev_fav', favs);
    this.setData({ fav });
    wx.showToast({ title: fav ? '已收藏电站' : '已取消收藏', icon: 'none' });
  },
  onNav() {
    wx.showActionSheet({ itemList: ['高德地图', '腾讯地图', '复制地址'], success: (r) => {
      wx.showToast({ title: r.tapIndex === 2 ? '地址已复制' : '演示环境不跳转导航', icon: 'none' });
    } });
  },
  onScan() { wx.navigateTo({ url: '/pages/scan/scan' }); }
});
