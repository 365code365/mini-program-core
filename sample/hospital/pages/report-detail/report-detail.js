const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const clinic = require('../../utils/clinic.js');
Page({
  data: { r: null, t: null, abnormal: 0 },
  onLoad(q) {
    const r = store.getReports().find(x => x.id === q.id) || null;
    if (!r) return;
    const t = clinic.REPORTS[r.key];
    store.markReportRead(r.id);
    this.setData({ r, t, abnormal: (t.items || []).filter(x => x.f).length });
    wx.setNavigationBarTitle({ title: t.name });
  },
  // 看不懂：带上报告去问诊；要复查：回到开单科室挂号
  onConsult() { wx.navigateTo({ url: '/pages/consult/consult?report=' + this.data.r.id }); },
  onRevisit() {
    let id = 'hx';
    data.groups.forEach(g => g.depts.forEach(d => { if (d.name === this.data.r.dept) id = d.id; }));
    wx.navigateTo({ url: '/pages/doctors/doctors?dept=' + id });
  }
});
