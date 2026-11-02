// 私信会话列表：未读数同步到自定义 tabBar 的红点；长按会话可标已读 / 删除
const store = require('../../utils/store.js');
const D = require('../../utils/data.js');
function preview(m) {
  if (!m) return '开始聊聊这件宝贝吧';
  if (m.type === 'offer') return (m.from === 'me' ? '[我的出价] ¥' : '[卖家改价] ¥') + m.price;
  if (m.type === 'status') return '[交易] ' + m.text;
  return (m.from === 'me' ? '我：' : '') + m.text;
}
Page({
  data: { list: [], unread: 0 },
  onShow() { this.refresh(); },
  refresh() {
    const list = store.convs().slice().sort((a, b) => b.updated - a.updated).map(c => {
      const it = store.findItem(c.item);
      const peer = D.SELLERS[c.peer] || D.ME;
      return { id: c.id, name: peer.name, avatar: peer.avatar, unread: c.unread || 0, time: store.timeText(c.updated),
        last: preview(c.msgs[c.msgs.length - 1]), cover: it ? it.cover : '/images/img-fail.jpg', sold: it && it.sold };
    });
    this.setData({ list, unread: store.unreadTotal() });
    store.syncTab(this, 2);
  },
  open(e) {
    if (Date.now() - (this._lp || 0) < 800) return;
    wx.navigateTo({ url: '/pages/chat/chat?id=' + e.currentTarget.dataset.id });
  },
  onLong(e) {
    this._lp = Date.now();
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: ['标为已读', '删除会话'],
      success: r => {
        if (r.tapIndex === 0) { store.markRead(id); this.refresh(); return; }
        wx.showModal({
          title: '删除会话', content: '删除后聊天记录将不再显示', confirmColor: '#ff4d2d',
          success: m => { if (m.confirm) { store.removeConv(id); this.refresh(); } }
        });
      }
    });
  },
  readAll() {
    if (!this.data.unread) { wx.showToast({ title: '没有未读消息', icon: 'none' }); return; }
    store.markRead();
    this.refresh();
    wx.showToast({ title: '已全部标为已读', icon: 'none' });
  },
  goHome() { wx.switchTab({ url: '/pages/index/index' }); },
  onPullDownRefresh() { this.refresh(); wx.stopPullDownRefresh(); }
});
