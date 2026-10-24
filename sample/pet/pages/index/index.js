const data = require('../../utils/data.js');

function remindOf(p) {
  const r = p.records.filter(x => x.status !== 'ok').sort((a, b) => (a.next < b.next ? -1 : 1))[0];
  return r ? (r.status === 'due' ? '已到期：' : '即将到期：') + r.name + '（' + r.next + '）' : '健康档案已完善，各项提醒均正常';
}

Page({
  data: {
    pets: [], petIdx: 0, stores: [],
    entries: [
      { key: 'wash', name: '洗护', sub: '低至 ¥58', bg: 'linear-gradient(135deg, #7ec8ff, #3b9cf0)' },
      { key: 'stay', name: '寄养', sub: '¥89/晚起', bg: 'linear-gradient(135deg, #ffcf6b, #ffa53d)' },
      { key: 'med', name: '医疗', sub: '24h 在线', bg: 'linear-gradient(135deg, #7fe0b0, #2fb57a)' },
      { key: 'shop', name: '商城', sub: '满 99 包邮', bg: 'linear-gradient(135deg, #ff9fb5, #f2557a)' }
    ]
  },
  onLoad() {
    this.setData({
      pets: data.pets.map(p => Object.assign({}, p, { remindText: remindOf(p) })),
      stores: data.stores.map(s => Object.assign({}, s, { short: s.name.split('· ')[1].slice(0, 2) }))
    });
  },
  onPet(e) { this.setData({ petIdx: e.detail.current }); },
  goPet(e) {
    wx.setStorageSync('pet_current', e.currentTarget.dataset.id);
    wx.switchTab({ url: '/pages/pet/pet' });
  },
  onEntry(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'shop') return this.goMall();
    if (k === 'med') {
      return wx.showActionSheet({
        itemList: ['在线问诊（¥19 起）', '预约到院就诊', '拨打 24h 急诊 0571-8822 6120'],
        success: (r) => { if (r.tapIndex === 1) wx.navigateTo({ url: '/pages/booking/booking?store=st3&svc=s4' }); else wx.showToast({ title: '已为你接通值班医生', icon: 'none' }); }
      });
    }
    if (k === 'stay') {
      return wx.showModal({ title: '寄养服务', content: '独立猫舍 / 犬舍 ¥89 起每晚，含每日 2 次视频回传。需提供疫苗本。', confirmText: '去预约', success: (r) => { if (r.confirm) wx.navigateTo({ url: '/pages/booking/booking?store=st1&svc=stay' }); } });
    }
    wx.navigateTo({ url: '/pages/booking/booking?svc=s1' });
  },
  goBooking(e) { wx.navigateTo({ url: '/pages/booking/booking?store=' + e.currentTarget.dataset.id }); },
  goBookingDefault() { wx.navigateTo({ url: '/pages/booking/booking?svc=s1' }); },
  goMall() { wx.switchTab({ url: '/pages/mall/mall' }); },
  pickCity() { wx.showActionSheet({ itemList: ['杭州 · 西溪', '杭州 · 滨江', '杭州 · 钱江新城'], success: () => wx.showToast({ title: '已切换', icon: 'none' }) }); }
});
