// 在线预约：选项目 → 选发型师（横向滚动，选中的自动滚到可视区）→ 选日期时段（嵌入式 picker-view）→ 确认面板（draggable-sheet）
// 带 ?reschedule=<预约 id> 进来是改约：预填原来的项目 / 发型师 / 时间，提交时改原单而不是新建
const data = require('../../utils/data.js');
const store = require('../../utils/store.js');
const slotsUtil = require('../../utils/slots.js');

const TMPL_REMIND = 'Kq3xVt8sLpYbM2nRw7GfJd1HcE5uZa9oT4iWkQ6yBvN'; // 预约成功 / 开始前 2 小时提醒

Page({
  data: {
    mode: 'new',
    cats: data.categories.filter(c => c.id !== 'all'),
    cat: 'cut',
    services: [],
    serviceId: '',
    svc: null,
    stylists: [],
    stylistId: '',
    stylistView: '',
    days: [],
    slots: [],
    pv: [0, 0],
    slotState: '',
    slotText: '',
    endText: '',
    sheetOpen: false,
    sheetSize: 0,
    form: { name: '', phone: '', remark: '' },
    pay: 'store',
    balance: 0,
    level: null,
    cardPrice: 0,
    errors: {}
  },

  onLoad(q) {
    const m = store.member();
    const lv = store.levelOf(m.growth).cur;
    const p = store.profile();
    let serviceId = q.serviceId || '', stylistId = q.stylistId || '', date = '', time = '';
    if (q.reschedule) {
      const a = store.appt(q.reschedule);
      if (a) {
        this.rescheduleId = a.id;
        serviceId = a.serviceId; stylistId = a.stylistId; date = a.date; time = a.time;
        p.name = a.contact; p.phone = a.phone;
        this.setData({ mode: 'reschedule', 'form.remark': a.remark || '', pay: a.pay });
        wx.setNavigationBarTitle({ title: '修改预约时间' });
      }
    }
    // 只带了发型师：项目分类默认取 TA 的第一个擅长分类
    let cat = 'cut';
    if (serviceId && data.service(serviceId)) cat = data.service(serviceId).cat;
    else if (stylistId && data.stylist(stylistId)) cat = data.stylist(stylistId).cats[0];
    this.setData({ days: slotsUtil.days(7), balance: m.balance, level: lv, 'form.name': p.name, 'form.phone': p.phone });
    this.pickCat(cat, serviceId, stylistId);
    if (date) {
      const di = Math.max(0, this.data.days.findIndex(d => d.date === date));
      this.refreshSlots(di, time);
    }
  },

  // ---------- 项目 ----------
  pickCat(cat, serviceId, stylistId) {
    const services = data.services.filter(s => s.cat === cat);
    const sid = services.some(s => s.id === serviceId) ? serviceId : services[0].id;
    this.setData({ cat, services });
    this.pickService(sid, stylistId);
  },
  onCat(e) { this.pickCat(e.currentTarget.dataset.id, '', this.data.stylistId); },
  onService(e) { this.pickService(e.currentTarget.dataset.id, this.data.stylistId); },
  pickService(serviceId, keepStylist) {
    const svc = data.service(serviceId);
    const stylists = [{ id: 'any', name: '到店分配', title: '由门店安排', avatar: '/images/avatar/me.jpg', ok: true }]
      .concat(data.stylists.map(s => Object.assign({}, s, { ok: s.cats.indexOf(svc.cat) >= 0 })));
    // 原来选的发型师不做这个项目：自动换成第一个能做的
    let sid = keepStylist;
    const cur = stylists.find(s => s.id === sid);
    if (!cur || !cur.ok) sid = (stylists.find(s => s.ok && s.id !== 'any') || stylists[0]).id;
    this.setData({ serviceId, svc, stylists, cardPrice: Math.round(svc.price * this.data.level.discount) });
    this.pickStylist(sid);
  },

  // ---------- 发型师：scroll-into-view 让选中的那张卡滚到可视区 ----------
  onStylist(e) {
    const s = this.data.stylists.find(x => x.id === e.currentTarget.dataset.id);
    if (!s.ok) {
      wx.showToast({ title: s.name + ' 不提供「' + this.data.svc.name + '」', icon: 'none' });
      return;
    }
    this.pickStylist(s.id);
  },
  pickStylist(id) {
    const s = this.data.stylists.find(x => x.id === id);
    this.setData({ stylistId: id, stylistView: 'sty-' + id, styText: id === 'any' ? '到店分配（由门店安排空闲发型师）' : s.name + ' · ' + s.title });
    this.refreshSlots(this.data.pv[0], this.data.slots.length ? this.data.slots[this.data.pv[1]].time : '');
  },

  // ---------- 日期时段：picker-view 两列（日期、时段），bindchange 的 value 是 number 数组 ----------
  refreshSlots(dayIdx, keepTime) {
    const day = this.data.days[dayIdx];
    const sid = this.data.stylistId === 'any' ? '' : this.data.stylistId;
    const slots = slotsUtil.slots(sid, day.date, this.rescheduleId);
    let si = keepTime ? slots.findIndex(s => s.time === keepTime) : -1;
    if (si < 0 || slots[si].state !== 'free') si = slotsUtil.firstFree(slots);
    this.setData({ slots, pv: [dayIdx, si] });
    this.describeSlot();
  },
  onPv(e) {
    const v = e.detail.value; // [日期下标, 时段下标]，都是 number
    if (v[0] !== this.data.pv[0]) {
      this.refreshSlots(v[0], this.data.slots[v[1]] ? this.data.slots[v[1]].time : '');
      return;
    }
    this.setData({ pv: [v[0], Math.min(v[1], this.data.slots.length - 1)] });
    this.describeSlot();
  },
  describeSlot() {
    const day = this.data.days[this.data.pv[0]];
    const slot = this.data.slots[this.data.pv[1]];
    if (!slot) return;
    const [h, m] = slot.time.split(':').map(Number);
    const end = h * 60 + m + this.data.svc.duration;
    const endText = `${store.pad(Math.floor(end / 60) % 24)}:${store.pad(end % 60)}`;
    const msg = {
      free: `${day.text} ${slot.time} 可预约，预计 ${endText} 结束`,
      full: '该时段已约满，请换一个时间',
      past: '该时段已过，请选择更晚的时间',
      mine: '你在这个时段已有一个预约'
    }[slot.state];
    this.setData({ slotState: slot.state, slotText: msg, endText });
  },

  // ---------- 确认面板 ----------
  openSheet() {
    if (this.data.slotState !== 'free') {
      wx.showToast({ title: this.data.slotText, icon: 'none' });
      return;
    }
    this.setData({ sheetOpen: true, errors: {} });
  },
  closeSheet() { this.setData({ sheetOpen: false }); },
  // 面板内部点空白：收起键盘，不让这次点击冒泡到外层遮罩把面板关掉
  onPanelTap() { wx.hideKeyboard && wx.hideKeyboard(); },
  onSheetSize(e) { this.setData({ sheetSize: Math.round(e.detail.size * 100) }); },
  // DraggableSheetContext.scrollTo：展开 / 收起
  toggleExpand() {
    const sheet = this.selectComponent('#sheet');
    const full = this.data.sheetSize >= 90;
    if (sheet && sheet.scrollTo) sheet.scrollTo({ size: full ? 0.7 : 0.92, animated: true, duration: 250 });
  },
  onField(e) {
    const k = e.currentTarget.dataset.k;
    this.setData({ ['form.' + k]: e.detail.value, ['errors.' + k]: '' });
  },
  onPay(e) {
    const v = e.detail.value;
    if (v === 'card' && this.data.balance < this.data.cardPrice) {
      this.setData({ pay: 'store' });
      wx.showModal({
        title: '储值卡余额不足', content: `当前余额 ¥${this.data.balance}，本单需 ¥${this.data.cardPrice}。去充值？`,
        confirmText: '去充值', success: r => { if (r.confirm) wx.navigateTo({ url: '/pages/member/member' }); }
      });
      return;
    }
    this.setData({ pay: v });
  },
  onShow() {
    // 从会员卡页充值回来：刷新余额
    if (this.data.level) this.setData({ balance: store.member().balance });
  },
  validate() {
    const f = this.data.form, errors = {};
    if (!f.name.trim()) errors.name = '请填写到店人姓名';
    if (!/^1[3-9]\d{9}$/.test(f.phone)) errors.phone = '请输入 11 位手机号';
    this.setData({ errors });
    return !Object.keys(errors).length;
  },
  submit() {
    if (!this.validate()) return;
    // 订阅消息必须在点击回调里同步发起；不管同不同意，都接着下单
    wx.requestSubscribeMessage({
      tmplIds: [TMPL_REMIND],
      success: res => this.place(res[TMPL_REMIND] === 'accept'),
      fail: () => this.place(false)
    });
  },
  place(subscribed) {
    if (this._placing) return;
    this._placing = true;
    const d = this.data, day = d.days[d.pv[0]], slot = d.slots[d.pv[1]];
    const sty = d.stylists.find(s => s.id === d.stylistId);
    const patch = {
      serviceId: d.svc.id, serviceName: d.svc.name, cat: d.svc.cat, duration: d.svc.duration,
      stylistId: d.stylistId, stylistName: sty.name, stylistAvatar: sty.avatar,
      date: day.date, dateText: day.text, time: slot.time, endTime: d.endText,
      price: d.svc.price, payPrice: d.pay === 'card' ? d.cardPrice : d.svc.price, pay: d.pay,
      contact: d.form.name.trim(), phone: d.form.phone, remark: d.form.remark.trim(), subscribed
    };
    store.saveProfile({ name: patch.contact, phone: patch.phone });
    wx.showLoading({ title: '正在提交', mask: true });
    setTimeout(() => {
      wx.hideLoading();
      let id;
      if (this.rescheduleId) {
        const old = store.appt(this.rescheduleId);
        store.updateAppt(this.rescheduleId, Object.assign(patch, { changes: (old.changes || 0) + 1 }));
        id = this.rescheduleId;
      } else {
        id = 'A' + Date.now().toString().slice(-8);
        store.addAppt(Object.assign(patch, { id, status: 'upcoming', code: String(1000 + Math.floor(Math.random() * 9000)),
          createdAt: store.nowText(), review: null }));
      }
      this._placing = false;
      wx.redirectTo({ url: `/pages/result/result?id=${id}&sub=${subscribed ? 1 : 0}&mode=${this.data.mode}` });
    }, 500);
  }
});
