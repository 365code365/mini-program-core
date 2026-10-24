const S = require('../../utils/store.js');
const M = require('../../utils/money.js');

Page({
  data: { balText: '', holdText: '', holds: [], pwd: {}, pwdText: '', nopwd: false, recentCount: 0 },

  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 3 });
    this.load();
  },

  load() {
    const accts = S.accounts();
    const hs = S.holdings();
    const pwd = S.pwdState();
    this.setData({
      balText: M.fmt(accts.reduce((s, a) => s + a.balance, 0)),
      holdText: M.fmt(hs.reduce((s, h) => s + h.amount, 0)),
      holds: hs.map((h, i) => ({
        key: h.time + '-' + i, pid: h.pid, name: h.name, amtText: M.fmt(h.amount), incText: M.fmt(h.income),
        rateText: h.rate.toFixed(2), termText: h.days === 1 ? '随存随取' : h.days + '天', dateText: M.dateStr(h.time).slice(5)
      })),
      pwd,
      pwdText: pwd.locked ? '已锁定：连续输错 3 次' : pwd.errors ? '今日已输错 ' + pwd.errors + ' 次，还可尝试 ' + (3 - pwd.errors) + ' 次' : '正常',
      nopwd: !!S.get('nopwd', false),
      recentCount: S.payees().length
    });
  },

  unlock() {
    const pwd = this.data.pwd;
    if (!pwd.locked && !pwd.errors) return wx.showToast({ title: '支付密码状态正常', icon: 'none' });
    wx.showModal({
      title: pwd.locked ? '解除锁定' : '重置错误次数',
      content: '演示环境：已通过人脸核验，确认后可重新使用支付密码',
      confirmColor: '#D7263D',
      success: r => {
        if (!r.confirm) return;
        S.set('pwd', { errors: 0, locked: false });
        this.load();
        wx.showToast({ title: '已解除', icon: 'success' });
      }
    });
  },
  onNopwd(e) { S.set('nopwd', e.detail.value); this.setData({ nopwd: e.detail.value }); },
  goPayees() { wx.navigateTo({ url: '/pages/payees/payees?mode=go' }); },
  goWealth() { wx.switchTab({ url: '/pages/wealth/wealth' }); },
  openProduct(e) { wx.navigateTo({ url: '/pages/product/product?id=' + e.currentTarget.dataset.pid }); },
  resetData() {
    wx.showModal({
      title: '恢复演示数据',
      content: '余额、交易、持仓、收款人与密码状态都会恢复成初始值',
      confirmColor: '#D7263D',
      success: r => {
        if (!r.confirm) return;
        S.resetAll();
        this.load();
        wx.showToast({ title: '已恢复', icon: 'success' });
      }
    });
  }
});
