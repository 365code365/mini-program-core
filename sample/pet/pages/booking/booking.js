const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const host = require('../../utils/host.js');
const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];
const PICKUP_FEE = 20;
const NEWBIE_CUT = 30;
// 服务预约：?cat=care|med|stay 类别，?svc= 预选服务，?store= 门店，?pet= 宠物
// 一次只约一类服务（门店按类别筛）；下单即付款：微信支付，或用洗护次卡抵扣 1 次基础洗护
Page({
  data: {
    pets: [], petId: '', cats: data.cats, cat: 'care', svcs: [], stores: [], storeNames: [], storeIdx: 0,
    today: '', maxDate: '', date: '', week: '', time: '14:00', timeStart: '09:00', timeEnd: '20:00',
    nights: 1, pickup: false, remark: '', payBy: 'wechat', cardLeft: 0, newbie: false,
    base: 0, cardCut: 0, newbieCut: 0, pay: 0, totalMin: 0, hasS1: false
  },
  onLoad(q) {
    const now = new Date();
    const tomorrow = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
    const pets = store.getPets().map(p => ({ id: p.id, name: p.name, sx: p.name.charAt(0), color: p.color, kind: p.kind, weight: p.weight }));
    const cur = store.findPet(q.pet) || store.currentPet();
    const st = q.store ? data.findStore(q.store) : null;
    let cat = data.cats.some(c => c.key === q.cat) ? q.cat : (st ? st.cats[0] : 'care');
    if (st && st.cats.indexOf(cat) < 0) cat = st.cats[0];
    this.preSvc = q.svc || '';
    this.preStore = st ? st.id : '';
    this.setData({
      pets, petId: cur ? cur.id : '', today: store.dateStr(now.getTime()), maxDate: store.addDays(store.today(), 30),
      date: store.dateStr(tomorrow.getTime()), week: WEEK[tomorrow.getDay()],
      cardLeft: store.cardLeft(), newbie: store.isNewbie()
    });
    this.setCat(cat);
  },
  setCat(cat) {
    const svcs = data.services.filter(s => s.cat === cat).map((s, i) => Object.assign({}, s, { on: this.preSvc ? s.id === this.preSvc : (cat === 'stay' || i === 0) }));
    const stores = store.storesByArea().filter(s => s.cats.indexOf(cat) >= 0);
    const keep = this.preStore || (stores[this.data.storeIdx] || {}).id;
    const storeIdx = Math.max(0, stores.findIndex(s => s.id === keep));
    this.preSvc = '';
    this.setData({ cat, svcs, stores, storeNames: stores.map(s => s.name + ' · ' + s.dist), storeIdx, payBy: 'wechat' });
    this.applyStore();
  },
  applyStore() {
    const s = this.data.stores[this.data.storeIdx];
    let time = this.data.time;
    if (time < s.start) time = s.start;
    if (time > s.end) time = s.end;
    this.setData({ timeStart: s.start, timeEnd: s.end, time });
    this.calc();
  },
  pickCat(e) {
    this.preStore = '';
    this.setCat(e.currentTarget.dataset.k);
  },
  pickPet(e) { this.setData({ petId: e.currentTarget.dataset.id }); },
  toggleSvc(e) {
    const i = Number(e.currentTarget.dataset.index);
    if (this.data.cat === 'stay') return;
    this.setData({ ['svcs[' + i + '].on']: !this.data.svcs[i].on });
    this.calc();
  },
  calc() {
    const d = this.data;
    const on = d.svcs.filter(s => s.on);
    const base = on.reduce((a, s) => a + s.price * (s.perNight ? d.nights : 1), 0);
    const hasS1 = on.some(s => s.id === 's1');
    const payBy = hasS1 && d.cardLeft > 0 ? d.payBy : 'wechat';
    const cardCut = payBy === 'card' ? data.findService('s1').price : 0;
    const newbieCut = !cardCut && hasS1 && d.newbie ? NEWBIE_CUT : 0;
    this.setData({
      hasS1, payBy, base, cardCut, newbieCut,
      pay: store.round2(base + (d.pickup ? PICKUP_FEE : 0) - cardCut - newbieCut),
      totalMin: on.reduce((a, s) => a + s.min, 0)
    });
  },
  onStore(e) {
    this.setData({ storeIdx: Number(e.detail.value) });
    this.applyStore();
  },
  onDate(e) {
    const p = e.detail.value.split('-').map(Number);
    this.setData({ date: e.detail.value, week: WEEK[new Date(p[0], p[1] - 1, p[2]).getDay()] });
  },
  onTime(e) { this.setData({ time: e.detail.value }); },
  nightsDec() { if (this.data.nights > 1) { this.setData({ nights: this.data.nights - 1 }); this.calc(); } },
  nightsInc() { if (this.data.nights < 14) { this.setData({ nights: this.data.nights + 1 }); this.calc(); } },
  onPickup(e) { this.setData({ pickup: e.detail.value }); this.calc(); },
  onRemark(e) { this.setData({ remark: e.detail.value }); },
  pickPay(e) {
    const by = e.currentTarget.dataset.by;
    if (by === 'card' && !this.data.cardLeft) return wx.showToast({ title: '洗护卡次数已用完', icon: 'none' });
    this.setData({ payBy: by });
    this.calc();
  },
  goAddPet() { wx.navigateTo({ url: '/pages/pet-edit/pet-edit' }); },
  onShow() {
    // 从「添加宠物」回来：刷新可选宠物
    if (!this.data.today) return;
    const pets = store.getPets().map(p => ({ id: p.id, name: p.name, sx: p.name.charAt(0), color: p.color, kind: p.kind, weight: p.weight }));
    this.setData({ pets, petId: pets.some(p => p.id === this.data.petId) ? this.data.petId : (pets[0] ? pets[0].id : '') });
  },
  check() {
    const d = this.data;
    const pet = store.findPet(d.petId);
    const on = d.svcs.filter(s => s.on);
    if (!pet) return '请先选择宠物';
    if (!on.length) return '请至少选择一项服务';
    const s = d.stores[d.storeIdx];
    if (d.time < s.start || d.time > s.end) return s.name + ' 可预约 ' + s.start + '-' + s.end;
    const t = d.time.split(':').map(Number);
    const start = store.parseDate(d.date) + (t[0] * 60 + t[1]) * store.MIN;
    if (start < Date.now() + 30 * store.MIN) return '请至少提前 30 分钟预约';
    // 同一只宠物同一天同一类服务只约一次（洗完澡再寄养可以）
    const clash = store.getOrders().find(o => o.kind === 'service' && o.status === 'booked' && o.petId === pet.id && o.date === d.date && o.cat === d.cat);
    if (clash) return pet.name + ' ' + d.date.slice(5) + ' 已有' + clash.svcNames + '的预约';
    return '';
  },
  submit() {
    if (this.paying) return;
    const err = this.check();
    if (err) return wx.showToast({ title: err, icon: 'none' });
    const d = this.data;
    const pet = store.findPet(d.petId);
    // 寄养要疫苗在有效期内：过期了先去约疫苗
    if (d.cat === 'stay') {
      const bad = store.recordsOf(pet).find(r => r.type === '疫苗' && r.status === 'due');
      if (bad) {
        wx.showModal({
          title: '疫苗已过期',
          content: pet.name + '的「' + bad.name + '」已过期，寄养需要疫苗在有效期内，先预约接种吧',
          confirmText: '去接种',
          success: r => { if (r.confirm) wx.redirectTo({ url: '/pages/booking/booking?cat=med&svc=s6&pet=' + pet.id }); }
        });
        return;
      }
    }
    const on = d.svcs.filter(s => s.on);
    const s = d.stores[d.storeIdx];
    const order = {
      petId: pet.id, petName: pet.name, cat: d.cat, svcs: on.map(x => ({ id: x.id, name: x.name, price: x.price })),
      svcNames: on.map(x => x.name).join(' + '), storeId: s.id, storeName: s.name, date: d.date, week: d.week, time: d.time,
      nights: d.cat === 'stay' ? d.nights : 0, min: d.totalMin, pickup: d.pickup, pickupFee: d.pickup ? PICKUP_FEE : 0, remark: d.remark.trim(),
      base: d.base, cardUsed: d.payBy === 'card', cardCut: d.cardCut, newbieCut: d.newbieCut, pay: d.pay, payBy: d.pay > 0 ? 'wechat' : 'card'
    };
    const desc = pet.name + ' · ' + order.svcNames + '\n' + d.date + ' ' + d.week + ' ' + d.time + ' · ' + s.name;
    const finish = () => {
      const o = store.createServiceOrder(order);
      wx.showToast({ title: '预约成功', icon: 'success' });
      setTimeout(() => wx.redirectTo({ url: '/pages/order-detail/order-detail?id=' + o.id }), 600);
    };
    this.paying = true;
    const go = d.pay > 0
      ? host.pay(d.pay, desc)
      : new Promise(resolve => wx.showModal({ title: '使用洗护卡', content: desc + '\n抵扣 1 次基础洗护，剩余 ' + (d.cardLeft - 1) + ' 次', confirmText: '确认预约', success: r => resolve(!!r.confirm), fail: () => resolve(false) }));
    go.then(ok => {
      this.paying = false;
      if (ok) finish();
      else wx.showToast({ title: '已取消，预约未提交', icon: 'none' });
    });
  }
});
