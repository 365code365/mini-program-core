const D = require('../../utils/songs.js')
const P = require('../../utils/player.js')
const S = require('../../utils/store.js')
const { withMini } = require('../../utils/mini.js')
Page(withMini({
  data: {
    miniOnTab: true,
    langs: D.LANGS,
    lang: 0,
    keyword: '',
    songs: [],
    curId: 0
  },
  onShow() {
    const tb = this.getTabBar && this.getTabBar()
    if (tb) tb.setData({ selected: 1 })
    this.refresh()
  },
  // 按语种 + 关键词（歌名 / 歌手）过滤；收藏状态每次从 storage 读，播放页里改的收藏回来就生效
  refresh() {
    const key = D.LANGS[this.data.lang].key
    const kw = this.data.keyword.trim().toLowerCase()
    const favs = S.favs()
    const songs = D.SONGS.filter(s => key === 'all' || s.lang === key)
      .filter(s => !kw || s.name.toLowerCase().indexOf(kw) >= 0 || s.artist.toLowerCase().indexOf(kw) >= 0)
      .map(s => Object.assign({}, s, { fav: favs.indexOf(s.id) >= 0 }))
    const cur = P.snapshot().song
    this.setData({ songs, curId: cur ? cur.id : 0 })
  },
  onLang(e) {
    this.setData({ lang: Number(e.currentTarget.dataset.index) })
    this.refresh()
  },
  onInput(e) {
    this.setData({ keyword: e.detail.value })
    this.refresh()
  },
  onClear() {
    this.setData({ keyword: '' })
    this.refresh()
  },
  onSong(e) {
    const i = Number(e.currentTarget.dataset.index)
    P.playList(this.data.songs.map(s => s.id), i)
    this.setData({ curId: this.data.songs[i].id })
  },
  onFav(e) {
    const i = Number(e.currentTarget.dataset.index)
    const on = S.toggleFav(this.data.songs[i].id)
    this.setData({ ['songs[' + i + '].fav']: on })
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' })
  }
}))
