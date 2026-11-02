// 隐私设置：简历是否公开、隐藏手机号、消息通知、屏蔽公司（被屏蔽公司的职位不在首页出现）
const data = require('../../utils/data.js');
const S = require('../../utils/store.js');
Page({
  data: { open: true, hidePhone: false, notify: true, cos: [] },
  onShow() { this.refresh(); },
  refresh() {
    const st = S.getSettings();
    this.setData({
      open: S.getResume().open !== false,
      hidePhone: st.hidePhone,
      notify: st.notify,
      cos: Object.keys(data.companies).map(k => Object.assign({}, data.companies[k], { blocked: st.blocked.indexOf(k) >= 0 }))
    });
  },
  onOpen(e) {
    const r = S.getResume();
    r.open = e.detail.value;
    S.saveResume(r);
    this.setData({ open: r.open });
    wx.showToast({ title: r.open ? '简历已公开，HR 可以主动联系你' : '简历已隐藏，只有你投递的公司能看到', icon: 'none' });
  },
  onHidePhone(e) {
    S.saveSettings({ hidePhone: e.detail.value });
    this.setData({ hidePhone: e.detail.value });
  },
  onNotify(e) {
    S.saveSettings({ notify: e.detail.value });
    this.setData({ notify: e.detail.value });
  },
  onBlock(e) {
    const id = e.currentTarget.dataset.id;
    const blocked = S.getSettings().blocked.filter(x => x !== id);
    if (e.detail.value) blocked.push(id);
    S.saveSettings({ blocked });
    this.refresh();
  }
});
