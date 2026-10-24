const data = require('../../utils/data.js');

Page({
  data: { groups: data.groups, active: 0, intoView: '', kw: '', results: [] },
  onReady() { this.measure(); },
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
  onInput(e) {
    const kw = (e.detail.value || '').trim();
    const results = [];
    if (kw) {
      data.groups.forEach(g => g.depts.forEach(d => {
        if (d.name.indexOf(kw) >= 0 || d.desc.indexOf(kw) >= 0 || g.name.indexOf(kw) >= 0) {
          results.push({ id: d.id, name: d.name, desc: d.desc, group: g.name });
        }
      }));
    }
    this.setData({ kw, results });
  },
  clearKw() {
    this.setData({ kw: '', results: [] });
    setTimeout(() => this.measure(), 50);
  },
  goDoctors(e) {
    wx.navigateTo({ url: '/pages/doctors/doctors?dept=' + e.currentTarget.dataset.id });
  }
});
