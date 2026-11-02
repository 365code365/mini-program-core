// 我的：头部资料（可改昵称）、余额 / 券 / 行程 / 可开票四个数（都能点）、会员等级（按完成的行程数）、
// 常用地址 / 紧急联系人 / 发票 / 客服中心、夜间自动分享开关。数字全部来自 store
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const ICON = {
  coupons: 'linear-gradient(135deg, #FFB067, #FF7A1A)', trips: 'linear-gradient(135deg, #6BA8FF, #3D7BFF)',
  address: 'linear-gradient(135deg, #5FD99A, #19BE6B)', contacts: 'linear-gradient(135deg, #FF8A8A, #F5455C)',
  invoice: 'linear-gradient(135deg, #B28CFF, #7B5CFF)', service: 'linear-gradient(135deg, #7ED6E8, #26A8C7)'
};
// 客服中心里的常见问题：有去处的给一个按钮
const FAQ_GO = ['/pages/trips/trips', '/pages/trips/trips', '/pages/trips/trips', '/pages/invoice/invoice'];

Page({
  data: {
    nick: '', phoneMask: '', level: {}, done: 0, balance: '0.00', couponCount: 0, tripCount: 0, invoiceAmount: 0,
    menus: [], autoShare: true, profile: false, nickInput: '', formSeq: 0, tabGap: 0
  },
  onLoad() {
    // 同首页：本引擎 tab 页的 windowHeight 含 tabBar 区域，底部面板要自己让出 tabBar 的高度
    const info = wx.getWindowInfo();
    const inset = info.safeArea ? Math.max(0, info.screenHeight - info.safeArea.bottom) : 0;
    this.setData({ tabGap: info.screenHeight - info.windowHeight > 90 ? 0 : 50 + inset });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 2 });
    this.load();
  },
  load() {
    const st = store.stats();
    const p = store.getProfile();
    const coupons = store.validCoupons().length;
    const inv = store.invoiceable();
    const contacts = store.getContacts().length;
    const balance = store.getWallet().balance;
    this.setData({
      nick: p.nick, initial: p.nick.charAt(0), phoneMask: store.maskPhone(p.phone), level: store.levelOf(st.done), done: st.done,
      balance: balance.toFixed(2), couponCount: coupons, tripCount: st.count,
      invoiceAmount: store.round2(inv.reduce((s, t) => s + t.pay, 0)),
      autoShare: store.autoShare(),
      menus: [
        { key: 'coupons', name: '我的优惠券', glyph: '券', extra: coupons + ' 张可用', url: '/pages/coupons/coupons' },
        { key: 'trips', name: '我的行程', glyph: '程', extra: st.count + ' 个行程', tab: '/pages/trips/trips' },
        { key: 'address', name: '常用地址', glyph: '址', extra: store.placesText(), url: '/pages/address/address' },
        { key: 'contacts', name: '紧急联系人', glyph: '安', extra: contacts ? contacts + ' 位' : '未添加', url: '/pages/contacts/contacts' },
        { key: 'invoice', name: '电子发票', glyph: '票', extra: inv.length ? inv.length + ' 个行程可开票' : '', url: '/pages/invoice/invoice' },
        { key: 'service', name: '客服中心', glyph: '服', extra: '7×24 小时' }
      ].map(m => Object.assign(m, { bg: ICON[m.key] }))
    });
  },
  onMenu(e) {
    const m = this.data.menus[Number(e.currentTarget.dataset.idx)];
    if (m.url) wx.navigateTo({ url: m.url });
    else if (m.tab) wx.switchTab({ url: m.tab });
    else this.onService();
  },
  onWallet() {
    wx.navigateTo({ url: '/pages/wallet/wallet' });
  },
  onCoupons() {
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  },
  onTrips() {
    wx.switchTab({ url: '/pages/trips/trips' });
  },
  onInvoice() {
    wx.navigateTo({ url: '/pages/invoice/invoice' });
  },
  onLevel() {
    const lv = this.data.level;
    wx.showModal({
      title: '滴达会员 · ' + lv.name,
      content: '会员等级按完成的行程数计算：白银 0 单、黄金 4 单、铂金 10 单、钻石 30 单。你已完成 ' + this.data.done + ' 单' +
        (lv.top ? '，已是最高等级。' : '，再完成 ' + lv.need + ' 单升级' + lv.next + '。'),
      showCancel: false,
      confirmText: '知道了'
    });
  },
  // 客服中心：常见问题 + 拨打客服电话（宿主不能拨号时弹号码）
  onService() {
    const items = data.FAQS.map(f => f.q).concat('拨打客服电话 ' + data.SERVICE_PHONE);
    wx.showActionSheet({
      itemList: items,
      success: r => {
        const f = data.FAQS[r.tapIndex];
        if (!f) {
          host.call(data.SERVICE_PHONE, '滴达客服（7×24 小时）');
          return;
        }
        const go = FAQ_GO[r.tapIndex];
        wx.showModal({
          title: f.q,
          content: f.a,
          confirmText: go.indexOf('invoice') > 0 ? '去开票' : '查看行程',
          cancelText: '知道了',
          success: m => {
            if (!m.confirm) return;
            if (go.indexOf('trips') > 0) wx.switchTab({ url: go });
            else wx.navigateTo({ url: go });
          }
        });
      }
    });
  },
  onShare(e) {
    store.setAutoShare(e.detail.value);
    this.setData({ autoShare: !!e.detail.value });
    wx.showToast({ title: e.detail.value ? '夜间上车后自动分享行程' : '已关闭夜间自动分享', icon: 'none' });
  },
  // ───────── 个人资料 ─────────
  onProfile() {
    this.setData({ profile: true, nickInput: this.data.nick, formSeq: this.data.formSeq + 1 });
  },
  onCloseProfile() {
    this.setData({ profile: false });
  },
  noop() {},
  onNick(e) {
    this.setData({ nickInput: e.detail.value });
  },
  onSaveProfile() {
    const nick = this.data.nickInput.trim();
    if (!nick) {
      wx.showToast({ title: '昵称不能为空', icon: 'none' });
      return;
    }
    store.setProfile({ nick });
    this.setData({ profile: false });
    this.load();
    wx.showToast({ title: '已保存', icon: 'success' });
  }
});
