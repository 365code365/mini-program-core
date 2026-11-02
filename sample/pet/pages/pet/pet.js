const store = require('../../utils/store.js');
// 疫苗 / 驱虫 / 体检到期了去约哪项服务
const SVC_OF = { 疫苗: 's6', 驱虫: 's4', 体检: 's7' };
Page({
  data: {
    pets: [], pet: null, canAdd: true, remind: {},
    filters: ['全部', '疫苗', '驱虫', '体检'], filter: '全部', records: [], weights: [],
    statusText: { ok: '正常', soon: '即将到期', due: '已到期' }
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 1 });
    store.tick();
    this.load();
  },
  load() {
    const all = store.getPets();
    const pets = all.map(p => ({ id: p.id, name: p.name, sx: p.name.charAt(0), color: p.color }));
    const pet = store.currentPet();
    if (!pet) {
      this.all = [];
      this.setData({ pets, pet: null, canAdd: true, records: [], weights: [] });
      return;
    }
    const ws = (pet.weights || []).slice(-6);
    const vs = ws.map(w => w.v);
    const min = Math.min.apply(null, vs) * 0.9;
    const max = Math.max.apply(null, vs);
    this.all = store.recordsOf(pet);
    this.setData({
      pets,
      canAdd: all.length < store.MAX_PETS,
      pet: Object.assign({}, pet, { sx: pet.name.charAt(0), age: store.ageOf(pet.birth) }),
      remind: store.remindOf(pet),
      weights: ws.map(w => ({ k: w.date, m: Number(w.date.slice(5, 7)) + '/' + Number(w.date.slice(8)), v: w.v, h: max > min ? Math.round(40 + (w.v - min) / (max - min) * 120) : 100 }))
    });
    this.applyFilter();
  },
  applyFilter() {
    const f = this.data.filter;
    this.setData({ records: this.all.filter(r => f === '全部' || r.type === f) });
  },
  pickPet(e) {
    store.setCurrent(e.currentTarget.dataset.id);
    this.load();
  },
  setFilter(e) {
    this.setData({ filter: e.currentTarget.dataset.f });
    this.applyFilter();
  },
  addPet() {
    if (!this.data.canAdd) {
      wx.showToast({ title: '最多添加 ' + store.MAX_PETS + ' 只宠物', icon: 'none' });
      return;
    }
    wx.navigateTo({ url: '/pages/pet-edit/pet-edit' });
  },
  editPet() { wx.navigateTo({ url: '/pages/pet-edit/pet-edit?id=' + this.data.pet.id }); },
  addRecord() { wx.navigateTo({ url: '/pages/record-edit/record-edit?pet=' + this.data.pet.id + (this.data.filter !== '全部' ? '&type=' + this.data.filter : '') }); },
  addWeight() { wx.navigateTo({ url: '/pages/record-edit/record-edit?mode=weight&pet=' + this.data.pet.id }); },
  book(e) {
    const svc = SVC_OF[e.currentTarget.dataset.type] || 's8';
    wx.navigateTo({ url: '/pages/booking/booking?cat=med&svc=' + svc + '&pet=' + this.data.pet.id });
  },
  markDone(e) {
    const r = this.all.find(x => x.id === e.currentTarget.dataset.id);
    wx.showModal({
      title: '标记为已完成',
      content: '确认「' + r.name + '」今天已完成？下次提醒将顺延到 ' + r.cycle + ' 天后。',
      success: res => {
        if (!res.confirm) return;
        store.doneRecord(this.data.pet.id, r.id);
        this.load();
        wx.showToast({ title: '已更新档案', icon: 'success' });
      }
    });
  },
  // 点记录标题：完成 / 删除
  onRecord(e) {
    const r = this.all.find(x => x.id === e.currentTarget.dataset.id);
    wx.showActionSheet({
      itemList: ['标记为今天已完成', '删除这条记录'],
      success: res => {
        if (res.tapIndex === 0) {
          store.doneRecord(this.data.pet.id, r.id);
          this.load();
          wx.showToast({ title: '已更新档案', icon: 'success' });
          return;
        }
        wx.showModal({
          title: '删除记录',
          content: '删除「' + r.name + '」后不再提醒，确定吗？',
          confirmText: '删除',
          confirmColor: '#ff5b35',
          success: m => {
            if (!m.confirm) return;
            store.removeRecord(this.data.pet.id, r.id);
            this.load();
            wx.showToast({ title: '已删除', icon: 'none' });
          }
        });
      }
    });
  }
});
