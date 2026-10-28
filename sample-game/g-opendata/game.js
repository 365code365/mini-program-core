// g-opendata（G9.7）：主域拿 OpenDataContext，把 sharedCanvas 画到主画布左上 200×200；query cmd=rank 让开放数据域拉好友数据
var T = require('_lib/t.js');
var c = wx.createCanvas(), g = c.getContext('2d');
var odc = wx.getOpenDataContext(), sc = odc.canvas;
sc.width = 200; sc.height = 200;
var r = { same: wx.getOpenDataContext() === odc, ctx: sc.getContext('2d'), w: sc.width };
try { sc.toDataURL(); r.read = 'ok'; } catch (e) { r.read = 'denied'; }
T.ev('main', r);
odc.postMessage({ cmd: T.q.cmd || 'paint', color: '#ff0000' });
function frame() {
    g.fillStyle = '#203040'; g.fillRect(0, 0, c.width, c.height);
    g.drawImage(sc, 0, 0);
    requestAnimationFrame(frame);
}
requestAnimationFrame(frame);
T.ready('g-opendata');
