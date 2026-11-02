// 我的：昵称（点名字可改，wx.getStorage / setStorage 的 Promise 写法读写资料）、订单状态入口、
// 处方记录、用药提醒、收货地址、药师热线、清除浏览缓存（不动订单与处方）
const S = require('../../utils/store.js');
const F = require('../../utils/fly.js');

const PROFILE = 'ph_profile';

Page({
  data: { profile: { nick: '康源会员', level: '普通会员' }, editing: false, nickInput: '', counts: {}, rx: [], reminders: 0, addr: null },
  onShow() {
    F.syncTab(this, 3, S.cartCount());
    wx.getStorage({ key: PROFILE })
      .then(r => this.setData({ profile: r.data }))
      .catch(() => wx.setStorage({ key: PROFILE, data: this.data.profile }));
    const orders = S.get('orders', []);
    if (S.advanceOrders(orders)) S.set('orders', orders);
    const c = s => orders.filter(o => o.status === s).length;
    this.setData({
      counts: { w: c('待发货'), s: c('配送中'), d: c('已完成'), all: orders.length },
      rx: S.get('rx', []).slice(0, 3).map(r => Object.assign({}, r, { date: new Date(r.at).toLocaleDateString(), valid: Date.now() - r.at < 7 * 86400000 })),
      reminders: S.get('reminders', []).filter(r => r.on).length,
      addr: S.get('addr', null)
    });
  },
  editNick() { this.setData({ editing: true, nickInput: this.data.profile.nick }); },
  onNick(e) { this.setData({ nickInput: e.detail.value }); },
  saveNick() {
    const nick = this.data.nickInput.trim();
    if (nick.length < 2 || nick.length > 12) { wx.showToast({ title: '昵称 2~12 个字', icon: 'none' }); return; }
    const profile = Object.assign({}, this.data.profile, { nick });
    wx.setStorage({ key: PROFILE, data: profile })
      .then(() => { this.setData({ profile, editing: false }); wx.showToast({ title: '已保存', icon: 'success' }); })
      .catch(err => wx.showToast({ title: '保存失败：' + err.errMsg, icon: 'none' }));
  },
  cancelNick() { this.setData({ editing: false }); },
  goOrders(e) { wx.navigateTo({ url: '/pages/orders/orders?status=' + (e.currentTarget.dataset.s || '') }); },
  goRemind() { wx.navigateTo({ url: '/pages/remind/remind' }); },
  goRx() { wx.navigateTo({ url: '/pages/rx/rx' }); },
  editAddr() {
    if (!this.data.addr) { wx.showToast({ title: '下单时填写的地址会保存在这里', icon: 'none' }); return; }
    wx.showModal({
      title: '收货地址', content: this.data.addr.name + ' ' + this.data.addr.phone + '\n' + this.data.addr.region.join('') + ' ' + this.data.addr.detail,
      confirmText: '删除地址', cancelText: '关闭', confirmColor: '#f04a3e',
      success: r => { if (r.confirm) { wx.removeStorageSync(S.K.addr); this.setData({ addr: null }); } }
    });
  },
  call() {
    wx.makePhoneCall({
      phoneNumber: '0571-87001234',
      fail: () => wx.showModal({
        title: '药师热线', content: '0571-87001234（每天 8:00-22:00，执业药师免费咨询）', confirmText: '复制号码',
        success: r => { if (r.confirm) wx.setClipboardData({ data: '0571-87001234' }); }
      })
    });
  },
  clearCache() {
    const info = wx.getStorageInfoSync();
    wx.showModal({
      title: '清除缓存', content: '将清除搜索历史与用药打卡记录（当前占用 ' + info.currentSize + 'KB），订单、处方与购物车保留。',
      confirmText: '清除', confirmColor: '#f04a3e',
      success: r => {
        if (!r.confirm) return;
        wx.batchSetStorageSync([{ key: S.K.search, value: [] }, { key: S.K.taken, value: {} }]);
        wx.showToast({ title: '已清除', icon: 'success' });
      }
    });
  }
});
