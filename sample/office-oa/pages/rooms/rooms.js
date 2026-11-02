// 会议室预订：左列会议室，右边横向滚动的时间轴网格（09:00-18:00，每格 30 分钟）。
// 点空闲格开始选，再点同一行的另一格把中间连成一段（中间有被占的格就重新起选）；点已选的格清空。
const S = require('../../utils/store.js');
const ask = require('../../utils/ask.js');

const WEEK = ['周日', '周一', '周二', '周三', '周四', '周五', '周六'];

Page({
  data: { days: [], date: '', rooms: S.rooms, slots: S.SLOTS, rows: [], sel: null, selText: '', mine: [], dlg: { show: false } },
  onLoad() {
    const days = [];
    for (let i = 0; i < 5; i++) {
      const d = new Date(); d.setDate(d.getDate() + i);
      days.push({ key: S.ymd(d), top: i === 0 ? '今天' : (i === 1 ? '明天' : WEEK[d.getDay()]), md: `${d.getMonth() + 1}/${d.getDate()}` });
    }
    this.setData({ days, date: days[0].key });
    this.build();
  },
  build() {
    const all = S.bookings();
    const day = all.filter(b => b.date === this.data.date);
    const isToday = this.data.date === S.ymd(new Date());
    const nowHm = S.hm(new Date());
    const sel = this.data.sel;
    const rows = S.rooms.map(r => ({
      room: r,
      cells: S.SLOTS.map((t, i) => {
        const b = day.find(x => x.room === r.id && i >= x.start && i < x.end);
        let st = 'free', label = '';
        if (b) { st = b.mine ? 'mine' : 'busy'; label = i === b.start ? b.subject : ''; }
        else if (isToday && S.slotEnd(i + 1) <= nowHm) st = 'past';
        if (sel && sel.room === r.id && i >= sel.start && i < sel.end) st = 'sel';
        return { i, st, label, bid: b ? b.id : '' };
      })
    }));
    const mine = all.filter(b => b.mine && b.date >= S.ymd(new Date())).sort((a, b) => (a.date + a.start < b.date + b.start ? -1 : 1));
    this.setData({ rows, mine, selText: sel ? `${S.rooms.find(r => r.id === sel.room).name}（${sel.room}） ${S.SLOTS[sel.start]}-${S.slotEnd(sel.end)}` : '' });
  },
  pickDay(e) { this.setData({ date: e.currentTarget.dataset.key, sel: null }); this.build(); },
  tapCell(e) {
    const { room, i } = e.currentTarget.dataset;
    const row = this.data.rows.find(r => r.room.id === room);
    const c = row.cells[i];
    if (c.st === 'busy' || c.st === 'mine') {
      const b = S.bookings().find(x => x.id === c.bid);
      return wx.showToast({ title: `${b.time} ${b.subject}（${b.mine ? '我' : b.ownerName}）`, icon: 'none' });
    }
    if (c.st === 'past') return wx.showToast({ title: '这个时段已经过去了', icon: 'none' });
    let sel = this.data.sel;
    if (c.st === 'sel') sel = null;
    else if (!sel || sel.room !== room) sel = { room, start: i, end: i + 1 };
    else {
      const start = Math.min(sel.start, i), end = Math.max(sel.end, i + 1);
      const ok = row.cells.slice(start, end).every(x => x.st === 'free' || x.st === 'sel');
      sel = ok ? { room, start, end } : { room, start: i, end: i + 1 };
    }
    this.setData({ sel });
    this.build();
  },
  async book() {
    const sel = this.data.sel;
    if (!sel) return wx.showToast({ title: '先在网格里选时段', icon: 'none' });
    const subject = await ask.text(this, { title: '会议主题', placeholder: '如：V3.2 需求评审', required: true });
    if (!subject) return;
    const rec = S.addBooking({ room: sel.room, date: this.data.date, start: sel.start, end: sel.end, subject });
    if (!rec) return wx.showToast({ title: '时段刚被别人订走了', icon: 'none' });
    this.setData({ sel: null });
    this.build();
    wx.showToast({ title: '预订成功', icon: 'success' });
  },
  onDlgOk(e) { ask.ok(this, e); },
  onDlgCancel() { ask.cancel(this); },
  cancel(e) {
    const b = this.data.mine.find(x => x.id === e.currentTarget.dataset.id);
    wx.showModal({
      title: '取消预订', content: `${b.date} ${b.time} ${b.roomName}「${b.subject}」`, confirmText: '取消预订', cancelText: '保留', confirmColor: '#f53f3f',
      success: r => { if (r.confirm) { S.cancelBooking(b.id); this.build(); wx.showToast({ title: '已取消', icon: 'none' }); } }
    });
  }
});
