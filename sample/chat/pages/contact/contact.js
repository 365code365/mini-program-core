const store = require('../../utils/store.js');
Page({
  data: { c: {}, star: false, moments: [], msgCount: 0 },
  onLoad(q) {
    const c = store.contact(q.id) || store.CONTACTS[0];
    this.id = c.id;
    const tones = ['#c9d4ff', '#ffd8bf', '#b7eb8f', '#ffd6e7'];
    this.setData({ c, moments: tones.map((t, i) => ({ k: i, color: t })) });
  },
  onShow() {
    this.setData({ star: store.stars().indexOf(this.id) >= 0, msgCount: store.messages(this.id).length });
  },
  toggleStar() {
    let s = store.stars();
    const on = s.indexOf(this.id) >= 0;
    s = on ? s.filter((x) => x !== this.id) : s.concat([this.id]);
    wx.setStorageSync('chat_star', s);
    this.setData({ star: !on });
    wx.showToast({ title: on ? '已取消星标' : '已设为星标朋友', icon: 'none' });
  },
  sendMsg() {
    // 聊天页已在栈里（从聊天页点头像进来的）就直接回去，否则打开对应会话
    const pages = getCurrentPages();
    const prev = pages[pages.length - 2];
    if (prev && prev.route === 'pages/chat/chat' && prev.id === this.id) return wx.navigateBack();
    wx.navigateTo({ url: '/pages/chat/chat?id=' + this.id });
  },
  call() {
    wx.showActionSheet({ itemList: ['视频通话', '语音通话'], success: (r) => {
      wx.showToast({ title: (r.tapIndex === 0 ? '视频' : '语音') + '通话暂不可用', icon: 'none' });
    } });
  },
  copyId() { wx.setClipboardData({ data: this.data.c.wxid }); }
});
