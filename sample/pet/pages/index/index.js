const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
Page({
  data: {
    pets: [], petIdx: 0, canAdd: true, stores: [], area: '',
    entries: [
      { key: 'wash', name: '洗护', sub: '低至 ¥58', bg: 'linear-gradient(135deg, #7ec8ff, #3b9cf0)' },
      { key: 'stay', name: '寄养', sub: '¥89/晚起', bg: 'linear-gradient(135deg, #ffcf6b, #ffa53d)' },
      { key: 'med', name: '医疗', sub: '24h 急诊', bg: 'linear-gradient(135deg, #7fe0b0, #2fb57a)' },
      { key: 'shop', name: '商城', sub: '满 99 包邮', bg: 'linear-gradient(135deg, #ff9fb5, #f2557a)' }
    ]
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    store.tick();
    const pets = store.getPets().map(p => Object.assign({}, p, { sx: p.name.charAt(0), age: store.ageOf(p.birth), remind: store.remindOf(p) }));
    const canAdd = pets.length < store.MAX_PETS;
    this.setData({
      pets,
      canAdd,
      // 删了宠物之后 swiper 的 current 不能越界（末尾还有一张「添加」卡）
      petIdx: Math.min(this.data.petIdx, Math.max(0, pets.length - (canAdd ? 0 : 1))),
      area: data.areas[store.areaIdx()].name,
      stores: store.storesByArea().map(s => Object.assign({}, s, { short: s.name.split('· ')[1].slice(0, 2) }))
    });
  },
  onPet(e) { this.setData({ petIdx: e.detail.current }); },
  goPet(e) {
    store.setCurrent(e.currentTarget.dataset.id);
    wx.switchTab({ url: '/pages/pet/pet' });
  },
  addPet() { wx.navigateTo({ url: '/pages/pet-edit/pet-edit' }); },
  onEntry(e) {
    const k = e.currentTarget.dataset.key;
    if (k === 'shop') return this.goMall();
    if (k === 'med') {
      return wx.showActionSheet({
        itemList: ['预约到院就诊（门诊问诊）', '疫苗 / 驱虫 / 体检预约', '拨打 24h 急诊 ' + data.EMERGENCY],
        success: r => {
          if (r.tapIndex === 0) wx.navigateTo({ url: '/pages/booking/booking?cat=med&svc=s8' });
          else if (r.tapIndex === 1) wx.navigateTo({ url: '/pages/booking/booking?cat=med&svc=s6' });
          else host.call(data.EMERGENCY, '萌宠家宠物医院 · 24h 急诊');
        }
      });
    }
    if (k === 'stay') {
      return wx.showModal({
        title: '寄养服务',
        content: '独立猫舍 / 犬舍 ¥89 起每晚，含每日 2 次视频回传。寄养需疫苗在有效期内。',
        confirmText: '去预约',
        success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/booking/booking?cat=stay' }); }
      });
    }
    wx.navigateTo({ url: '/pages/booking/booking?cat=care&svc=s1' });
  },
  goBooking(e) { wx.navigateTo({ url: '/pages/booking/booking?store=' + e.currentTarget.dataset.id }); },
  goBookingDefault() { wx.navigateTo({ url: '/pages/booking/booking?cat=care&svc=s1' }); },
  openStore(e) { host.openMap(data.findStore(e.currentTarget.dataset.id)); },
  goMall() { wx.switchTab({ url: '/pages/mall/mall' }); },
  pickCity() {
    wx.showActionSheet({
      itemList: data.areas.map(a => a.name),
      success: r => {
        store.setArea(r.tapIndex);
        this.onShow();
        wx.showToast({ title: '已切换到' + data.areas[r.tapIndex].name + '，门店按距离重排', icon: 'none' });
      }
    });
  }
});
