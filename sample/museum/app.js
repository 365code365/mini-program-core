// 云川博物院：导览 + 预约。跨页数据都在 storage（utils/util.js 的 load / save），这里只做启动时的整理
const { load, save } = require('./utils/util.js');
const { ymd } = require('./utils/data.js');

App({
  globalData: { calendarDate: '' },
  onLaunch() {
    // 过期的待使用预约改成「已过期」：凭证当日有效，退出重进状态也要对
    const today = ymd(new Date());
    const list = load('bookings', []);
    let changed = false;
    list.forEach(b => { if (b.status === '待使用' && b.date < today) { b.status = '已过期'; changed = true; } });
    if (changed) save('bookings', list);
  }
});
