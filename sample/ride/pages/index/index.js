// 叫车首页：选车型 / 起终点 → 预估价（自动用最省的券，券页「去使用」点过的优先）→ 呼叫。
// 预约用车、接送机、代叫车、预估明细都是页内底部面板；有进行中的订单时每秒走一拍（状态机在 utils/order.js），
// 顶部横条实时显示进度，行程走完变成「待支付」横条。
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const order = require('../../utils/order.js');
const host = require('../../utils/host.js');

const SERVICES = [
  { key: 'book', name: '预约用车', glyph: '约', bg: 'linear-gradient(135deg, #6BA8FF, #3D7BFF)' },
  { key: 'hub', name: '接送机', glyph: '机', bg: 'linear-gradient(135deg, #5FD99A, #19BE6B)' },
  { key: 'proxy', name: '代叫车', glyph: '代', bg: 'linear-gradient(135deg, #B28CFF, #7B5CFF)' },
  { key: 'safe', name: '安全中心', glyph: '安', bg: 'linear-gradient(135deg, #FF8A8A, #F5455C)' }
];
const TITLES = { book: '选择出发时间', hub: '接送机 · 接送站', proxy: '帮谁叫车', fare: '预估价明细' };
const PHONE = /^1\d{10}$/;

Page({
  data: {
    types: [], carType: 'express', typeName: '快车', callText: '呼叫快车',
    from: null, to: null,
    km: 0, min: 0, fare: null, price: 0, cut: 0, couponName: '',
    when: 0, whenText: '', passenger: null,
    services: SERVICES, claimed: false, recording: true,
    ongoing: null, unpaid: null, tabGap: 0,
    sheet: '', sheetTitle: '', slotDays: [], slotDay: 0, hubs: [], hubMode: 'to',
    contacts: [], pName: '', pPhone: '', formSeq: 0, fareLines: [], rules: []
  },
  onLoad() {
    // 微信里 tab 页的 windowHeight 已经扣掉了 tabBar，fixed 面板的 bottom:0 贴在 tabBar 上沿；
    // 本引擎 windowHeight 含 tabBar 区域，面板会被 tabBar 盖住：没扣就自己让出 tabBar 的高度（同 coffee 的菜单页）
    const info = wx.getWindowInfo();
    const inset = info.safeArea ? Math.max(0, info.screenHeight - info.safeArea.bottom) : 0;
    this.setData({ tabGap: info.screenHeight - info.windowHeight > 90 ? 0 : 50 + inset });
  },
  onShow() {
    if (typeof this.getTabBar === 'function' && this.getTabBar()) this.getTabBar().setData({ selected: 0 });
    this.setData({
      carType: store.getType(), from: store.getFrom(), to: store.getTo(),
      claimed: store.weekendClaimed(), recording: store.recording()
    });
    this.recalc();
    this.refreshOrder();
    this.startTicker();
  },
  onHide() {
    this.stopTicker();
  },
  onUnload() {
    this.stopTicker();
  },
  // ───────── 进行中的订单 ─────────
  startTicker() {
    this.stopTicker();
    if (order.get()) this.timer = setInterval(() => this.tick(), 1000);
  },
  stopTicker() {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  },
  tick() {
    const r = order.step();
    if (r.ev === 'accepted') wx.showToast({ title: '司机已接单，正在赶来', icon: 'none' });
    else if (r.ev === 'depart') wx.showToast({ title: '预约的司机已出发，正在赶来', icon: 'none' });
    else if (r.ev === 'arrived') wx.showToast({ title: '司机已到达上车点', icon: 'none' });
    else if (r.ev === 'finished') {
      this.stopTicker();
      wx.showToast({ title: '行程已结束，请支付车费', icon: 'none' });
      this.setData({ to: null });
      this.recalc();
    }
    this.refreshOrder();
  },
  refreshOrder() {
    const o = order.get();
    const u = store.unpaidTrip();
    this.setData({
      ongoing: o ? order.banner(o) : null,
      unpaid: u ? { id: u.id, due: store.dueOf(u), text: u.cancelled ? '取消费待支付' : '有一笔行程待支付' } : null
    });
  },
  onOngoing() {
    wx.navigateTo({ url: '/pages/calling/calling' });
  },
  onUnpaid() {
    wx.navigateTo({ url: '/pages/pay/pay?id=' + this.data.unpaid.id });
  },
  // ───────── 预估价 ─────────
  recalc() {
    const d = this.data;
    const cur = data.carType(d.carType);
    const callText = (d.when ? '预约' : d.passenger ? '帮' + d.passenger.name + '叫' : '呼叫') + cur.name;
    if (!d.to) {
      this.setData({ types: data.CAR_TYPES.map(t => ({ key: t.key, name: t.name, sub: t.desc })), typeName: cur.name, callText, fare: null });
      return;
    }
    const k = data.km(d.from, d.to);
    const min = data.minutes(k);
    const at = d.when || Date.now();
    const est = key => {
      const f = data.fare(key, k, min);
      const c = store.pickFor({ typeKey: key, from: d.from, to: d.to, amount: f.total, at });
      return { f, c, pay: store.round1(f.total - c.cut) };
    };
    const e = est(cur.key);
    this.setData({
      types: data.CAR_TYPES.map(t => ({ key: t.key, name: t.name, sub: '约 ¥' + est(t.key).pay })),
      typeName: cur.name, callText, km: k, min, fare: e.f, price: e.pay, cut: e.c.cut,
      couponName: e.c.coupon ? e.c.coupon.name : ''
    });
  },
  onType(e) {
    const key = e.currentTarget.dataset.key;
    store.setType(key);
    this.setData({ carType: key });
    this.recalc();
  },
  onPick(e) {
    wx.navigateTo({ url: '/pages/place/place?type=' + e.currentTarget.dataset.type });
  },
  onClearTo() {
    store.setTo(null);
    this.setData({ to: null });
    this.recalc();
  },
  onLocate() {
    host.locate().then(ok => {
      const p = store.place(data.ORIGIN);
      store.setFrom(p);
      this.setData({ from: p });
      this.recalc();
      wx.showToast({ title: (ok ? '已定位，推荐在 ' : '没拿到定位，已切回推荐上车点 ') + p.name, icon: 'none' });
    });
  },
  onCall() {
    const d = this.data;
    if (d.unpaid) {
      wx.showModal({
        title: '还有未支付的订单',
        content: d.unpaid.text + ' ¥' + d.unpaid.due + '，支付后才能继续叫车',
        confirmText: '去支付',
        success: r => {
          if (r.confirm) this.onUnpaid();
        }
      });
      return;
    }
    if (order.get()) {
      this.onOngoing();
      return;
    }
    if (!d.to) {
      wx.showToast({ title: '请先选择目的地', icon: 'none' });
      return;
    }
    if (d.to.name === d.from.name) {
      wx.showToast({ title: '上车点和目的地不能相同', icon: 'none' });
      return;
    }
    if (d.when && d.when - Date.now() < 15 * 60000) {
      this.setData({ when: 0, whenText: '' });
      this.recalc();
      wx.showToast({ title: '预约时间已临近，请重新选择出发时间', icon: 'none' });
      return;
    }
    order.create({ typeKey: d.carType, from: d.from, to: d.to, passenger: d.passenger, when: d.when });
    // 预约时间、乘车人只管这一单
    this.setData({ when: 0, whenText: '', passenger: null });
    wx.navigateTo({ url: '/pages/calling/calling' });
  },
  // ───────── 更多出行 / 安全 / 活动 ─────────
  onService(e) {
    const key = e.currentTarget.dataset.key;
    if (key === 'book') this.openBook();
    else if (key === 'hub') this.openHub();
    else if (key === 'proxy') this.openProxy();
    else this.onSafety();
  },
  onSafety() {
    wx.navigateTo({ url: '/pages/contacts/contacts' });
  },
  onBanner() {
    if (this.data.claimed) {
      wx.navigateTo({ url: '/pages/coupons/coupons' });
      return;
    }
    const got = store.claimWeekend();
    this.setData({ claimed: true });
    this.recalc();
    if (got) wx.showToast({ title: '已领取 3 张周末快车券，叫快车自动抵扣', icon: 'none' });
  },
  // ───────── 底部面板 ─────────
  open(sheet, patch) {
    this.setData(Object.assign({ sheet, sheetTitle: TITLES[sheet] }, patch || {}));
  },
  onCloseSheet() {
    this.setData({ sheet: '' });
  },
  noop() {},
  // 预约：今天 / 明天的半点档
  openBook() {
    const days = order.slots().map(d => ({ name: d.name, list: d.list.map(ts => ({ ts, text: store.hm(ts) })) }));
    const when = this.data.when;
    const slotDay = (when && days[1].list.some(x => x.ts === when)) || !days[0].list.length ? 1 : 0;
    this.open('book', { slotDays: days, slotDay });
  },
  onSlotDay(e) {
    this.setData({ slotDay: Number(e.currentTarget.dataset.i) });
  },
  onNow() {
    this.setData({ when: 0, whenText: '', sheet: '' });
    this.recalc();
  },
  onSlot(e) {
    const ts = Number(e.currentTarget.dataset.ts);
    this.setData({ when: ts, whenText: order.whenText(ts), sheet: '' });
    this.recalc();
    if (!this.data.to) wx.navigateTo({ url: '/pages/place/place?type=to' });
  },
  // 接送机：送机 = 终点设成机场 / 车站；接机 = 上车点设成机场 / 车站，再去选目的地
  openHub() {
    const d = this.data;
    const hubs = data.HUBS.map(h => {
      const k = data.km(d.from, h);
      const f = data.fare(d.carType, k, data.minutes(k));
      const c = store.pickFor({ typeKey: d.carType, from: d.from, to: h, amount: f.total, at: d.when || Date.now() });
      return { name: h.name, addr: h.addr, airport: !!h.airport, km: k, price: store.round1(f.total - c.cut) };
    });
    this.open('hub', { hubs, hubMode: 'to' });
  },
  onHubMode(e) {
    this.setData({ hubMode: e.currentTarget.dataset.mode });
  },
  onHub(e) {
    const p = store.place(data.HUBS[Number(e.currentTarget.dataset.i)]);
    store.pushHistory(p);
    if (this.data.hubMode === 'to') {
      if (p.name === this.data.from.name) {
        wx.showToast({ title: '上车点和目的地不能相同', icon: 'none' });
        return;
      }
      store.setTo(p);
      this.setData({ to: p, sheet: '' });
      this.recalc();
      return;
    }
    store.setFrom(p);
    const keep = this.data.to && this.data.to.name !== p.name ? this.data.to : null;
    if (!keep) store.setTo(null);
    this.setData({ from: p, to: keep, sheet: '' });
    this.recalc();
    if (!keep) wx.navigateTo({ url: '/pages/place/place?type=to' });
  },
  // 代叫车：本人 / 紧急联系人 / 手填一位
  openProxy() {
    const p = this.data.passenger;
    this.open('proxy', {
      contacts: store.getContacts().map(c => ({ name: c.name, phone: c.phone, relation: c.relation, mask: store.maskPhone(c.phone) })),
      pName: p ? p.name : '',
      pPhone: p ? p.phone : '',
      formSeq: this.data.formSeq + 1
    });
  },
  onSelf() {
    this.setData({ passenger: null, sheet: '' });
    this.recalc();
  },
  onPickContact(e) {
    const c = this.data.contacts[Number(e.currentTarget.dataset.i)];
    this.setPassenger(c.name, c.phone);
  },
  onPName(e) {
    this.setData({ pName: e.detail.value });
  },
  onPPhone(e) {
    this.setData({ pPhone: e.detail.value });
  },
  onImportContact() {
    host.contact().then(c => {
      if (c) this.setData({ pName: c.name, pPhone: c.phone, formSeq: this.data.formSeq + 1 });
    });
  },
  onProxyOk() {
    const name = this.data.pName.trim();
    const phone = String(this.data.pPhone).trim();
    if (!name) {
      wx.showToast({ title: '请填写乘车人姓名', icon: 'none' });
      return;
    }
    if (!PHONE.test(phone)) {
      wx.showToast({ title: '请填写乘车人的 11 位手机号', icon: 'none' });
      return;
    }
    this.setPassenger(name, phone);
  },
  setPassenger(name, phone) {
    this.setData({ passenger: { name, phone }, sheet: '' });
    this.recalc();
    wx.showToast({ title: '将为 ' + name + ' 叫车，司机信息会短信通知 TA', icon: 'none' });
  },
  // 预估明细：起步 / 里程 / 时长 / 远途 / 拼车折扣 / 券
  onOpenFare() {
    const d = this.data;
    if (!d.fare) return;
    const f = d.fare;
    const lines = [
      { name: '起步价', value: '¥' + f.base },
      { name: '里程费（' + d.km + ' 公里）', value: '¥' + f.mileage },
      { name: '时长费（' + d.min + ' 分钟）', value: '¥' + f.time }
    ];
    if (f.far) lines.push({ name: '远途费（超出 ' + data.FAR_KM + ' 公里部分）', value: '¥' + f.far });
    if (f.discount) lines.push({ name: '拼成 82 折', value: '-¥' + f.discount, cut: true });
    if (d.cut) lines.push({ name: d.couponName, value: '-¥' + d.cut, cut: true });
    // 计价规则一条一行：整段排的话引擎换行不避头，句末的「。」会单独落到下一行行首
    this.open('fare', { fareLines: lines, rules: data.ruleText(d.carType).replace(/。$/, '').split('；') });
  },
  onCoupons() {
    this.setData({ sheet: '' });
    wx.navigateTo({ url: '/pages/coupons/coupons' });
  }
});
