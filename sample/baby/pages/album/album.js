const store = require('../../utils/store.js');
const T = require('../../utils/time.js');

const SAMPLES = ['/images/photos/p1.jpg', '/images/photos/p2.jpg', '/images/photos/p3.jpg', '/images/photos/p4.jpg', '/images/photos/p5.jpg', '/images/photos/p6.jpg'];

Page({
  data: { groups: [], count: 0, voiceCount: 0, name: '' },

  onShow() {
    this.getTabBar && this.getTabBar() && this.getTabBar().setData({ selected: 2 });
    this.load();
  },

  load() {
    const baby = store.profile();
    const list = store.list(store.K.moments).slice().sort((a, b) => (a.date < b.date ? 1 : -1));
    const groups = [];
    list.forEach((m) => {
      const key = m.date.slice(0, 7);
      let g = groups.find((x) => x.key === key);
      if (!g) {
        const [y, mo] = key.split('-');
        g = { key, title: y + '年' + Number(mo) + '月', age: T.age(baby.birthday, T.parse(m.date)).months + '个月', items: [] };
        groups.push(g);
      }
      const a = T.age(baby.birthday, T.parse(m.date));
      g.items.push({ id: m.id, photo: m.photo, caption: m.caption, tag: m.tag, day: Number(m.date.slice(8)) + '日', dayN: a.days + 1, voice: !!m.voice });
    });
    this.setData({ groups, count: list.length, voiceCount: list.filter((m) => m.voice).length, name: baby.name });
  },

  add() {
    wx.showActionSheet({
      itemList: ['拍照', '从手机相册选择', '使用示例照片'],
      success: (r) => {
        if (r.tapIndex === 0) return wx.navigateTo({ url: '/pages/shoot/shoot' });
        if (r.tapIndex === 1) return this.pick();
        this.useSample();
      }
    });
  },

  pick() {
    wx.chooseImage({
      count: 1, sizeType: ['compressed'], sourceType: ['album'],
      success: (res) => wx.navigateTo({ url: '/pages/shoot/shoot?photo=' + encodeURIComponent(res.tempFilePaths[0]) }),
      fail: () => {
        wx.showToast({ title: '没能打开相册，先用示例照片', icon: 'none' });
        this.useSample();
      }
    });
  },

  useSample() {
    const p = SAMPLES[this.data.count % SAMPLES.length];
    wx.navigateTo({ url: '/pages/shoot/shoot?photo=' + encodeURIComponent(p) });
  },

  open(e) { wx.navigateTo({ url: '/pages/moment/moment?id=' + e.currentTarget.dataset.id }); }
});
