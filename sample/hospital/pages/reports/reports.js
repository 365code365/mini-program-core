const store = require('../../utils/store.js');
const clinic = require('../../utils/clinic.js');
Page({
  data: { people: [], pid: '', list: [], unread: 0 },
  onShow() {
    store.tick();
    const people = [{ id: '', name: '全部' }].concat(store.getPatients().map(p => ({ id: p.id, name: p.name })));
    this.setData({ people });
    this.refresh();
  },
  refresh() {
    const all = store.getReports();
    const list = all.filter(r => !this.data.pid || r.patientId === this.data.pid).map(r => {
      const t = clinic.REPORTS[r.key] || { name: '检查报告', kind: '检查' };
      return Object.assign({}, r, { name: t.name, kind: t.kind, abnormal: (t.items || []).filter(x => x.f).length });
    });
    this.setData({ list, unread: all.filter(r => !r.read).length });
  },
  pickPerson(e) {
    this.setData({ pid: e.currentTarget.dataset.id });
    this.refresh();
  },
  goDetail(e) { wx.navigateTo({ url: '/pages/report-detail/report-detail?id=' + e.currentTarget.dataset.id }); },
  goBills() { wx.navigateTo({ url: '/pages/bills/bills' }); }
});
