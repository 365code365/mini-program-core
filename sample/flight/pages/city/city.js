const C = require('../../utils/cities');

// 城市选择：热门 + 字母分组 + 右侧索引（scroll-into-view）+ 名称 / 拼音 / 三字码搜索。选中经 EventChannel 回传给打开它的页面
Page({
  data: { type: 'from', current: '', keyword: '', results: [], hot: [], recent: [], groups: [], letters: [], intoView: '', bubble: '' },
  onLoad(q) {
    const groups = C.groups();
    this.setData({
      type: q.type || 'from',
      hot: C.CITIES.filter(c => c.hot).map(c => ({ code: c.code, name: c.name })),
      groups: groups.map(g => ({ letter: g.letter, list: g.list.map(c => ({ code: c.code, name: c.name, py: c.py, ap: c.airports.map(a => a.short + ' ' + a.code).join(' / ') })) })),
      letters: groups.map(g => g.letter),
      recent: (wx.getStorageSync('fl_city_recent') || []).map(code => ({ code, name: C.getCity(code).name }))
    });
    wx.setNavigationBarTitle({ title: q.type === 'to' ? '选择到达城市' : '选择出发城市' });
    this.channel = this.getOpenerEventChannel ? this.getOpenerEventChannel() : null;
    if (this.channel && this.channel.on) this.channel.on('init', (d) => this.setData({ type: d.type, current: d.current }));
  },
  onInput(e) {
    const keyword = e.detail.value;
    const results = C.search(keyword).map(c => ({ code: c.code, name: c.name, py: c.py, ap: c.airports.map(a => a.name + ' ' + a.code).join(' / ') }));
    this.setData({ keyword, results });
  },
  clear() { this.setData({ keyword: '', results: [] }); },
  jump(e) {
    const l = e.currentTarget.dataset.l;
    this.setData({ intoView: 'L-' + l, bubble: l === 'hot' ? '热' : l });
    clearTimeout(this._t);
    this._t = setTimeout(() => this.setData({ bubble: '' }), 600);
  },
  choose(e) {
    const code = e.currentTarget.dataset.code;
    const recent = [code].concat((wx.getStorageSync('fl_city_recent') || []).filter(c => c !== code)).slice(0, 4);
    wx.setStorageSync('fl_city_recent', recent);
    if (this.channel && this.channel.emit) this.channel.emit('pickCity', { type: this.data.type, code });
    wx.navigateBack();
  },
  onUnload() { clearTimeout(this._t); }
});
