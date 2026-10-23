// EventChannel 的另一头：onLoad 里注册监听（打开方的 success 在本页 onShow 之后才回，消息不会丢）
Page({
  data: { got: '还没收到', query: '' },
  onLoad(q) {
    this.setData({ query: JSON.stringify(q) });
    this.ch = this.getOpenerEventChannel();
    this.ch.on('fromOpener', (d) => this.setData({ got: JSON.stringify(d) }));
  },
  reply() {
    this.ch.emit('fromOpened', { reply: '收到了，我是 channel 页', at: Date.now() % 100000 });
    wx.navigateBack();
  }
});
