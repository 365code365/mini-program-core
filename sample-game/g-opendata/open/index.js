// 开放数据域（G9.7）：收到 {cmd:'rank', color} 就拉好友托管数据（宿主给），画进 sharedCanvas
var sc = wx.getSharedCanvas(), g = sc.getContext('2d');
var color = '#00ff00';
function draw(list) {
    g.fillStyle = color; g.fillRect(0, 0, sc.width, sc.height);
    g.fillStyle = '#ffffff';
    (list || []).forEach(function (f, i) { g.fillRect(10, 10 + i * 30, Math.min(200, f.score), 20); });
}
wx.onMessage(function (m) {
    console.log('收到 ' + JSON.stringify(m) + ' wx.request=' + typeof wx.request + ' storage=' + typeof wx.setStorageSync + ' gl=' + sc.getContext('webgl'));
    if (m.color) color = m.color;
    if (m.cmd === 'rank') {
        wx.getFriendCloudStorage({ keyList: ['score'],
            success: function (r) { draw(r.data.map(function (d) { return { score: Number(d.KVDataList[0].value) }; })); console.log('rank ok ' + r.data.length); },
            fail: function (e) { draw([]); console.log('rank fail ' + e.errMsg); } });
    } else draw([]);
});
requestAnimationFrame(function tick() { requestAnimationFrame(tick); });
console.log('开放数据域就绪 ' + sc.width + 'x' + sc.height);
