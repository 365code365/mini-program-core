// 订单：下单、状态推导、取消退款。存储里的 status 只有 paid / used / cancelled，
// 「已结束」是 paid 且最后一个时段已过，按当前时间推出来（不回写，避免各页时间不一致）。
const D = require('./data.js');
const F = require('./fmt.js');
const store = require('./store.js');
const PAY_TEXT = { wechat: '微信支付', balance: '储值余额', times: '次卡' };
const ST = {
  paid: { text: '待使用', color: '#13b37b' },
  used: { text: '已入场', color: '#3a7bd5' },
  ended: { text: '已结束', color: '#8a8d96' },
  cancelled: { text: '已取消', color: '#b0b3ba' }
};
function at(dateKey, h) { const d = F.fromKey(dateKey); d.setHours(h, 0, 0, 0); return d.getTime(); }
function startTs(o) { return at(o.date, Math.min.apply(null, o.segments.map((s) => s.start))); }
function endTs(o) { return at(o.date, Math.max.apply(null, o.segments.map((s) => s.end))); }
function stateOf(o) {
  if (o.status === 'paid' && Date.now() >= endTs(o)) return 'ended';
  return o.status;
}
// 列表 / 详情要的展示字段
function view(o) {
  const st = stateOf(o);
  return Object.assign({}, o, {
    st, stText: ST[st].text, stColor: ST[st].color, payText: PAY_TEXT[o.payMethod] || '',
    dateText: F.dateText(o.date), segText: o.segments.map((s) => s.courtLabel + ' ' + s.text).join('；'),
    canCancel: st === 'paid', canPass: st === 'paid'
  });
}
function create(draft, payMethod, contact) {
  const v = D.getVenue(draft.venueId);
  const hours = draft.segs.reduce((a, s) => a + s.hours, 0);
  const o = {
    id: F.orderNo(), venueId: v.id, venueName: v.name, photo: v.photo, sport: v.sport, sportName: D.sportName(v.sport),
    date: draft.date, segments: draft.segs, hours, total: draft.total,
    payMethod, paid: payMethod === 'times' ? 0 : draft.total,
    contact, code: String(Math.floor(100000 + Math.random() * 900000)), status: 'paid', createdAt: Date.now()
  };
  return store.upsertOrder(o);
}
// 取消：开场前 2 小时以上可免费取消，钱 / 次数原路退回
function cancel(o) {
  if (startTs(o) - Date.now() < 2 * 3600000) return { ok: false, msg: '开场前 2 小时内不能取消，可联系场馆协商' };
  const c = store.card();
  let msg = '¥' + o.paid + ' 将原路退回微信，1~3 个工作日到账';
  if (o.payMethod === 'balance') { c.balance += o.paid; store.addRecord(c, '退款 · ' + o.venueName, o.paid); msg = '¥' + o.paid + ' 已退回储值余额'; }
  if (o.payMethod === 'times') {
    const t = c.times.find((x) => x.sport === o.sport);
    if (t) t.left += o.hours;
    store.addRecord(c, '退回次卡 ' + o.hours + ' 次 · ' + o.venueName, 0);
    msg = o.hours + ' 次已退回次卡';
  }
  store.saveCard(c);
  o.status = 'cancelled'; o.cancelledAt = Date.now();
  store.upsertOrder(o);
  return { ok: true, msg };
}
module.exports = { view, create, cancel, stateOf, startTs, endTs, PAY_TEXT };
