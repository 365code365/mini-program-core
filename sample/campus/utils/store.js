// 本地状态：已读通知、上课提醒、借阅记录、校园卡余额与流水（kit.get/set 带 campus_ 前缀）
const kit = require('./kit.js');
const D = require('./data.js');
function readSet() { return kit.get('read', []); }
function markRead(id) { const s = readSet(); if (s.indexOf(id) < 0) { s.push(id); kit.set('read', s); } }
function unread() { const s = readSet(); return D.NOTICES.filter(n => s.indexOf(n.id) < 0).length; }
// 通知未读角标画在自定义 tabBar 的「通知」上（等价 wx.setTabBarBadge）
function syncTab(page, idx) { kit.tab(page, idx, { 'list[2].badge': unread() }); }

function borrows() {
  return kit.get('borrows', [
    { id: 'r1', bookId: 'b3', title: '活着', call: 'I247.57/Y79', borrowed: '2026-10-09', due: '2026-11-08', renewed: false },
    { id: 'r2', bookId: 'b8', title: 'Python 编程：从入门到实践', call: 'TP311.56/M43', borrowed: '2026-10-20', due: '2026-11-19', renewed: false }
  ]);
}
function saveBorrows(l) { kit.set('borrows', l); }
function card() {
  return kit.get('card', { no: '2024013588', balance: 86.5, records: [
    { id: 't1', title: '第一食堂 二楼', amount: -12.5, time: '2026-11-01 12:06' },
    { id: 't2', title: '东区浴室', amount: -4, time: '2026-10-31 21:40' },
    { id: 't3', title: '微信充值', amount: 100, time: '2026-10-30 18:12' },
    { id: 't4', title: '图书馆 打印复印', amount: -3.6, time: '2026-10-30 10:25' }
  ] });
}
function saveCard(c) { kit.set('card', c); }
module.exports = { markRead, readSet, unread, syncTab, borrows, saveBorrows, card, saveCard };
