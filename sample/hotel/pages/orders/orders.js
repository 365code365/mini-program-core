const app = getApp();
const SEED = [
  { id: 'HT20260915', hid: 'h5', hotel: '亚朵酒店（杭州湖滨银泰店）', img: '/images/h5.jpg', room: '几木大床房', count: 1, inShort: '9月15日', outShort: '9月17日', nights: 2, guests: '林晓舟', total: 1193, status: 'done' },
  { id: 'HT20260802', hid: 'h2', hotel: '隐居·西溪湿地民宿', img: '/images/h2.jpg', room: '竹林庭院大床房', count: 1, inShort: '8月2日', outShort: '8月3日', nights: 1, guests: '林晓舟', total: 663, status: 'cancel' }
];
Page({
  data: {
    tabs: [{ key: 'all', name: '全部' }, { key: 'wait', name: '待入住' }, { key: 'done', name: '已完成' }, { key: 'cancel', name: '已取消' }],
    tab: 'all', list: [], statusText: { wait: '待入住', done: '已完成', cancel: '已取消' }
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    this.load();
  },
  load() {
    const all = app.getOrders().concat(SEED);
    this.setData({ list: this.data.tab === 'all' ? all : all.filter(o => o.status === this.data.tab) });
  },
  onTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.load(); },
  onOpen(e) {
    const o = this.data.list.find(x => x.id === e.currentTarget.dataset.id);
    wx.showModal({ title: '订单 ' + o.id, content: o.hotel + '\n' + o.room + ' · ' + o.inShort + '入住 · ' + o.nights + '晚\n实付 ¥' + o.total, showCancel: false });
  },
  onCancel(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '取消订单', content: '当前可免费取消，款项将在 1-3 个工作日原路退回。', confirmText: '确认取消', confirmColor: '#FF5A36',
      success: (r) => {
        if (!r.confirm) return;
        const orders = app.getOrders().map(o => o.id === id ? Object.assign({}, o, { status: 'cancel' }) : o);
        app.saveOrders(orders);
        this.load();
        wx.showToast({ title: '已取消', icon: 'success' });
      }
    });
  },
  onContact() { wx.showActionSheet({ itemList: ['拨打酒店电话', '修改到店时间', '在线客服'] }); },
  onRebook(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.hid }); },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
