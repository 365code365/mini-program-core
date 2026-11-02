const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
// 三种进法：?dept=科室（选医生）/ ?today=1（当日挂号：今天还有号的医生）/ ?follow=1（我关注的医生）
Page({
  data: {
    mode: 'dept',
    dept: {},
    head: { name: '', desc: '' },
    all: [],
    list: [],
    filter: 'all',
    filters: [{ key: 'all', name: '全部' }, { key: 'has', name: '有号' }, { key: 'chief', name: '主任医师' }, { key: 'cheap', name: '普通号' }],
    emptyText: '该科室暂无符合条件的医生'
  },
  onLoad(q) {
    this.q = q;
    if (q.today === '1') {
      this.setData({ mode: 'today', head: { name: '当日挂号', desc: '今天 ' + store.today().slice(5) + ' 还有号源的医生，挂号后请当天到院签到' }, emptyText: '今天的号源已全部约满或已过时段' });
      wx.setNavigationBarTitle({ title: '当日挂号' });
    } else if (q.follow === '1') {
      this.setData({ mode: 'follow', head: { name: '我关注的医生', desc: '关注的医生放号时会提醒您' }, emptyText: '还没有关注医生，在医生主页点「关注」' });
      wx.setNavigationBarTitle({ title: '我关注的医生' });
    } else {
      const dept = data.findDept(q.dept) || data.findDept('xxg');
      this.setData({ dept, head: { name: dept.name, desc: '擅长：' + dept.desc } });
      wx.setNavigationBarTitle({ title: dept.name });
    }
  },
  // 回到这页时余号要重算（刚挂了号 / 取消了关注）
  onShow() {
    const mode = this.data.mode;
    const orders = store.getOrders();
    const days = mode === 'today' ? 1 : 7;
    let docs;
    if (mode === 'today') docs = data.doctors.filter(d => store.leftOf(d, 1, orders) > 0);
    else if (mode === 'follow') docs = store.getFollows().map(id => data.findDoctor(id)).filter(Boolean);
    else docs = data.doctors.filter(d => d.dept === this.data.dept.id);
    // 接诊量过万写成「1.3 万」：卡片底行要同时放下好评率、接诊量、挂号费和余号角标
    const all = docs.map(d => Object.assign({}, d, { left: store.leftOf(d, days, orders), deptName: data.findDept(d.dept).name, visitsText: d.visits >= 10000 ? (d.visits / 10000).toFixed(1) + ' 万' : String(d.visits) }));
    this.setData({ all });
    this.applyFilter();
  },
  applyFilter() {
    const f = this.data.filter;
    const list = this.data.all.filter(d => f === 'all' || (f === 'has' && d.left > 0) || (f === 'chief' && d.title === '主任医师') || (f === 'cheap' && d.fee <= 20));
    this.setData({ list });
  },
  setFilter(e) {
    this.setData({ filter: e.currentTarget.dataset.key });
    this.applyFilter();
  },
  goDetail(e) {
    wx.navigateTo({ url: '/pages/doctor/doctor?id=' + e.currentTarget.dataset.id + (this.data.mode === 'today' ? '&today=1' : '') });
  },
  goDept() { wx.switchTab({ url: '/pages/dept/dept' }); }
});
