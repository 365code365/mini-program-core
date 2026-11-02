const data = require('../../utils/data.js');
const host = require('../../utils/host.js');
// 医院导航：到院（系统地图，宿主没有地图能力时弹地址与交通）+ 院内（门诊楼按楼层列科室诊区）
// ?dept=科室 id：从挂号单进来，高亮要去的诊区
function floorOf(room) {
  const m = /(\d+) 楼/.exec(room || '');
  return m ? Number(m[1]) : 0;
}
Page({
  data: { h: data.hospital, target: null, floors: [], open: 0 },
  onLoad(q) {
    const target = q.dept ? data.findDept(q.dept) : null;
    const map = {};
    data.groups.forEach(g => g.depts.forEach(d => {
      const f = floorOf(d.room);
      (map[f] = map[f] || []).push({ id: d.id, name: d.name, area: d.room.split(' ').slice(2).join(' '), on: !!target && target.id === d.id });
    }));
    // WXML 里不能调方法（微信不支持 {{ a.join() }}），设施文案先拼好
    const floors = Object.keys(map).map(Number).sort((a, b) => b - a).map(f => {
      const fac = data.facilities[f] || [];
      return { f, depts: map[f], fac: fac.join('、'), sum: map[f].length + ' 个诊区' + (fac.length ? ' · ' + fac[0] : '') };
    });
    this.setData({ target, floors, open: target ? floorOf(target.room) : 1 });
    if (target) wx.setNavigationBarTitle({ title: '去' + target.name });
  },
  onFloor(e) {
    const f = Number(e.currentTarget.dataset.f);
    this.setData({ open: this.data.open === f ? 0 : f });
  },
  openMap() { host.openMap(); },
  copyAddr() { host.copy(data.hospital.address); },
  callHos() { host.call(data.hospital.phone, '医院总机'); },
  goDept(e) { wx.navigateTo({ url: '/pages/doctors/doctors?dept=' + e.currentTarget.dataset.id }); }
});
