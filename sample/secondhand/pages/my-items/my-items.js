// 我的发布：在售 / 已下架；擦亮（每天一次，排到前面）、降价、下架 / 重新上架、删除
const store = require('../../utils/store.js');
Page({
  data: { tab: 'on', list: [], counts: { on: 0, off: 0 } },
  onShow() { this.refresh(); },
  refresh() {
    const all = store.mine();
    const today = store.today();
    this.setData({
      counts: { on: all.filter(m => m.status === 'on').length, off: all.filter(m => m.status !== 'on').length },
      list: all.filter(m => (this.data.tab === 'on') === (m.status === 'on')).map(m => Object.assign({}, m, {
        cover: m.imgs[0], polishedToday: m.polished === today, expose: store.exposeOf(m.id), postedText: store.timeText(m.created)
      }))
    });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.refresh(); },
  patch(id, p) {
    const all = store.mine(), m = all.find(x => x.id === id);
    if (m) Object.assign(m, p);
    store.saveMine(all);
    this.refresh();
  },
  polish(e) {
    const id = e.currentTarget.dataset.id;
    const all = store.mine(), m = all.find(x => x.id === id);
    if (m.polished === store.today()) { wx.showToast({ title: '今天已经擦亮过了', icon: 'none' }); return; }
    // 擦亮：更新发布时间并排到最前
    m.polished = store.today(); m.created = Date.now();
    store.saveMine([m].concat(all.filter(x => x.id !== id)));
    wx.showToast({ title: '擦亮成功，宝贝已排到前面', icon: 'none' });
    this.refresh();
  },
  cut(e) {
    const id = e.currentTarget.dataset.id;
    const m = store.mine().find(x => x.id === id);
    const opts = [0.95, 0.9, 0.8].map(r => Math.max(1, Math.floor(m.price * r)));
    wx.showActionSheet({
      itemList: opts.map(p => '降到 ¥' + p),
      success: r => { this.patch(id, { price: opts[r.tapIndex] }); wx.showToast({ title: '已降价，想要的人会收到通知', icon: 'none' }); }
    });
  },
  toggle(e) {
    const id = e.currentTarget.dataset.id, on = this.data.tab === 'on';
    this.patch(id, { status: on ? 'off' : 'on' });
    wx.showToast({ title: on ? '已下架' : '已重新上架', icon: 'none' });
  },
  remove(e) {
    const id = e.currentTarget.dataset.id;
    wx.showModal({
      title: '删除宝贝', content: '删除后无法恢复，确定删除吗？', confirmColor: '#ff4d2d',
      success: r => { if (r.confirm) { store.saveMine(store.mine().filter(x => x.id !== id)); this.refresh(); } }
    });
  },
  open(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  goPublish() { wx.switchTab({ url: '/pages/publish/publish' }); }
});
