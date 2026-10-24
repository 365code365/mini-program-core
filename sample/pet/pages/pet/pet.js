const data = require('../../utils/data.js');

Page({
  data: { pets: data.pets, pet: {}, filters: ['全部', '疫苗', '驱虫', '体检'], filter: '全部', records: [], weights: [], statusText: { ok: '正常', soon: '即将到期', due: '已到期' } },
  onShow() {
    const id = wx.getStorageSync('pet_current') || 'p1';
    this.load(id);
  },
  load(id) {
    const pet = data.findPet(id) || data.pets[0];
    const done = wx.getStorageSync('pet_done_' + pet.id) || [];
    this.all = pet.records.map(r => Object.assign({}, r, done.indexOf(r.name) >= 0 ? { status: 'ok' } : {}));
    const ws = pet.weights;
    const min = Math.min.apply(null, ws) * 0.9, max = Math.max.apply(null, ws);
    const months = ['5月', '6月', '7月', '8月', '9月', '10月'];
    this.setData({
      pet, weights: ws.map((v, i) => ({ m: months[i], v, h: Math.round(40 + (v - min) / (max - min) * 120) }))
    });
    this.applyFilter();
  },
  applyFilter() {
    const f = this.data.filter;
    // 先列需要处理的，再列正常的
    const order = { due: 0, soon: 1, ok: 2 };
    const records = this.all.filter(r => f === '全部' || r.type === f).sort((a, b) => order[a.status] - order[b.status]);
    this.setData({ records });
  },
  pickPet(e) {
    wx.setStorageSync('pet_current', e.currentTarget.dataset.id);
    this.load(e.currentTarget.dataset.id);
  },
  setFilter(e) { this.setData({ filter: e.currentTarget.dataset.f }); this.applyFilter(); },
  book() { wx.navigateTo({ url: '/pages/booking/booking?store=st3&svc=s4' }); },
  markDone(e) {
    const name = e.currentTarget.dataset.name;
    wx.showModal({
      title: '标记为已完成', content: '确认「' + name + '」已完成？系统将按周期自动生成下次提醒。',
      success: (r) => {
        if (!r.confirm) return;
        const key = 'pet_done_' + this.data.pet.id;
        const done = wx.getStorageSync(key) || [];
        done.push(name);
        wx.setStorageSync(key, done);
        this.load(this.data.pet.id);
        wx.showToast({ title: '已更新档案', icon: 'success' });
      }
    });
  },
  addPet() { wx.showToast({ title: '最多可添加 5 只宠物，敬请期待', icon: 'none' }); }
});
