const app = getApp();
Page({
  data: { shot: '', log: [], types: ['getPhoneNumber', 'getRealtimePhoneNumber', 'chooseAvatar', 'contact', 'openSetting', 'getUserInfo', 'feedback', 'launchApp', 'agreePrivacyAuthorization'] },
  onShareAppMessage(o) { app.note(this, `onShareAppMessage from=${o.from} target.id=${o.target && o.target.id}`); return { title: '组件实验室' }; },
  // 宿主没接这些能力时按官方的 errMsg 回 fail；接了就是宿主的结果
  onRes(e) { app.note(this, `${e.type} ${e.detail.errMsg || JSON.stringify(e.detail)}`); },
  onAdErr(e) { app.note(this, `ad binderror errCode=${e.detail.errCode}`); },
  onOaErr(e) { app.note(this, `official-account binderror status=${e.detail.status}`); },
  shoot() {
    this.createSelectorQuery().select('#snap').node().exec((res) => {
      const node = res[0] && res[0].node;
      if (!node) return app.note(this, '拿不到 snapshot 节点');
      node.takeSnapshot({ type: 'file', format: 'png', success: (r) => { this.setData({ shot: r.tempFilePath }); app.note(this, `takeSnapshot ok ${r.width}×${r.height}`); },
        fail: (e) => app.note(this, `takeSnapshot fail ${e.errMsg}`) });
    });
  },
  onNavRes(e) { app.note(this, `navigator ${e.type} ${e.detail.errMsg || ''}`); }
});
