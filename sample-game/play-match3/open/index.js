// 开放数据域：收到主域的 { cmd: 'rank', me, records, w, h, dpr } 就拉好友托管数据（key m3_total）画排行榜进 sharedCanvas。
// 开放数据域没有存储、没有网络（与微信一致），本机记录由主域随消息带过来。
// 好友数据拿不到（宿主没接关系链 → fail）时画本机记录，标题下注明来源，不让面板空着。
var sc = wx.getSharedCanvas(), g = sc.getContext('2d');
var cfg = { w: 300, h: 380, dpr: 2 };
var MEDAL = ['#ffc107', '#b0bec5', '#d7a86e'];

function rr(x, y, w, h, r) {
  g.beginPath();
  g.moveTo(x + r, y);
  g.arcTo(x + w, y, x + w, y + h, r); g.arcTo(x + w, y + h, x, y + h, r);
  g.arcTo(x, y + h, x, y, r); g.arcTo(x, y, x + w, y, r);
  g.closePath();
}
function txt(s, x, y, size, color, align, bold) {
  g.font = (bold ? 'bold ' : '') + size + 'px sans-serif';
  g.fillStyle = color; g.textAlign = align || 'left'; g.textBaseline = 'middle';
  g.fillText(String(s), x, y);
}
// rows：[{ name, score, sub, me }]
function paint(title, note, rows) {
  var W = cfg.w, H = cfg.h;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.clearRect(0, 0, sc.width, sc.height);
  g.setTransform(cfg.dpr, 0, 0, cfg.dpr, 0, 0);
  rr(0, 0, W, H, 16); g.fillStyle = '#fff8e1'; g.fill();
  rr(0, 0, W, 64, 16); g.fillStyle = '#ff8a65'; g.fill();
  g.fillRect(0, 40, W, 24);
  txt(title, W / 2, 26, 20, '#ffffff', 'center', true);
  txt(note, W / 2, 50, 12, '#fff3e0', 'center');
  if (!rows.length) { txt('还没有成绩，先去过一关吧', W / 2, H / 2, 15, '#8d6e63', 'center'); return; }
  rows.slice(0, 8).forEach(function (r, i) {
    var y = 76 + i * 37;
    if (r.me) { rr(8, y, W - 16, 33, 10); g.fillStyle = '#ffe0b2'; g.fill(); }
    var cy = y + 17;
    if (i < 3) { g.beginPath(); g.arc(28, cy, 12, 0, Math.PI * 2); g.fillStyle = MEDAL[i]; g.fill(); txt(i + 1, 28, cy + 1, 14, '#ffffff', 'center', true); }
    else txt(i + 1, 28, cy, 15, '#a1887f', 'center', true);
    txt(r.name, 50, cy - 7, 15, '#4e342e', 'left', true);
    txt(r.sub, 50, cy + 9, 11, '#a1887f', 'left');
    txt(r.score, W - 18, cy, 17, '#e65100', 'right', true);
  });
}
function localRows(m) {
  return (m.records || []).map(function (r) {
    return { name: r.win ? '第' + r.level + '关 · ' + r.stars + '星' : '第' + r.level + '关 · 未过关', score: r.score, sub: r.date, me: r.latest };
  });
}
wx.onMessage(function (m) {
  if (!m || m.cmd !== 'rank') return;
  cfg.w = m.w || cfg.w; cfg.h = m.h || cfg.h; cfg.dpr = m.dpr || cfg.dpr;
  paint('好友排行', '正在读取…', []);
  wx.getFriendCloudStorage({
    keyList: ['m3_total'],
    success: function (res) {
      var rows = (res.data || []).map(function (d) {
        var kv = (d.KVDataList || [])[0], s = 0;
        try { s = JSON.parse(kv.value).wxgame.score; } catch (e) { s = Number(kv && kv.value) || 0; }
        return { name: d.nickname || '好友', score: s, sub: '总分', me: false };
      });
      if (m.me) rows.push({ name: m.me.name, score: m.me.score, sub: '总分（我）', me: true });
      rows.sort(function (a, b) { return b.score - a.score; });
      paint('好友排行', '微信好友 · 总分', rows);
      console.log('排行：好友 ' + rows.length + ' 人');
    },
    fail: function (e) {
      paint('本机排行', '未接入微信好友关系链，显示本机最好成绩', localRows(m));
      console.log('排行：好友数据不可用（' + (e && e.errMsg) + '），显示本机记录 ' + (m.records || []).length + ' 条');
    }
  });
});
console.log('开放数据域就绪 ' + sc.width + 'x' + sc.height);
