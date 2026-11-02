const db = require('../../utils/db.js');
const { SERVICES, REDEEM } = require('../../utils/data.js');
const TABS = [{ key: 'unused', text: '可使用' }, { key: 'used', text: '已使用' }, { key: 'expired', text: '已过期' }];
function scope(c) {
  if (!c.cat) return '全品类通用';
  const s = SERVICES.find((x) => x.id === c.cat);
  return '限' + (s ? s.name : '指定服务') + '使用';
}
Page({
  data: { tabs: TABS, tab: 'unused', list: [], counts: {}, code: '', codeErr: '', loading: true },
  onShow() { this.load(); },
  load() {
    const all = db.getCoupons();
    const counts = {};
    all.forEach((c) => { counts[c.state] = (counts[c.state] || 0) + 1; });
    const list = all.filter((c) => c.state === this.data.tab)
      .map((c) => Object.assign({}, c, { scope: scope(c), cond: c.min ? '满 ' + c.min + ' 元可用' : '无门槛' }))
      .sort((a, b) => (a.expire < b.expire ? -1 : 1));
    this.setData({ list, counts, loading: false });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.k }); this.load(); },
  onCode(e) { this.setData({ code: e.detail.value, codeErr: '' }); },
  redeem() {
    const code = this.data.code.trim().toUpperCase();
    if (!code) { this.setData({ codeErr: '请输入兑换码' }); return; }
    if (!/^[A-Z0-9]{6,16}$/.test(code)) { this.setData({ codeErr: '兑换码是 6~16 位字母或数字' }); return; }
    const c = REDEEM[code];
    if (!c) { this.setData({ codeErr: '兑换码不存在或已失效' }); return; }
    if (!db.addCoupon(c)) { this.setData({ codeErr: '这个兑换码已经兑换过了' }); return; }
    this.setData({ code: '', tab: 'unused' });
    this.load();
    wx.showToast({ title: '兑换成功：' + c.name, icon: 'none' });
  },
  use(e) {
    const c = this.data.list.find((x) => x.id === e.currentTarget.dataset.id);
    // 指定品类的券直接进那个服务；通用券回首页挑
    if (c && c.cat) wx.navigateTo({ url: '/pages/service/service?id=' + c.cat });
    else wx.switchTab({ url: '/pages/index/index' });
  }
});
