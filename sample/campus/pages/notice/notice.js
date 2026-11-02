const D = require('../../utils/data.js');
const store = require('../../utils/store.js');
Page({
  data: { n: null, paras: [] },
  onLoad(q) {
    const n = D.NOTICES.find(x => x.id === q.id);
    if (!n) return;
    store.markRead(n.id);
    this.setData({ n, paras: n.body.split('\n') });
  },
  copy() { wx.setClipboardData({ data: this.data.n.title + '\n' + this.data.n.body }); },
  onShareAppMessage() { return { title: this.data.n.title, path: '/pages/notice/notice?id=' + this.data.n.id }; }
});
