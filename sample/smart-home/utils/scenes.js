// 场景：内置三个（回家 / 离家 / 睡眠）+ 用户自定义（键 sh_scenes）。
// 执行场景 = 批量改设备状态；离线设备跳过。
const D = require('./devices.js')

const BUILTIN = [
  {
    id: 'home',
    name: '回家',
    icon: '家',
    color: '#ff9f1c',
    desc: '开客厅灯与空调、打开电视和窗帘',
    // 每条：设备 id → 要改成的状态
    actions: [
      { id: 'light1', patch: { on: true, brightness: 80 } },
      { id: 'ac1', patch: { on: true, temp: 26, mode: 'cool' } },
      { id: 'tv1', patch: { on: true } },
      { id: 'curtain1', patch: { on: true } }
    ]
  },
  {
    id: 'away',
    name: '离家',
    icon: '离',
    color: '#5c677d',
    desc: '除冰箱外全部关闭',
    actions: 'allOffExceptFridge'
  },
  {
    id: 'sleep',
    name: '睡眠',
    icon: '眠',
    color: '#6a4c93',
    desc: '关掉所有灯和电视，卧室空调 27℃ 除湿，开加湿器',
    actions: [
      { id: 'light1', patch: { on: false } },
      { id: 'light2', patch: { on: false } },
      { id: 'light3', patch: { on: false } },
      { id: 'light4', patch: { on: false } },
      { id: 'tv1', patch: { on: false } },
      { id: 'ac2', patch: { on: true, temp: 27, mode: 'dry', wind: 1 } },
      { id: 'humid1', patch: { on: true } }
    ]
  }
]

function loadCustom() {
  return wx.getStorageSync('sh_scenes') || []
}

function saveCustom(list) {
  wx.setStorageSync('sh_scenes', list)
}

function find(id) {
  return BUILTIN.find(s => s.id === id) || loadCustom().find(s => s.id === id) || null
}

// 场景 → 具体动作列表
function actionsOf(scene, devices) {
  if (scene.actions === 'allOffExceptFridge') {
    return devices.filter(d => d.type !== 'fridge').map(d => ({ id: d.id, patch: { on: false } }))
  }
  if (scene.custom) {
    return scene.devices.map(id => ({ id, patch: { on: scene.action === 'on' } }))
  }
  return scene.actions
}

// 执行：返回 { changed: 实际改了几台, skipped: 离线跳过几台 }
function run(scene) {
  const list = D.load()
  let changed = 0
  let skipped = 0
  actionsOf(scene, list).forEach(a => {
    const d = list.find(x => x.id === a.id)
    if (!d) return
    if (!d.online) {
      skipped++
      return
    }
    const diff = Object.keys(a.patch).some(k => d[k] !== a.patch[k])
    Object.assign(d, a.patch)
    if (diff) changed++
  })
  D.save(list)
  wx.setStorageSync('sh_last_scene', scene.id)
  return { changed, skipped }
}

// 新建场景的校验：返回错误文案，合法时返回空串
function validate(name, devices) {
  const n = (name || '').trim()
  if (!n) return '请输入场景名称'
  if (n.length > 8) return '名称不超过 8 个字'
  if (BUILTIN.concat(loadCustom()).some(s => s.name === n)) return '已有同名场景'
  if (!devices || !devices.length) return '至少选择一台设备'
  return ''
}

function addCustom(name, devices, action) {
  const list = loadCustom()
  const s = { id: 'c' + (list.length + 1) + '_' + Date.now() % 100000, name: name.trim(), devices, action, custom: true, icon: name.trim()[0], color: '#2a9d8f' }
  list.push(s)
  saveCustom(list)
  return s
}

function removeCustom(id) {
  saveCustom(loadCustom().filter(s => s.id !== id))
}

module.exports = { BUILTIN, loadCustom, find, run, validate, addCustom, removeCustom }
