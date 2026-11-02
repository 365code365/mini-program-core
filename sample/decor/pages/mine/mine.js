const D = require('../../utils/data.js');
const store = require('../../utils/store.js');

Page({
  data: { user: {}, favs: [], follows: [], budgets: [], projects: 0, layoutAt: '' },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 4 });
    this.load();
  },

  load() {
    const favs = store.get(store.K.favCases, []).map((id) => D.caseById(id)).filter(Boolean);
    const follows = store.get(store.K.favDesigners, []).map((id) => D.designer(id)).filter(Boolean);
    const budgets = store.get(store.K.budgets, []).slice(0, 3).map((b) => Object.assign({}, b, { time: store.ymdhm(b.ts) }));
    const layout = store.get(store.K.layout, null);
    const user = store.get(store.K.user, {});
    // WXML 表达式里不能调方法（与微信一致），首字与打码手机号在这里算好
    const phone = String(user.phone || '');
    this.setData({
      user, initial: user.nick ? user.nick.charAt(0) : '业',
      maskedPhone: phone.length === 11 ? phone.slice(0, 3) + '****' + phone.slice(7) : '',
      favs, follows, budgets,
      projects: store.projects().length, layoutAt: layout ? layout.savedAt : ''
    });
  },

  openCase(e) { wx.navigateTo({ url: '/pages/case/case?id=' + e.currentTarget.dataset.id }); },
  openDesigner(e) { wx.navigateTo({ url: '/pages/designer/designer?id=' + e.currentTarget.dataset.id }); },
  goDiary() { wx.switchTab({ url: '/pages/diary/diary' }); },
  goLayout() { wx.navigateTo({ url: '/pages/layout/layout' }); },
  goBudget() { wx.switchTab({ url: '/pages/budget/budget' }); },
  goIndex() { wx.switchTab({ url: '/pages/index/index' }); },

  service() {
    wx.showActionSheet({
      itemList: ['拨打客服 400-826-1688', '投诉与建议'],
      success: (r) => {
        if (r.tapIndex === 0) wx.makePhoneCall({ phoneNumber: '4008261688', fail: () => wx.showToast({ title: '客服时间 9:00-21:00', icon: 'none' }) });
        else wx.showModal({ title: '投诉与建议', content: '请发送邮件至 service@haozhai.example.cn，我们会在 1 个工作日内回复。', showCancel: false });
      }
    });
  },

  clear() {
    wx.showModal({
      title: '清除本机数据？', content: '收藏、户型方案、预算记录和预约项目都会删除', confirmText: '清除', confirmColor: '#d9534f',
      success: (r) => {
        if (!r.confirm) return;
        [store.K.favCases, store.K.favDesigners, store.K.projects, store.K.layout, store.K.budgets].forEach((k) => wx.removeStorageSync(k));
        wx.showToast({ title: '已清除', icon: 'none' });
        this.load();
      }
    });
  }
});
