const store = require('../../utils/store.js');
const ORD = require('../../utils/order.js');
const T = require('../../utils/teams.js');
const HOTLINE = '400-826-3377';
Page({
  data: { contact: null, initial: '约', stats: {}, card: null, edit: false, name: '', phone: '', err: '' },
  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 3 });
    this.load();
  },
  load() {
    const c = store.contact();
    const orders = store.orders().map(ORD.view);
    const played = orders.filter((o) => o.st === 'used' || o.st === 'ended');
    const card = store.card();
    this.setData({
      contact: c, initial: c.name ? c.name.slice(0, 1) : '约',
      stats: {
        paid: orders.filter((o) => o.st === 'paid').length,
        hours: played.reduce((a, o) => a + o.hours, 0),
        teams: T.all().filter((t) => t.joined.indexOf(T.me()) >= 0).length,
        times: card.times.reduce((a, t) => a + t.left, 0)
      },
      card
    });
  },
  toOrders(e) { wx.navigateTo({ url: '/pages/orders/orders' + (e.currentTarget.dataset.tab ? '?tab=' + e.currentTarget.dataset.tab : '') }); },
  toTeams() { getApp().globalData.teamFilter = 'mine'; wx.switchTab({ url: '/pages/teams/teams' }); },
  toCard() { wx.switchTab({ url: '/pages/card/card' }); },
  // 编辑常用联系人：showModal 的 editable 在本引擎没有输入框，用页内底部面板
  openEdit() { this.setData({ edit: true, name: this.data.contact.name, phone: this.data.contact.phone, err: '' }); },
  closeEdit() { this.setData({ edit: false }); },
  onName(e) { this.setData({ name: e.detail.value, err: '' }); },
  onPhone(e) { this.setData({ phone: e.detail.value, err: '' }); },
  saveEdit() {
    const name = this.data.name.trim();
    if (name.length < 2) { this.setData({ err: '姓名至少 2 个字' }); return; }
    if (!/^1[3-9]\d{9}$/.test(this.data.phone)) { this.setData({ err: '手机号格式不对' }); return; }
    store.saveContact({ name, phone: this.data.phone });
    this.setData({ edit: false });
    this.load();
    wx.showToast({ title: '已保存', icon: 'success' });
  },
  call() {
    wx.makePhoneCall({
      phoneNumber: HOTLINE.replace(/-/g, ''),
      fail: () => wx.setClipboardData({ data: HOTLINE, success: () => wx.showToast({ title: '已复制客服电话 ' + HOTLINE, icon: 'none' }) })
    });
  },
  more() {
    wx.showActionSheet({
      itemList: ['订场规则', '清除本机数据'],
      success: (r) => {
        if (r.tapIndex === 0) {
          wx.showModal({ title: '订场规则', content: '场地按小时预订，同一场地连续时段自动合并计价；开场前 2 小时以上可免费取消；请提前 10 分钟凭入场码入场。', showCancel: false });
          return;
        }
        wx.showModal({
          title: '清除本机数据', content: '订单、会员卡、组局与联系人都会清空，不能恢复。', confirmColor: '#e5484d',
          success: (m) => { if (m.confirm) { store.clearAll(); this.load(); wx.showToast({ title: '已清除', icon: 'none' }); } }
        });
      }
    });
  },
  onShareAppMessage() { return { title: '约场：附近球馆空场一键订，约球组局凑人更快', path: '/pages/index/index' }; }
});
