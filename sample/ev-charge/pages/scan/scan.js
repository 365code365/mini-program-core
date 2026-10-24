const data = require('../../utils/data.js');

Page({
  data: { code: '', error: '', found: null, light: false, recent: ['XX0128-02', 'GW2210-03', 'XJ3301-05'], balance: '0.00' },
  onShow() {
    if (wx.getStorageSync('ev_session')) { wx.redirectTo({ url: '/pages/charging/charging' }); return; }
    this.setData({ balance: data.getBalance().toFixed(2) });
  },
  lookup(code) {
    const found = data.findTerminal(code);
    this.setData({ code, found, error: '' });
  },
  onInput(e) { this.lookup(e.detail.value.toUpperCase()); },
  onClear() { this.setData({ code: '', found: null, error: '' }); },
  onRecent(e) { this.lookup(e.currentTarget.dataset.code); },
  onLight() { this.setData({ light: !this.data.light }); },
  onScanTap() {
    // 演示环境没有摄像头：模拟识别到一个空闲终端
    wx.showLoading({ title: '识别中' });
    setTimeout(() => { wx.hideLoading(); this.lookup('XX0128-02'); wx.showToast({ title: '已识别 XX0128-02', icon: 'none' }); }, 600);
  },
  onStart() {
    const f = this.data.found;
    if (!this.data.code) { this.setData({ error: '请输入终端编号' }); return; }
    if (!f) { this.setData({ error: '未找到该终端，请核对桩身铭牌上的编号' }); return; }
    if (f.terminal.status !== 'idle') { this.setData({ error: f.terminal.status === 'busy' ? '该终端正在被使用' : '该终端维护中，请换一个' }); return; }
    if (data.getBalance() < 10) {
      wx.showModal({ title: '余额不足', content: '账户余额低于 10 元，请先充值', confirmText: '去充值', confirmColor: '#00B87A',
        success: (r) => { if (r.confirm) wx.switchTab({ url: '/pages/wallet/wallet' }); } });
      return;
    }
    wx.navigateTo({ url: '/pages/charging/charging?no=' + f.terminal.no });
  }
});
