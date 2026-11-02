const store = require('../../utils/store');
const draft = require('../../utils/draft');
const guide = require('../../utils/guide');
const { getItem } = require('../../utils/items');

Page({
  data: { user: null, stat: {}, draft: null, favs: [], showFavs: false },
  onShow() {
    store.syncTab(this, 3);
    const u = store.USER;
    const favs = (wx.getStorageSync('gov_favs') || []).map(getItem).filter(Boolean).map(i => ({ id: i.id, name: i.name, dept: i.dept }));
    this.setData({
      user: { name: store.maskName(u.name), id: store.maskId(u.idcard), level: u.level },
      stat: {
        orders: store.getOrders().length,
        appoints: store.getAppoints().filter(a => a.status === 'booked').length,
        licenses: (wx.getStorageSync('gov_licenses') || ['L1', 'L2', 'L3']).length
      },
      favs
    });
    draft.load((d) => {
      const item = d && getItem(d.form.itemId);
      this.setData({ draft: d ? { name: item ? item.name : '申报事项', time: store.fmtTime(d.savedAt) } : null });
    });
  },
  goOrders() { wx.switchTab({ url: '/pages/orders/orders' }); },
  goAppoints() { getApp().globalData.orderTab = 'appoint'; wx.switchTab({ url: '/pages/orders/orders' }); },
  goLicense() { wx.switchTab({ url: '/pages/license/license' }); },
  goProgress() { wx.navigateTo({ url: '/pages/progress/progress' }); },
  openDraft() {
    if (!this.data.draft) { wx.showToast({ title: '草稿箱是空的', icon: 'none' }); return; }
    wx.navigateTo({ url: '/pages/apply/apply?draft=1' });
  },
  dropDraft() {
    draft.remove((ok) => { this.setData({ draft: null }); wx.showToast({ title: ok ? '草稿已删除' : '草稿不存在', icon: 'none' }); });
  },
  toggleFavs() { this.setData({ showFavs: !this.data.showFavs }); },
  openFav(e) { wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id }); },
  guide() { guide.open(''); },
  call() {
    wx.makePhoneCall({
      phoneNumber: '12345',
      fail: () => wx.showModal({ title: '12345 政务服务便民热线', content: '工作日 8:30-17:30 人工服务，其余时间可留言，工作人员将在 1 个工作日内回电。', showCancel: false })
    });
  },
  clearAll() {
    wx.showModal({
      title: '清除本地数据', content: '将清除办件记录、预约、收藏和申报草稿（演示数据会重新生成），确定吗？', confirmColor: '#dc2626',
      success: (r) => {
        if (!r.confirm) return;
        wx.clearStorageSync();
        draft.remove(() => wx.reLaunch({ url: '/pages/index/index' }));
      }
    });
  },
  about() {
    wx.showModal({ title: '关于云川办事', content: '云川办事 v2.6.0\n主办：云川市政务服务和数据管理局\n技术支持：云川数字政务科技有限公司', showCancel: false });
  }
});
