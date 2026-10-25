// 房间与设备：初始数据 + storage 持久化（键 sh_devices）
const ROOMS = [
  { key: 'living', name: '客厅' },
  { key: 'bedroom', name: '卧室' },
  { key: 'kitchen', name: '厨房' },
  { key: 'study', name: '书房' }
]

// 设备类型：图标字、颜色、开着时的状态文案
const TYPES = {
  ac: { icon: '空', color: '#3a86ff', label: '空调' },
  light: { icon: '灯', color: '#ffb703', label: '灯' },
  tv: { icon: '视', color: '#8e7dff', label: '电视' },
  curtain: { icon: '帘', color: '#2ec4b6', label: '窗帘' },
  humidifier: { icon: '湿', color: '#4cc9f0', label: '加湿器' },
  fridge: { icon: '冰', color: '#06d6a0', label: '冰箱' },
  hood: { icon: '烟', color: '#8d99ae', label: '油烟机' },
  speaker: { icon: '音', color: '#ef476f', label: '音箱' }
}

const MODES = [
  { key: 'cool', name: '制冷' },
  { key: 'heat', name: '制热' },
  { key: 'dry', name: '除湿' },
  { key: 'fan', name: '送风' }
]

const TIMERS = ['不定时', '30 分钟后关', '1 小时后关', '2 小时后关', '4 小时后关']

function ac(id, room, name, on) {
  return { id, room, type: 'ac', name, on, online: true, temp: 26, mode: 'cool', wind: 3, timer: 0 }
}

function light(id, room, name, on, brightness) {
  return { id, room, type: 'light', name, on, online: true, brightness, ct: 4000, color: '#ffffff' }
}

function plain(id, room, type, name, on, online) {
  return { id, room, type, name, on, online }
}

function seed() {
  return [
    ac('ac1', 'living', '客厅空调', false),
    light('light1', 'living', '客厅吸顶灯', true, 80),
    plain('tv1', 'living', 'tv', '智能电视', false, true),
    plain('curtain1', 'living', 'curtain', '电动窗帘', false, false),
    ac('ac2', 'bedroom', '卧室空调', false),
    light('light2', 'bedroom', '床头灯', false, 40),
    plain('humid1', 'bedroom', 'humidifier', '加湿器', false, true),
    plain('fridge1', 'kitchen', 'fridge', '冰箱', true, true),
    plain('hood1', 'kitchen', 'hood', '油烟机', false, true),
    light('light3', 'kitchen', '厨房灯', false, 100),
    light('light4', 'study', '护眼台灯', false, 60),
    plain('speaker1', 'study', 'speaker', '智能音箱', false, false)
  ]
}

function load() {
  let list = wx.getStorageSync('sh_devices')
  if (!list || !list.length) {
    list = seed()
    wx.setStorageSync('sh_devices', list)
  }
  return list
}

function save(list) {
  wx.setStorageSync('sh_devices', list)
}

function get(id) {
  return load().find(d => d.id === id) || null
}

// 改一台设备并落盘，返回改后的设备
function update(id, patch) {
  const list = load()
  const d = list.find(x => x.id === id)
  if (!d) return null
  Object.assign(d, patch)
  save(list)
  return d
}

// 卡片上的一行状态
function status(d) {
  if (!d.online) return '离线'
  if (!d.on) return '已关闭'
  if (d.type === 'ac') return modeName(d.mode) + ' ' + d.temp + '℃'
  if (d.type === 'light') return '亮度 ' + d.brightness + '%'
  if (d.type === 'fridge') return '冷藏 4℃'
  return '运行中'
}

function modeName(key) {
  const m = MODES.find(x => x.key === key)
  return m ? m.name : ''
}

function roomName(key) {
  const r = ROOMS.find(x => x.key === key)
  return r ? r.name : ''
}

// 给页面用：带上图标、颜色、状态文案
function view(d) {
  const t = TYPES[d.type]
  return Object.assign({}, d, { icon: t.icon, color: t.color, status: status(d), roomName: roomName(d.room), detail: d.type === 'ac' || d.type === 'light' })
}

module.exports = { ROOMS, TYPES, MODES, TIMERS, seed, load, save, get, update, status, view, roomName }
