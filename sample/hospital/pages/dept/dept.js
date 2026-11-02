const data = require('../../utils/data.js');
Page({
  // kwSeq：打过字的搜索框，「取消」时 setData 清空画面不更新，换 id 让它重建
  data: { groups: data.groups, active: 0, intoView: '', kw: '', kwSeq: 0, results: [], docResults: [] },
  onReady() { this.measure(); },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
  },
  // 右侧每个分组的顶部位置，滚动时据此反推左侧高亮
  measure() {
    this.createSelectorQuery().selectAll('.sec').boundingClientRect().exec(res => {
      const rects = (res && res[0]) || [];
      if (!rects.length) return;
      const base = rects[0].top;
      this.tops = rects.map(r => r.top - base);
    });
  },
  pickGroup(e) {
    const i = e.currentTarget.dataset.index;
    this.lock = true;
    this.setData({ active: i, intoView: 'g-' + data.groups[i].id });
    setTimeout(() => { this.lock = false; }, 400);
  },
  onScroll(e) {
    if (this.lock || !this.tops) return;
    const y = e.detail.scrollTop + 20;
    let i = 0;
    for (let k = 0; k < this.tops.length; k++) if (this.tops[k] <= y) i = k;
    if (i !== this.data.active) this.setData({ active: i });
  },
  // 搜科室（名称、擅长病症、分组）和医生（姓名、擅长）
  onInput(e) {
    const kw = (e.detail.value || '').trim();
    const results = [];
    const docResults = [];
    if (kw) {
      data.groups.forEach(g => g.depts.forEach(d => {
        if (d.name.indexOf(kw) >= 0 || d.desc.indexOf(kw) >= 0 || g.name.indexOf(kw) >= 0) {
          results.push({ id: d.id, name: d.name, desc: d.desc, group: g.name });
        }
      }));
      data.doctors.forEach(d => {
        if (d.name.indexOf(kw) >= 0 || d.good.indexOf(kw) >= 0) {
          docResults.push({ id: d.id, name: d.name, title: d.title, sx: d.sx, color: d.color, dept: data.findDept(d.dept).name });
        }
      });
    }
    this.setData({ kw, results, docResults: docResults.slice(0, 8) });
  },
  clearKw() {
    this.setData({ kw: '', kwSeq: this.data.kwSeq + 1, results: [], docResults: [] });
    setTimeout(() => this.measure(), 50);
  },
  goDoctors(e) {
    wx.navigateTo({ url: '/pages/doctors/doctors?dept=' + e.currentTarget.dataset.id });
  },
  goDoctor(e) {
    wx.navigateTo({ url: '/pages/doctor/doctor?id=' + e.currentTarget.dataset.id });
  }
});
