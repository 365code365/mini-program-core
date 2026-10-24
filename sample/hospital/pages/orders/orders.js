const store = require('../../utils/store.js');

Page({
  data: {
    tabs: [{ key: 'wait', name: '待就诊' }, { key: 'done', name: '已完成' }, { key: 'cancel', name: '已取消' }],
    tab: 'wait', tabName: '待就诊', list: [],
    statusText: { wait: '待就诊', done: '已完成', cancel: '已取消' }
  },
  onShow() { this.refresh(); },
  refresh() {
    const t = this.data.tab;
    this.setData({ list: store.getOrders().filter(o => o.status === t), tabName: this.data.tabs.find(x => x.key === t).name });
  },
  setTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.refresh(); },
  cancel(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消预约', content: '取消后挂号费将原路退回，确定取消吗？', confirmText: '确定取消', confirmColor: '#ff5a36',
      success: (r) => {
        if (!r.confirm) return;
        const all = store.getOrders().map(o => o.id === id ? Object.assign({}, o, { status: 'cancel' }) : o);
        store.saveOrders(all);
        this.refresh();
        wx.showToast({ title: '已取消，退款 1~3 个工作日到账', icon: 'none' });
      }
    });
  },
  checkin() { wx.showToast({ title: '请到院后在诊区签到', icon: 'none' }); },
  again() { wx.switchTab({ url: '/pages/dept/dept' }); }
});
