const store = require('../../utils/store.js');
Page({
  data: { list: [], kw: '', total: 0, empty: false },
  onShow() { this.load(); },
  load() {
    const now = Date.now();
    const kw = this.data.kw.trim();
    let all = store.sessions().slice();
    all.sort((a, b) => (b.pinned - a.pinned) || (b.lastTime - a.lastTime));
    const list = [];
    all.forEach((s) => {
      const c = store.contact(s.id);
      if (!c) return;
      if (kw && c.name.indexOf(kw) < 0 && s.lastText.indexOf(kw) < 0) return;
      list.push({ id: s.id, name: c.remark ? c.name + '（' + c.remark + '）' : c.name, initial: c.initial, color: c.color, pinned: s.pinned, unread: s.unread, badge: s.unread > 99 ? '99+' : '' + s.unread, wide: s.unread > 9, last: s.lastText, time: store.fmtTime(s.lastTime, now) });
    });
    const total = all.reduce((a, s) => a + s.unread, 0);
    this.setData({ list, total, empty: list.length === 0 });
    wx.setNavigationBarTitle({ title: total > 0 ? '轻聊(' + total + ')' : '轻聊' });
    store.syncTabBadge(this, 0);
  },
  onSearch(e) { this.setData({ kw: e.detail.value }); this.load(); },
  clearKw() { this.setData({ kw: '' }); this.load(); },
  open(e) {
    if (this.skipTap) { this.skipTap = false; return; }
    wx.navigateTo({ url: '/pages/chat/chat?id=' + e.currentTarget.dataset.id });
  },
  readAll() {
    if (!this.data.total) return wx.showToast({ title: '没有未读消息', icon: 'none' });
    const all = store.sessions(); all.forEach((s) => { s.unread = 0; });
    store.saveSessions(all); this.load();
    wx.showToast({ title: '已全部标为已读', icon: 'none' });
  },
  // 长按只记下是哪一条，抬手（touchend）再弹菜单：本引擎里长按那根手指抬起时会被当成点在
  // 操作菜单的遮罩上，菜单刚弹出就以 fail cancel 关掉。抬手再弹在微信里效果一样
  // 另外：微信里绑了 longpress 且触发后不再派发 tap，本引擎抬手仍会派 tap —— 用 skipTap 吃掉那一次
  onTouchStart() { this.skipTap = false; },
  onLong(e) { this.longId = e.currentTarget.dataset.id; this.skipTap = true; },
  onTouchEnd() {
    if (!this.longId) return;
    const id = this.longId; this.longId = '';
    this.more(id);
  },
  more(id) {
    const all = store.sessions();
    const s = all.find((x) => x.id === id);
    if (!s) return;
    const items = [s.pinned ? '取消置顶' : '置顶聊天', s.unread > 0 ? '标为已读' : '标为未读', '删除该聊天'];
    wx.showActionSheet({
      itemList: items,
      success: (r) => {
        if (r.tapIndex === 0) { s.pinned = !s.pinned; store.saveSessions(all); this.load(); }
        else if (r.tapIndex === 1) { s.unread = s.unread > 0 ? 0 : 1; store.saveSessions(all); this.load(); }
        else if (r.tapIndex === 2) {
          wx.showModal({
            title: '删除聊天', content: '删除后，将清空该聊天的消息记录', confirmText: '删除', confirmColor: '#F53F3F',
            success: (m) => {
              if (!m.confirm) return;
              store.saveSessions(all.filter((x) => x.id !== id));
              wx.removeStorageSync('chat_msgs_' + id);
              this.load();
              wx.showToast({ title: '已删除', icon: 'none' });
            }
          });
        }
      }
    });
  }
});
