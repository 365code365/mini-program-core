// 空教室：选教学楼 + 日期 + 节次段，按本学期课表与教室容量算出空闲教室（确定性：同一组条件结果不变）
const D = require('../../utils/data.js');
const kit = require('../../utils/kit.js');
const SLOTS = ['1-2 节', '3-4 节', '5-6 节', '7-8 节', '9-11 节'];
function rooms(building) {
  const list = [];
  for (let f = 1; f <= 4; f++) for (let r = 1; r <= 6; r++) list.push({ no: building + ' ' + (f * 100 + r), cap: [60, 120, 80, 40, 150, 90][(f + r) % 6] });
  return list;
}
// 简单的确定性「占用」：教室号 + 日期 + 节次求哈希
function busy(no, date, slot) {
  let h = 7;
  const s = no + date + slot;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 1000003;
  return h % 10 < 6;
}
Page({
  data: { buildings: D.BUILDINGS, b: 0, date: '', start: '', end: '', slots: SLOTS, slot: 0, list: [], minCap: false },
  onLoad() {
    this.setData({ date: kit.dateStr(0), start: kit.dateStr(0), end: kit.dateStr(13) });
    this.query();
  },
  onB(e) { this.setData({ b: Number(e.detail.value) }); this.query(); },
  onDate(e) { this.setData({ date: e.detail.value }); this.query(); },
  pickSlot(e) { this.setData({ slot: Number(e.currentTarget.dataset.i) }); this.query(); },
  onCap(e) { this.setData({ minCap: e.detail.value }); this.query(); },
  query() {
    const d = this.data;
    const list = rooms(d.buildings[d.b]).filter(r => !busy(r.no, d.date, d.slot) && (!d.minCap || r.cap >= 100));
    this.setData({ list });
  },
  reserve(e) {
    const no = e.currentTarget.dataset.no;
    kit.confirm('借用教室', '借用 ' + no + '（' + this.data.date + ' ' + SLOTS[this.data.slot] + '）用于自习 / 社团活动？需辅导员审批。').then(ok => {
      if (ok) kit.toast('已提交借用申请，审批结果将通过通知告知');
    });
  }
});
