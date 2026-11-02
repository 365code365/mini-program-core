// 面试邀约：接受（可加日历提醒）/ 拒绝（选原因），接受后能看路线、联系 HR
const S = require('../../utils/store.js');
const host = require('../../utils/host.js');
const REASONS = ['时间冲突，希望改期', '已接受其他 offer', '岗位与预期不符', '暂不考虑换工作'];
Page({
  data: { list: [], focus: '' },
  onLoad(q) {
    if (q.id) this.setData({ focus: q.id });
  },
  onShow() { this.refresh(); },
  refresh() {
    this.setData({ list: S.getInterviews() });
  },
  find(e) {
    return this.data.list.find(v => v.id === e.currentTarget.dataset.id);
  },
  onAccept(e) {
    const v = this.find(e);
    wx.showModal({
      title: '接受面试', content: v.co.name + ' · ' + v.title + '\n' + v.when + '\n' + v.mode, confirmText: '接受', confirmColor: '#2E6BFF',
      success: r => {
        if (!r.confirm) return;
        S.acceptInterview(v.id);
        this.refresh();
        wx.showToast({ title: '已接受，HR 会收到通知', icon: 'none' });
      }
    });
  },
  onDecline(e) {
    const v = this.find(e);
    wx.showActionSheet({
      itemList: REASONS,
      success: r => {
        S.declineInterview(v.id, REASONS[r.tapIndex]);
        this.refresh();
        wx.showToast({ title: '已婉拒', icon: 'none' });
      }
    });
  },
  onCalendar(e) {
    const v = this.find(e);
    host.addCalendar({ title: v.co.name + ' ' + v.mode, ts: v.ts, when: v.when, place: v.place, desc: v.title }).then(ok => {
      if (!ok) return;
      S.setReminder(v.id);
      this.refresh();
      wx.showToast({ title: '已设置提醒', icon: 'success' });
    });
  },
  onRoute(e) {
    const v = this.find(e);
    if (v.mode !== '现场面试') return wx.setClipboardData({ data: v.place, success: () => wx.showToast({ title: '会议号已复制', icon: 'none' }) });
    host.openMap({ lat: v.co.lat, lng: v.co.lng, name: v.co.name, addr: v.place });
  },
  onChat(e) {
    const v = this.find(e);
    wx.navigateTo({ url: '/pages/chat/chat?sid=' + v.session });
  },
  onGo() { wx.switchTab({ url: '/pages/index/index' }); }
});
