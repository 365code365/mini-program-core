const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const flows = require('../../utils/flows.js');
// tab 只有三个：候诊中算「待就诊」，过期未就诊算「已取消」
const IN_TAB = { wait: ['wait', 'checked'], done: ['done'], cancel: ['cancel', 'miss'] };
Page({
  data: {
    tabs: [{ key: 'wait', name: '待就诊' }, { key: 'done', name: '已完成' }, { key: 'cancel', name: '已取消' }],
    tab: 'wait', tabName: '待就诊', list: []
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.refresh();
    // 候诊中的号每分钟叫一位：开着页面也要跟着走
    clearInterval(this.timer);
    this.timer = setInterval(() => this.refresh(), 15000);
  },
  onHide() { clearInterval(this.timer); },
  onUnload() { clearInterval(this.timer); },
  refresh() {
    store.tick();
    const t = this.data.tab;
    const list = store.getOrders().filter(o => IN_TAB[t].indexOf(o.status) >= 0).map(o => {
      const st = store.stageOf(o);
      return Object.assign({}, o, { st: { status: st.status, tip: st.tip, stage: st.stage, unpaid: !!st.unpaid } });
    });
    this.setData({ list, tabName: this.data.tabs.find(x => x.key === t).name });
  },
  setTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.refresh(); },
  find(e) { return store.findOrder(e.currentTarget.dataset.id); },
  goDetail(e) { wx.navigateTo({ url: '/pages/order-detail/order-detail?id=' + e.currentTarget.dataset.id }); },
  cancel(e) { flows.cancel(this.find(e)).then(() => this.refresh()); },
  checkin(e) { flows.checkin(this.find(e)).then(() => this.refresh()); },
  goBills() { wx.navigateTo({ url: '/pages/bills/bills' }); },
  // 再次挂号：回到同一位医生的排班（老数据没有 doctorId 就按姓名找）
  again(e) {
    const o = e.currentTarget.dataset.id ? this.find(e) : null;
    const doc = o && (data.findDoctor(o.doctorId) || data.doctors.find(d => d.name === o.doctor));
    if (doc) wx.navigateTo({ url: '/pages/doctor/doctor?id=' + doc.id });
    else wx.switchTab({ url: '/pages/dept/dept' });
  }
});
