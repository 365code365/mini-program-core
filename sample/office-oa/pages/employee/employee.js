const O = require('../../utils/org.js');
const S = require('../../utils/store.js');

Page({
  data: { e: null, peers: [], isMe: false, phoneMask: '', busy: '' },
  onLoad(q) {
    const e = O.emp(q.id) || O.employees[1];
    wx.setNavigationBarTitle({ title: e.name });
    const peers = O.employees.filter(x => x.dept === e.dept && x.id !== e.id).slice(0, 6);
    // 今天是否在会：按会议室预订里他组织的会算
    const today = S.ymd(new Date());
    const m = S.bookings().find(b => b.date === today && b.owner === e.id);
    this.setData({ e, peers, isMe: e.id === O.ME, busy: m ? `今天 ${m.time} 在 ${m.roomName} 开「${m.subject}」` : '' });
  },
  call() {
    const e = this.data.e;
    wx.makePhoneCall({
      phoneNumber: e.phone,
      fail: () => wx.showActionSheet({
        itemList: ['复制手机号 ' + e.phone, '复制座机分机 0571-8866' + e.ext],
        success: r => wx.setClipboardData({ data: r.tapIndex === 0 ? e.phone : '0571-8866' + e.ext })
      })
    });
  },
  copyMail() { wx.setClipboardData({ data: this.data.e.email }); },
  apply(e) { wx.navigateTo({ url: `/pages/apply/apply?type=${e.currentTarget.dataset.type}&approver=${this.data.e.id}` }); },
  goPeer(e) { wx.redirectTo({ url: '/pages/employee/employee?id=' + e.currentTarget.dataset.id }); }
});
