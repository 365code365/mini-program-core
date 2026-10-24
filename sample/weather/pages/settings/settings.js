const W = require('../../utils/data.js')

const FREQ = ['每 15 分钟', '每 30 分钟', '每 1 小时', '每 3 小时', '仅手动刷新']
const WIND = ['级（蒲福风级）', 'km/h', 'm/s']

Page({
  data: { fahrenheit: false, rainAlert: true, morning: true, freq: FREQ, fi: 1, wind: WIND, wi: 0, cache: '0.0' },
  onLoad() {
    const s = wx.getStorageSync('weather_settings') || {}
    this.setData({
      fahrenheit: W.unit() === 'F',
      rainAlert: s.rainAlert !== false,
      morning: s.morning !== false,
      fi: typeof s.fi === 'number' ? s.fi : 1,
      wi: typeof s.wi === 'number' ? s.wi : 0,
      cache: (W.myCities().length * 0.37 + 0.8).toFixed(1)
    })
  },
  save() {
    const d = this.data
    wx.setStorageSync('weather_settings', { rainAlert: d.rainAlert, morning: d.morning, fi: d.fi, wi: d.wi })
  },
  onUnit(e) {
    const f = e.detail.value
    wx.setStorageSync('weather_unit', f ? 'F' : 'C')
    this.setData({ fahrenheit: f })
    wx.showToast({ title: f ? '已切换为华氏度 ℉' : '已切换为摄氏度 ℃', icon: 'none' })
  },
  onRain(e) { this.setData({ rainAlert: e.detail.value }); this.save() },
  onMorning(e) { this.setData({ morning: e.detail.value }); this.save() },
  onFreq(e) { this.setData({ fi: +e.detail.value }); this.save() },
  onWind(e) { this.setData({ wi: +e.detail.value }); this.save() },
  onCache() {
    wx.showModal({
      title: '清除缓存？', content: '将清除 ' + this.data.cache + ' MB 的天气缓存，城市列表与设置会保留。', confirmColor: '#2f7ff0',
      success: (r) => { if (r.confirm) { this.setData({ cache: '0.0' }); wx.showToast({ title: '已清除', icon: 'success' }) } }
    })
  },
  onReset() {
    wx.showModal({
      title: '恢复默认设置？', content: '城市列表将恢复为北京、上海、广州、杭州。', confirmColor: '#f04848', confirmText: '恢复',
      success: (r) => {
        if (!r.confirm) return
        ;['weather_unit', 'weather_settings', 'weather_cities', 'weather_current'].forEach(k => wx.removeStorageSync(k))
        this.onLoad()
        wx.showToast({ title: '已恢复默认', icon: 'success' })
      }
    })
  },
  onAbout() { wx.showModal({ title: '晴雨天气 v3.2.1', content: '云图气象信息技术有限公司出品。演示版本使用离线数据。', showCancel: false }) }
})
