const store = require('../../utils/store.js');
Page({
  data: { list: [], count: 0 },
  onShow() { this.refresh(); },
  refresh() {
    const all = store.getNotes();
    const by = {};
    const list = [];
    all.forEach(n => {
      if (!by[n.cid]) { by[n.cid] = { cid: n.cid, title: n.ctitle, items: [] }; list.push(by[n.cid]); }
      by[n.cid].items.push(n);
    });
    this.setData({ list, count: all.length });
  },
  open(e) { wx.navigateTo({ url: '/pages/lesson/lesson?cid=' + e.currentTarget.dataset.cid + '&lid=' + e.currentTarget.dataset.lid }); },
  del(e) {
    const id = String(e.currentTarget.dataset.id);
    wx.showModal({ title: '删除这条笔记？', confirmText: '删除', confirmColor: '#f0577a', success: r => {
      if (!r.confirm) return;
      store.saveNotes(store.getNotes().filter(n => n.id !== id));
      this.refresh();
      wx.showToast({ title: '已删除', icon: 'none' });
    } });
  },
  goLearn() { wx.switchTab({ url: '/pages/learn/learn' }); }
});
