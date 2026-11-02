// 办件进度：按「办件编号 + 申请人姓名」查询（他人代查也能用），或从我的办件 / 结果页带 id 进来。
// 状态变更：待受理可撤回；待补正可补传材料（→ 审核中）；审核中可催办（每件每天一次）。
const store = require('../../utils/store');
const { getItem } = require('../../utils/items');

Page({
  data: { no: '', name: '', order: null, steps: [], st: null, notFound: false, mine: [] },
  onLoad(q) {
    const mine = store.getOrders().slice(0, 4).map(o => ({ id: o.id, name: o.name }));
    this.setData({ mine });
    if (q.id) this.show(q.id);
  },
  show(id) {
    const o = store.getOrder(id);
    if (!o) { this.setData({ order: null, notFound: true }); return; }
    const item = getItem(o.itemId);
    const steps = o.timeline.map(t => ({ title: t.title, desc: t.desc, time: store.fmtTime(t.t) })).reverse();
    this.setData({
      order: o, no: o.id, name: o.applicant.name, notFound: false, steps, st: store.STATUS[o.status],
      promise: item.promise ? item.promise + ' 个工作日' : '即时办结', dept: item.dept,
      deadline: item.promise ? store.fmtDate(o.createdAt + item.promise * 1.4 * store.DAY) : store.fmtDate(o.createdAt)
    });
  },
  onNo(e) { this.setData({ no: e.detail.value.trim().toUpperCase() }); },
  onName(e) { this.setData({ name: e.detail.value.trim() }); },
  query() {
    const { no, name } = this.data;
    if (!/^YC\d{8,}$/.test(no)) { wx.showToast({ title: '办件编号以 YC 开头，见短信或结果页', icon: 'none' }); return; }
    if (!name) { wx.showToast({ title: '请填写申请人姓名', icon: 'none' }); return; }
    const o = store.getOrder(no);
    if (!o || o.applicant.name !== name) { this.setData({ order: null, notFound: true }); return; }
    this.show(no);
  },
  pick(e) { this.show(e.currentTarget.dataset.id); },
  push(o, title, desc, status) {
    store.updateOrder(o.id, (x) => {
      x.timeline.push({ t: Date.now(), title, desc });
      if (status) x.status = status;
    });
    this.show(o.id);
  },
  withdraw() {
    const o = this.data.order;
    wx.showModal({
      title: '撤回申请', content: '撤回后本次申报作废，如需办理须重新提交。确定撤回吗？', confirmColor: '#dc2626',
      success: (r) => { if (r.confirm) { this.push(o, '申请人撤回', '申请人主动撤回本次申报', 'withdrawn'); wx.showToast({ title: '已撤回', icon: 'none' }); } }
    });
  },
  supplement() {
    const o = this.data.order;
    const done = (name) => {
      this.push(o, '补正材料', `已补充上传：${name}`, 'reviewing');
      wx.showToast({ title: '补正材料已提交', icon: 'none' });
    };
    wx.chooseMessageFile({
      count: 1, type: 'file',
      success: (r) => done(r.tempFiles[0].name),
      fail: () => wx.showModal({
        title: '未选择文件', content: '是否使用示例材料「房屋租赁合同（含签字页）.pdf」完成补正？', confirmText: '用示例',
        success: (r) => { if (r.confirm) done('房屋租赁合同（含签字页）（示例）.pdf'); }
      })
    });
  },
  urge() {
    const o = this.data.order;
    const today = store.fmtDate(Date.now());
    if (o.urged === today) { wx.showToast({ title: '今天已催办过，请耐心等待', icon: 'none' }); return; }
    store.updateOrder(o.id, (x) => { x.urged = today; });
    this.push(store.getOrder(o.id), '申请人催办', `已提醒${this.data.dept}加快办理`);
    wx.showToast({ title: '已催办', icon: 'none' });
  },
  call() {
    wx.makePhoneCall({
      phoneNumber: '12345',
      fail: () => wx.showModal({ title: '政务服务便民热线', content: '请拨打 12345（工作日 8:30-17:30 人工服务，其余时间留言）', showCancel: false })
    });
  }
});
