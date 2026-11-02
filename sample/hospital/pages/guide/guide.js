const data = require('../../utils/data.js');
const host = require('../../utils/host.js');
// 智能导诊：选部位 → 勾症状（可多选，跨部位）→ 按命中次数推荐科室；有急症就先提示去急诊
Page({
  data: { areas: [], area: 0, syms: [], picked: [], recs: [], urgent: '' },
  onLoad() {
    this.on = {};
    this.render();
  },
  render() {
    const on = this.on;
    const areas = data.symptomAreas.map(a => ({ key: a.key, name: a.name, n: a.items.filter(x => on[a.key + ':' + x.n]).length }));
    const cur = data.symptomAreas[this.data.area];
    const syms = cur.items.map(x => ({ id: cur.key + ':' + x.n, n: x.n, on: !!on[cur.key + ':' + x.n] }));
    const picked = [];
    const count = {};
    let urgent = '';
    data.symptomAreas.forEach(a => a.items.forEach(x => {
      if (!on[a.key + ':' + x.n]) return;
      picked.push({ id: a.key + ':' + x.n, n: x.n });
      count[x.d] = (count[x.d] || 0) + 1;
      if (x.urgent && !urgent) urgent = x.n;
    }));
    const recs = Object.keys(count)
      .sort((a, b) => count[b] - count[a])
      .slice(0, 3)
      .map((id, i) => {
        const d = data.findDept(id);
        return { id, name: d.name, desc: d.desc, room: d.room, hit: count[id], best: i === 0 };
      });
    this.setData({ areas, syms, picked, recs, urgent });
  },
  pickArea(e) {
    this.setData({ area: Number(e.currentTarget.dataset.i) });
    this.render();
  },
  toggleSym(e) {
    const id = String(e.currentTarget.dataset.id);
    if (this.on[id]) delete this.on[id];
    else this.on[id] = true;
    this.render();
  },
  clearAll() {
    this.on = {};
    this.render();
  },
  goDept(e) { wx.navigateTo({ url: '/pages/doctors/doctors?dept=' + e.currentTarget.dataset.id }); },
  callEmergency() { host.call(data.hospital.emergency, '急诊 24 小时'); },
  goNavi() { wx.navigateTo({ url: '/pages/navi/navi' }); }
});
