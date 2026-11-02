// 把航班写进手机日历：先 wx.addPhoneCalendar，宿主不支持时复制行程文字到剪贴板
const D = require('./date');
const order = require('./order');

function addToCalendar(o) {
  const l = o.legs[0], f = l.flight;
  const start = Math.floor((D.parse(f.date).getTime() + f.dep * 60000) / 1000);
  wx.addPhoneCalendar({
    title: f.airlineName + f.no + ' ' + f.fromName + '→' + f.toName,
    startTime: start,
    endTime: start + (f.durMin || 150) * 60,
    location: f.depAirport,
    description: order.itinerary(o),
    alarm: true,
    alarmOffset: 3 * 3600,
    success: () => wx.showToast({ title: '已加入日历，起飞前 3 小时提醒', icon: 'none' }),
    fail: () => wx.setClipboardData({
      data: order.itinerary(o),
      success: () => wx.showModal({ title: '无法写入手机日历', content: '当前环境不支持添加日历，行程信息已复制到剪贴板，可粘贴到备忘录或日历中。', showCancel: false })
    })
  });
}

module.exports = { addToCalendar };
