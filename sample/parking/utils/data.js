// 静态数据：省份简称、车牌键盘、停车场、优惠券（演示用，不联网）
const PROVINCES = '京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤青藏川宁琼'.split('')
// 车牌里不用 I、O（与 1、0 混淆）
const ALNUM = '1234567890ABCDEFGHJKLMNPQRSTUVWXYZ'.split('')

// first：首小时；half：之后每半小时；cap：每 24 小时封顶；免费时长统一 15 分钟
const LOTS = [
  { id: 1, name: 'P1 滨江天街地下停车场', addr: '江南大道 228 号 B1-B2', dist: 0.4, first: 6, half: 3, cap: 50, free: 36, total: 420, open: '00:00-24:00' },
  { id: 2, name: '星光大道立体车库', addr: '星光大道 99 号', dist: 1.2, first: 5, half: 2, cap: 40, free: 0, total: 160, open: '07:00-23:00' },
  { id: 3, name: '市民中心 P3 停车楼', addr: '市心路 1 号', dist: 2.5, first: 4, half: 2, cap: 30, free: 112, total: 800, open: '00:00-24:00' },
  { id: 4, name: '江南大道路侧泊位', addr: '江南大道（时代大道口）', dist: 0.8, first: 8, half: 4, cap: 60, free: 5, total: 40, open: '07:00-21:00' },
  { id: 5, name: '奥体中心东区停车场', addr: '飞虹路 3 号', dist: 3.6, first: 3, half: 1, cap: 20, free: 260, total: 1200, open: '00:00-24:00' },
  { id: 6, name: '万象城 B2 停车场', addr: '富春路 701 号', dist: 1.9, first: 10, half: 5, cap: 80, free: 0, total: 600, open: '09:00-24:00' }
]
const FREE_MIN = 15

const DURATIONS = [
  { text: '1 小时', hours: 1 },
  { text: '2 小时', hours: 2 },
  { text: '3 小时', hours: 3 },
  { text: '4 小时', hours: 4 },
  { text: '8 小时', hours: 8 },
  { text: '12 小时', hours: 12 }
]

// min：满多少可用；off：减多少；firstHour：抵扣首小时费用
const COUPONS = [
  { name: '不使用优惠券', off: 0, min: 0 },
  { name: '满 10 减 3', off: 3, min: 10 },
  { name: '5 元无门槛', off: 5, min: 0 },
  { name: '首小时免费', firstHour: true, min: 0 }
]

function lotById(id) {
  return LOTS.find(l => l.id === Number(id)) || null
}

module.exports = { PROVINCES, ALNUM, LOTS, FREE_MIN, DURATIONS, COUPONS, lotById }
