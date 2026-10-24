const store = require('../../utils/store.js');
const GENDER = { male: '男', female: '女', secret: '保密' };
Page({
  data: { p: {}, initial: '', genderText: '', stats: [], notify: true, savedTip: '' },
  onShow() {
    store.syncTabBadge(this, 2);
    this.refresh();
  },
  refresh() {
    const p = store.profile();
    const sessions = store.sessions();
    const msgs = sessions.reduce((a, s) => a + store.messages(s.id).length, 0);
    const n = wx.getStorageSync('chat_notify');
    this.setData({
      p, initial: p.nickname.slice(0, 1), genderText: GENDER[p.gender] || '保密',
      notify: n === '' ? true : !!n,
      stats: [
        { k: 'f', label: '好友', value: store.CONTACTS.length },
        { k: 's', label: '会话', value: sessions.length },
        { k: 'm', label: '消息', value: msgs },
        { k: 't', label: '星标', value: store.stars().length }
      ]
    });
  },
  edit() {
    wx.navigateTo({
      url: '/pages/profile/profile',
      events: {
        saved: (np) => {
          this.refresh();
          this.setData({ savedTip: '资料已更新：' + np.nickname });
          clearTimeout(this.tt);
          this.tt = setTimeout(() => this.setData({ savedTip: '' }), 2500);
        }
      }
    });
  },
  toggleNotify() {
    const v = !this.data.notify;
    wx.setStorageSync('chat_notify', v ? 1 : 0);
    this.setData({ notify: v });
  },
  clearAll() {
    wx.showModal({
      title: '清空聊天记录', content: '将删除所有会话与消息，且无法恢复', confirmText: '清空', confirmColor: '#F53F3F',
      success: (r) => {
        if (!r.confirm) return;
        store.sessions().forEach((s) => wx.removeStorageSync('chat_msgs_' + s.id));
        store.saveSessions([]);
        this.refresh();
        store.syncTabBadge(this, 2);
        wx.showToast({ title: '已清空', icon: 'success' });
      }
    });
  },
  reset() {
    wx.showModal({
      title: '恢复演示数据', content: '会话、消息与资料将恢复为初始状态',
      success: (r) => { if (r.confirm) { store.seed(); this.refresh(); store.syncTabBadge(this, 2); wx.showToast({ title: '已恢复', icon: 'none' }); } }
    });
  }
});
