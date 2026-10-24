// 预约与打卡存在 storage
function getBookings() { return wx.getStorageSync('fit_bookings') || []; }
function setBookings(v) { wx.setStorageSync('fit_bookings', v); }
function isBooked(courseId, dayIdx) { return getBookings().some(b => b.id === courseId && b.dayIdx === dayIdx); }
function toggleBooking(c) {
  let list = getBookings();
  const has = list.some(b => b.id === c.id && b.dayIdx === c.dayIdx);
  list = has ? list.filter(b => !(b.id === c.id && b.dayIdx === c.dayIdx)) : list.concat({ id: c.id, dayIdx: c.dayIdx, name: c.name, time: c.time, room: c.room, dateText: c.dateText });
  setBookings(list);
  return !has;
}
function getChecks() {
  const v = wx.getStorageSync('fit_checks');
  return Array.isArray(v) ? v : null;
}
function setChecks(v) { wx.setStorageSync('fit_checks', v); }
module.exports = { getBookings, isBooked, toggleBooking, getChecks, setChecks };
