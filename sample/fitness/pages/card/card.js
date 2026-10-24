Page({
  data: {
    card: { name: '黑金年卡', level: 'LV.3 黑金会员', expire: '2025-08-31', leftDays: 0, usedPct: 0 },
    qrDots: [[44, 10], [56, 24], [44, 44], [62, 52], [36, 62], [80, 66], [52, 80], [70, 84], [86, 42], [20, 46]],
    perks: [
      { ch: '课', name: '团课无限约', desc: '每日 2 节' },
      { ch: '店', name: '全国通店', desc: '86 家门店' },
      { ch: '浴', name: '淋浴毛巾', desc: '免费使用' },
      { ch: '测', name: '体测报告', desc: '每月 1 次' },
      { ch: '教', name: '私教体验', desc: '赠 2 节' },
      { ch: '停', name: '免费停卡', desc: '30 天/年' },
      { ch: '友', name: '带友入场', desc: '每月 2 次' },
      { ch: '饮', name: '能量吧', desc: '饮品 8 折' }
    ],
    plans: [
      { name: '月卡', price: 299, per: '¥9.9/天', orig: 399, badge: '' },
      { name: '季卡', price: 799, per: '¥8.8/天', orig: 1197, badge: '热销' },
      { name: '黑金年卡', price: 2580, per: '¥7.0/天', orig: 3999, badge: '最划算' }
    ],
    planIdx: 2, autoRenew: false
  },
  onLoad() {
    const exp = wx.getStorageSync('fit_card_expire') || '2025-08-31';
    this.applyExpire(exp);
    this.setData({ autoRenew: !!wx.getStorageSync('fit_auto_renew') });
  },
  applyExpire(exp) {
    const p = exp.split('-').map(Number);
    const left = Math.max(0, Math.round((new Date(p[0], p[1] - 1, p[2]) - new Date()) / 86400000));
    this.setData({ 'card.expire': exp, 'card.leftDays': left, 'card.usedPct': Math.max(4, Math.min(100, 100 - Math.round(left / 365 * 100))) });
  },
  pickPlan(e) { this.setData({ planIdx: e.currentTarget.dataset.index }); },
  onAuto(e) { wx.setStorageSync('fit_auto_renew', e.detail.value); this.setData({ autoRenew: e.detail.value }); },
  showCode() { wx.showToast({ title: '入场码已刷新', icon: 'success' }); },
  pay() {
    const plan = this.data.plans[this.data.planIdx];
    wx.showModal({
      title: '确认支付', content: plan.name + ' ¥' + plan.price + '，有效期将在当前基础上顺延。',
      confirmText: '支付',
      success: (r) => {
        if (!r.confirm) return;
        const add = [30, 90, 365][this.data.planIdx];
        const p = this.data.card.expire.split('-').map(Number);
        const d = new Date(p[0], p[1] - 1, p[2] + add);
        const pad = v => (v < 10 ? '0' + v : '' + v);
        const exp = d.getFullYear() + '-' + pad(d.getMonth() + 1) + '-' + pad(d.getDate());
        wx.setStorageSync('fit_card_expire', exp);
        this.applyExpire(exp);
        wx.showToast({ title: '续费成功', icon: 'success' });
      }
    });
  }
});
