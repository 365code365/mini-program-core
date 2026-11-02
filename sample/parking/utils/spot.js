// 寻车：每辆在场车记一条停车位置（楼层 / 区域 / 车位号 / 备注），存在 pk_spots，按车牌索引。
// 柱子上的寻车码形如「P1-B2-C-086」（停车场 id - 楼层 - 区域 - 车位号），扫到就直接填好
const KEY = 'pk_spots'
const FLOORS = ['B1', 'B2', 'B3', '地面']
const ZONES = 'ABCDEFGH'.split('')
function all() {
  const v = wx.getStorageSync(KEY)
  return v && typeof v === 'object' && !Array.isArray(v) ? v : {}
}
function of(plate) { return all()[plate] || null }
function save(plate, s) {
  const m = all()
  m[plate] = Object.assign({ at: Date.now() }, s)
  wx.setStorageSync(KEY, m)
  return m[plate]
}
function clear(plate) {
  const m = all()
  delete m[plate]
  wx.setStorageSync(KEY, m)
}
function parseCode(text) {
  const m = String(text || '').toUpperCase().match(/P(\d)-(B[1-3]|F1)-([A-H])-(\d{1,4})/)
  if (!m) return null
  return { lotId: Number(m[1]), floor: m[2] === 'F1' ? '地面' : m[2], zone: m[3], no: m[4] }
}
// 步行指引：电梯厅按区域就近（A~D 区 1 号梯，E~H 区 2 号梯），距离按车位号估算
function guide(s) {
  const zi = ZONES.indexOf(s.zone)
  const lift = zi < 4 ? 1 : 2
  const meters = 20 + (Number(s.no) % 50) * 2
  const side = Number(s.no) % 2 ? '左' : '右'
  const head = s.floor === '地面' ? '从停车场入口岗亭' : '乘 ' + lift + ' 号电梯到 ' + s.floor + ' 层，出电梯厅后'
  return head + '沿「' + s.zone + ' 区」指示牌直行约 ' + meters + ' 米，车位在' + side + '手边'
}
module.exports = { FLOORS, ZONES, all, of, save, clear, parseCode, guide }
