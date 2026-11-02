// 故障报修单：存在 storage（ev_repairs）。处理进度按提交后经过的时间推算（演示用，不联网）：
// 5 分钟内「已受理」，30 分钟内「已派单」（运维 2 小时内到场），之后「已处理」
const KEY = 'ev_repairs';
const TYPES = ['无法启动充电', '充电中途断开', '枪头 / 线缆损坏', '屏幕无显示', '扣费异常', '车位被燃油车占用', '其他问题'];
const STEPS = ['已受理', '已派单', '已处理'];
function pad(n) { return n < 10 ? '0' + n : '' + n; }
function stamp(ts) {
  const d = new Date(ts);
  return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}
function step(r, now) {
  const m = ((now || Date.now()) - r.at) / 60000;
  return m < 5 ? 0 : m < 30 ? 1 : 2;
}
function view(r) {
  const s = step(r);
  const note = s === 0 ? '客服已收到，正在核实终端状态' : s === 1 ? '已通知运维人员，预计 2 小时内到场处理' : (r.type === '扣费异常' ? '已核实，多扣的费用 1~3 个工作日退回余额' : '终端已检修恢复，感谢你的反馈');
  return Object.assign({}, r, { step: s, stepText: STEPS[s], note, time: stamp(r.at), photoCount: (r.photos || []).length });
}
function all() {
  const v = wx.getStorageSync(KEY);
  return (Array.isArray(v) ? v : []).map(view);
}
// 同一终端同一问题 30 分钟内重复提交的，提示已有报修单
function duplicate(no, type) {
  return all().find(r => r.no === no && r.type === type && Date.now() - r.at < 30 * 60000) || null;
}
function add(r) {
  const v = wx.getStorageSync(KEY);
  const list = Array.isArray(v) ? v : [];
  const item = Object.assign({ id: 'R' + Date.now().toString().slice(-8), at: Date.now() }, r);
  list.unshift(item);
  wx.setStorageSync(KEY, list);
  return view(item);
}
function pending() { return all().filter(r => r.step < 2).length; }
module.exports = { TYPES, STEPS, all, add, duplicate, pending };
