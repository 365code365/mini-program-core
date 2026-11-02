const store = require('../../utils/store.js');
const GOALS = ['减脂塑形', '增肌增重', '保持健康', '提升体能'];
// 编辑资料：昵称、身高（算 BMI 用）、训练目标
Page({
  data: { name: '', height: '', goals: GOALS, goal: '' },
  onLoad() {
    const p = store.getProfile();
    this.setData({ name: p.name, height: String(p.height), goal: p.goal });
  },
  onName(e) { this.setData({ name: e.detail.value }); },
  onHeight(e) { this.setData({ height: e.detail.value }); },
  pickGoal(e) { this.setData({ goal: e.currentTarget.dataset.g }); },
  save() {
    const name = this.data.name.trim();
    const h = Number(this.data.height);
    if (name.length < 1 || name.length > 12) return wx.showToast({ title: '昵称 1~12 个字', icon: 'none' });
    if (!(h >= 120 && h <= 230)) return wx.showToast({ title: '身高请填 120~230 cm', icon: 'none' });
    store.saveProfile({ name, height: Math.round(h), goal: this.data.goal });
    wx.showToast({ title: '已保存', icon: 'success' });
    setTimeout(() => wx.navigateBack(), 500);
  }
});
