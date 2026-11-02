// 预约取号：网点 → 事项 → 日期（未来 7 天，周日不开放，周六仅市中心上午）→ 时段（半小时一档，余号按网点 / 日期 / 时段算出来）。
const { ITEMS, HALLS } = require('../../utils/items');
const store = require('../../utils/store');

const WEEK = ['日', '一', '二', '三', '四', '五', '六'];
const SLOTS = ['09:00', '09:30', '10:00', '10:30', '11:00', '11:30', '13:30', '14:00', '14:30', '15:00', '15:30', '16:00', '16:30'];

function pad(n) { return n < 10 ? '0' + n : '' + n; }
function seedLeft(s) { let h = 0; for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 997; return h % 9; }

Page({
  data: {
    halls: HALLS, hallIdx: 0,
    items: ITEMS.map(i => i.name), itemIdx: 0,
    dates: [], dateIdx: 0, slots: [], slotIdx: -1,
    ticket: null
  },
  onLoad(q) {
    const idx = q.item ? ITEMS.findIndex(i => i.id === q.item) : 0;
    this.setData({ itemIdx: idx < 0 ? 0 : idx });
    this.buildDates();
  },
  buildDates() {
    const hall = HALLS[this.data.hallIdx];
    const dates = [];
    const now = new Date();
    for (let k = 0; dates.length < 6 && k < 10; k++) {
      const d = new Date(now.getFullYear(), now.getMonth(), now.getDate() + k);
      const wd = d.getDay();
      if (wd === 0 || (wd === 6 && hall.id !== 'H1')) continue;
      dates.push({ key: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`, md: `${pad(d.getMonth() + 1)}-${pad(d.getDate())}`, wd: k === 0 ? '今天' : '周' + WEEK[wd], sat: wd === 6, today: k === 0 });
    }
    this.setData({ dates, dateIdx: 0 }, () => this.buildSlots());
  },
  buildSlots() {
    const hall = HALLS[this.data.hallIdx];
    const date = this.data.dates[this.data.dateIdx];
    const now = new Date();
    const mins = now.getHours() * 60 + now.getMinutes();
    const slots = SLOTS.filter(s => !(date.sat && s >= '12:00')).map(s => {
      const [h, m] = s.split(':').map(Number);
      const past = date.today && h * 60 + m <= mins + 30;
      const left = past ? 0 : seedLeft(hall.id + date.key + s);
      return { time: s, end: m === 30 ? pad(h + 1) + ':00' : pad(h) + ':30', left, off: left === 0, past };
    });
    this.setData({ slots, slotIdx: -1 });
  },
  pickHall(e) { this.setData({ hallIdx: e.currentTarget.dataset.i }, () => this.buildDates()); },
  pickItem(e) { this.setData({ itemIdx: Number(e.detail.value) }); },
  pickDate(e) { this.setData({ dateIdx: e.currentTarget.dataset.i }, () => this.buildSlots()); },
  pickSlot(e) {
    const s = this.data.slots[e.currentTarget.dataset.i];
    if (s.off) { wx.showToast({ title: s.past ? '该时段已过' : '该时段已约满', icon: 'none' }); return; }
    this.setData({ slotIdx: e.currentTarget.dataset.i });
  },
  submit() {
    const { hallIdx, itemIdx, dates, dateIdx, slots, slotIdx } = this.data;
    if (slotIdx < 0) { wx.showToast({ title: '请选择预约时段', icon: 'none' }); return; }
    const hall = HALLS[hallIdx], item = ITEMS[itemIdx], date = dates[dateIdx], slot = slots[slotIdx];
    const list = store.getAppoints();
    if (list.some(a => a.status === 'booked' && a.date === date.key && a.itemId === item.id)) {
      wx.showToast({ title: '同一事项同一天只能预约一次', icon: 'none' }); return;
    }
    const letter = 'ABCDEFG'['hukou social house biz traffic medical civil'.split(' ').indexOf(item.theme)] || 'A';
    const ticket = {
      id: 'AP' + Date.now(), no: letter + pad(10 + (list.length * 7 + slotIdx) % 89), status: 'booked',
      hall: hall.name, addr: hall.addr, itemId: item.id, item: item.name, date: date.key, wd: date.wd,
      time: `${slot.time}-${slot.end}`, user: store.maskName(store.USER.name)
    };
    list.unshift(ticket);
    store.saveAppoints(list);
    this.setData({ ticket, ['slots[' + slotIdx + '].left']: slot.left - 1 });
    wx.pageScrollTo({ scrollTop: 0, duration: 0 });
  },
  cancel() {
    wx.showModal({
      title: '取消预约', content: '取消后号源将释放给其他群众，当月取消超过 3 次将暂停预约资格 30 天。', confirmText: '确定取消',
      success: (r) => {
        if (!r.confirm) return;
        const list = store.getAppoints();
        const a = list.find(x => x.id === this.data.ticket.id);
        if (a) a.status = 'cancelled';
        store.saveAppoints(list);
        this.setData({ ticket: null });
        this.buildSlots();
        wx.showToast({ title: '已取消预约', icon: 'none' });
      }
    });
  },
  openMap() {
    const hall = HALLS[this.data.hallIdx];
    wx.openLocation({
      latitude: 25.04, longitude: 102.71, name: hall.name, address: hall.addr,
      fail: () => wx.showModal({ title: hall.name, content: `${hall.addr}\n电话：${hall.phone}\n${hall.time}`, showCancel: false })
    });
  },
  goOrders() { getApp().globalData.orderTab = 'appoint'; wx.switchTab({ url: '/pages/orders/orders' }); }
});
