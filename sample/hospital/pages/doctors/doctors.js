const data = require('../../utils/data.js');

Page({
  data: {
    dept: {},
    all: [],
    list: [],
    filter: 'all',
    filters: [{ key: 'all', name: '全部' }, { key: 'has', name: '有号' }, { key: 'chief', name: '主任医师' }, { key: 'cheap', name: '普通号' }]
  },
  onLoad(q) {
    const id = q.dept || 'xxg';
    const dept = data.findDept(id) || { name: '心血管内科', desc: '' };
    let all = data.doctors.filter(d => d.dept === id);
    // 演示数据只给部分科室排了医生，其余科室借用同系列医生避免空页
    if (!all.length) all = data.doctors.slice(0, 3).map(d => Object.assign({}, d, { dept: id }));
    wx.setNavigationBarTitle({ title: dept.name });
    this.setData({ dept, all, list: all });
  },
  setFilter(e) {
    const f = e.currentTarget.dataset.key;
    const all = this.data.all;
    const list = all.filter(d => f === 'all' || (f === 'has' && d.left > 0) || (f === 'chief' && d.title === '主任医师') || (f === 'cheap' && d.fee <= 20));
    this.setData({ filter: f, list });
  },
  goDetail(e) {
    wx.navigateTo({ url: '/pages/doctor/doctor?id=' + e.currentTarget.dataset.id });
  }
});
