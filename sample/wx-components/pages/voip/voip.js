// 实时音视频（试验）：<voip-room> 两种模式 + wx.joinVoIPChat / updateVoIPChatMuteConfig / exitVoIPChat。
// 房间要微信后台签名；在微信外由宿主接自己的通话后台，没接时如实回 fail
const app = getApp();
Page({
  data: { log: [], selfId: 'lab-self', peerId: 'lab-peer', position: 'front', roomErr: '', stateText: '未进房间', muted: false },
  onLoad() {
    // 房间里的人进出、谁在说话（宿主接了才会来）
    if (wx.onVoIPChatMembersChanged) wx.onVoIPChatMembersChanged((r) => app.note(this, `成员变了 ${JSON.stringify(r.openIdList)}`));
    if (wx.onVoIPChatSpeakersChanged) wx.onVoIPChatSpeakersChanged((r) => app.note(this, `在说话 ${JSON.stringify(r.openIdList)}`));
    if (wx.onVoIPChatInterrupted) wx.onVoIPChatInterrupted((r) => { this.setData({ stateText: `被打断：${r.errMsg}` }); app.note(this, `interrupted ${r.errCode}`); });
  },
  onUnload() { wx.exitVoIPChat({ fail() {} }); },
  onRoomErr(e) {
    this.setData({ roomErr: `binderror ${JSON.stringify(e.detail)}` });
    app.note(this, `voip-room binderror ${JSON.stringify(e.detail)}`);
  },
  switchPos() { this.setData({ position: this.data.position === 'front' ? 'back' : 'front' }); },
  join() {
    // 这几项在微信里由开发者服务器向微信后台换签名得到；这里是演示值，宿主没接通话后台时会回 fail
    wx.joinVoIPChat({
      roomType: 'video',
      signature: 'demo-signature',
      nonceStr: 'demo' + Date.now(),
      timeStamp: Math.floor(Date.now() / 1000),
      groupId: 'lab-room-1',
      muteConfig: { muteMicrophone: false, muteEarphone: false },
      success: (r) => {
        this.setData({ stateText: `已进房间：${(r.openIdList || []).join('、')}` });
        app.note(this, `joinVoIPChat ✓ ${JSON.stringify(r)}`);
      },
      fail: (err) => {
        this.setData({ stateText: err.errMsg });
        app.note(this, `joinVoIPChat ✗ ${err.errCode} ${err.errMsg}`);
      }
    });
  },
  mute() {
    const muted = !this.data.muted;
    wx.updateVoIPChatMuteConfig({
      muteConfig: { muteMicrophone: muted, muteEarphone: false },
      success: () => { this.setData({ muted }); app.note(this, `静音 ${muted}`); },
      fail: (err) => app.note(this, `updateVoIPChatMuteConfig ✗ ${err.errMsg}`)
    });
  },
  exit() {
    wx.exitVoIPChat({
      success: () => { this.setData({ stateText: '已退出' }); app.note(this, 'exitVoIPChat ✓'); },
      fail: (err) => app.note(this, `exitVoIPChat ✗ ${err.errMsg}`)
    });
  }
});
