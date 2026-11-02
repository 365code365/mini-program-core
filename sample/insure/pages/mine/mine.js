// 我的：守护天数、保单 / 理赔汇总、理赔记录、推荐码、客服、清除本地数据
const store = require('../../utils/store.js');
const util = require('../../utils/util.js');
Page({
  data: { days: 1, stat: { policies: 0, active: 0, cover: '0' }, claims: [], referral: '', showRef: false, refInput: '', version: '' },
  onShow() {
    const tb = this.getTabBar && this.getTabBar();
    if (tb) tb.setData({ selected: 3 });
    const since = wx.getStorageSync('ins_since') || Date.now();
    const all = store.policies();
    const active = all.filter(p => store.statusOf(p).key === 'active');
    const cover = active.reduce((s, p) => s + Number(p.amount || 0), 0);
    let info = {};
    try { info = wx.getAccountInfoSync().miniProgram || {}; } catch (e) { info = {}; }
    this.setData({
      days: Math.max(1, Math.ceil((Date.now() - since) / 86400000)),
      stat: { policies: all.filter(p => p.status !== 'cancelled').length, active: active.length, cover: util.amountText(cover) },
      claims: store.claims().map(c => store.claimView(c)),
      referral: store.profile().referral || '', version: info.version || '1.0.0'
    });
  },
  openClaim(e) { wx.navigateTo({ url: '/pages/claim/claim?claim=' + e.currentTarget.dataset.id }); },
  goPolicies() { wx.switchTab({ url: '/pages/policies/policies' }); },
  // 推荐码：优先用可输入的 showModal；宿主不支持 editable（res.content 拿不到）时改用页内输入框
  editReferral() {
    wx.showModal({
      title: '填写推荐码', editable: true, placeholderText: '6 位推荐码（选填）', content: this.data.referral,
      success: r => {
        if (!r.confirm) return;
        if (typeof r.content !== 'string') { this.setData({ showRef: true, refInput: this.data.referral }); return; }
        this.saveReferral(r.content);
      }
    });
  },
  onRefInput(e) { this.setData({ refInput: e.detail.value }); },
  closeRef() { this.setData({ showRef: false }); },
  confirmRef() { this.saveReferral(this.data.refInput); this.setData({ showRef: false }); },
  saveReferral(v) {
    v = String(v || '').trim().toUpperCase();
    if (v && !/^[A-Z0-9]{6}$/.test(v)) { wx.showToast({ title: '推荐码是 6 位字母或数字', icon: 'none' }); return; }
    store.saveProfile({ referral: v });
    this.setData({ referral: v });
    wx.showToast({ title: v ? '推荐码已保存' : '已清除推荐码', icon: 'none' });
  },
  call() {
    const phone = '400-820-6688';
    wx.makePhoneCall({ phoneNumber: phone, fail: () => wx.showModal({ title: '客服热线', content: phone + '（7×24 小时）', confirmText: '复制号码', success: r => { if (r.confirm) wx.setClipboardData({ data: phone }); } }) });
  },
  clear() {
    wx.showModal({
      title: '清除本地数据', content: '将删除本机上的全部保单、理赔记录与设置，此操作不可恢复。', confirmText: '清除', confirmColor: '#f0524a',
      success: r => { if (r.confirm) { store.clearAll(); wx.showToast({ title: '已清除', icon: 'success' }); this.onShow(); } }
    });
  },
  about() {
    wx.showModal({ title: '关于安心保', content: '安心保 v' + this.data.version + '\n本应用为演示示例，产品与保费均为模拟数据，不构成真实投保。', showCancel: false });
  }
});
