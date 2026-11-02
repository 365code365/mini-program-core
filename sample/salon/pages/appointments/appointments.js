// 我的预约：待到店（改约 / 取消 / 到店核销）→ 已完成（评价）→ 已取消
const store = require('../../utils/store.js');
const TABS = [{ key: 'upcoming', name: '待到店' }, { key: 'done', name: '已完成' }, { key: 'cancelled', name: '已取消' }];
const REASONS = ['行程有变，改天再约', '想换一位发型师', '临时有事去不了', '价格考虑一下'];
Page({
  data: { tabs: TABS, tab: 'upcoming', list: [], counts: {}, rv: null, rvScore: 5, rvText: '', rvTags: [], tagPool: ['技术专业', '沟通耐心', '准时开始', '环境舒适', '不推销'] },
  onShow() { this.refresh(); },
  refresh() {
    const all = store.appts();
    const counts = {};
    all.forEach(a => { counts[a.status] = (counts[a.status] || 0) + 1; });
    const list = all.filter(a => a.status === this.data.tab).map(a => Object.assign({}, a, {
      rvLine: a.review ? [a.review.tags.join(' · '), a.review.text].filter(Boolean).join(' ') : ''
    }));
    this.setData({ counts, list });
  },
  switchTab(e) { this.setData({ tab: e.currentTarget.dataset.key }); this.refresh(); },
  reschedule(e) {
    const a = store.appt(e.currentTarget.dataset.id);
    if ((a.changes || 0) >= 2) { wx.showModal({ title: '无法改约', content: '每笔预约最多改约 2 次，如需调整请取消后重新预约。', showCancel: false }); return; }
    wx.navigateTo({ url: '/pages/booking/booking?reschedule=' + a.id });
  },
  cancel(e) {
    const id = e.currentTarget.dataset.id;
    wx.showActionSheet({
      itemList: REASONS,
      success: r => {
        const a = store.appt(id);
        store.updateAppt(id, { status: 'cancelled', cancelReason: REASONS[r.tapIndex], cancelledAt: store.nowText() });
        if (a.pay === 'card') {   // 储值卡支付的退回卡里
          const m = store.member();
          m.balance += a.payPrice;
          store.addRecord(m, 'refund', '取消预约退款 · ' + a.serviceName, a.payPrice);
          store.saveMember(m);
        }
        wx.showToast({ title: a.pay === 'card' ? '已取消，¥' + a.payPrice + ' 已退回储值卡' : '预约已取消', icon: 'none' });
        this.refresh();
      }
    });
  },
  // 演示：前台核销（真实场景由门店收银系统完成）
  checkin(e) {
    const id = e.currentTarget.dataset.id;
    const a = store.appt(id);
    wx.showModal({
      title: '到店核销', content: '请向前台出示核销码 ' + a.code + '。前台确认后服务完成并累计成长值。', confirmText: '已核销', confirmColor: '#c0846a',
      success: r => {
        if (!r.confirm) return;
        store.updateAppt(id, { status: 'done', doneAt: store.nowText() });
        const m = store.member();
        m.growth += a.payPrice;
        m.points += Math.floor(a.payPrice);
        store.addRecord(m, 'points', '消费积分 · ' + a.serviceName, Math.floor(a.payPrice));
        store.saveMember(m);
        wx.showToast({ title: '服务完成，成长值 +' + a.payPrice, icon: 'none' });
        this.refresh();
      }
    });
  },
  review(e) { this.setData({ rv: store.appt(e.currentTarget.dataset.id), rvScore: 5, rvText: '', rvTags: [] }); },
  closeReview() { this.setData({ rv: null }); },
  onScore(e) { this.setData({ rvScore: e.detail.value }); },
  onRvText(e) { this.setData({ rvText: e.detail.value }); },
  toggleTag(e) {
    const t = e.currentTarget.dataset.t, tags = this.data.rvTags.slice();
    const i = tags.indexOf(t);
    if (i >= 0) tags.splice(i, 1); else tags.push(t);
    this.setData({ rvTags: tags });
  },
  submitReview() {
    const d = this.data;
    if (d.rvText.trim().length < 5 && !d.rvTags.length) { wx.showToast({ title: '选个标签或写几句感受吧', icon: 'none' }); return; }
    store.updateAppt(d.rv.id, { review: { score: d.rvScore, text: d.rvText.trim(), tags: d.rvTags, time: store.nowText() } });
    const m = store.member();
    m.points += 20;
    store.addRecord(m, 'points', '评价奖励', 20);
    store.saveMember(m);
    this.setData({ rv: null });
    wx.showToast({ title: '感谢评价，积分 +20', icon: 'none' });
    this.refresh();
  },
  goBook() { wx.navigateTo({ url: '/pages/booking/booking' }); }
});
