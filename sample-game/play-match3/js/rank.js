// 排行榜：主域把「我的成绩 + 本机记录」发给开放数据域（open/index.js），开放数据域拉好友托管数据画进 sharedCanvas，
// 主域把 sharedCanvas 当纹理贴出来（主域读不了它的像素，与微信一致）。
// 好友数据要宿主接关系链（openData.getFriendCloudStorage）；没接时开放数据域那边 fail，退回画本机记录并注明。
// 拿不到开放数据域（没配 / 不支持）时主域自己画本机排行（source = 'local-main'）。
var PW = 300, PH = 380;
var odc = null, sc = null, dpr = 2, opened = false, ticks = 0, source = 'none';

function init(pixelRatio) {
  dpr = pixelRatio || 2;
  try {
    odc = wx.getOpenDataContext ? wx.getOpenDataContext() : null;
    if (odc) { sc = odc.canvas; sc.width = Math.round(PW * dpr); sc.height = Math.round(PH * dpr); source = 'opendata'; }
  } catch (e) { odc = null; sc = null; }
  if (!odc) source = 'local-main';
}
// 写一份自己的托管数据（真机微信里好友排行读的就是它）；本引擎没接关系链时走 fail，不影响本机排行
function report(total) {
  if (!wx.setUserCloudStorage) return;
  try {
    wx.setUserCloudStorage({
      KVDataList: [{ key: 'm3_total', value: JSON.stringify({ wxgame: { score: total, update_time: Math.floor(Date.now() / 1000) } }) }],
      success: function () {}, fail: function () {}
    });
  } catch (e) {}
}
function open(me, records) {
  opened = true; ticks = 0;
  if (odc) odc.postMessage({ cmd: 'rank', dpr: dpr, w: PW, h: PH, me: me, records: records });
}
function close() { opened = false; if (odc) odc.postMessage({ cmd: 'close' }); }

// 画在 (x, y)，PW × PH。开放数据域是异步画的：开着的时候每 6 帧重传一次纹理
function draw(R, x, y, records) {
  if (!opened) return;
  if (sc) { R.image('rank', sc, x, y, PW, PH, ticks % 6 === 0); ticks++; return; }
  R.rrect(x, y, PW, PH, 16, '#fff8e1');
  R.text('本机排行', x + PW / 2, y + 30, 20, '#6d4c41', 'center', 'bold');
  (records || []).slice(0, 8).forEach(function (r, i) {
    var ry = y + 74 + i * 36;
    R.text(String(i + 1), x + 28, ry, 16, '#8d6e63', 'center', 'bold');
    R.text('第 ' + r.level + ' 关', x + 56, ry, 15, '#5d4037', 'left');
    R.text(String(r.score), x + PW - 20, ry, 16, '#e65100', 'right', 'bold');
  });
}
module.exports = { init: init, open: open, close: close, draw: draw, report: report, W: PW, H: PH,
  isOpen: function () { return opened; }, source: function () { return source; } };
