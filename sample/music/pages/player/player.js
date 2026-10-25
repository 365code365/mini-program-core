const D = require('../../utils/songs.js')
const P = require('../../utils/player.js')
const S = require('../../utils/store.js')
Page({
  data: {
    song: null,
    playing: false,
    pos: 0,
    posText: '00:00',
    mode: 0,
    modeText: '顺序播放',
    modeIcon: '顺',
    fav: false,
    lyric: [],
    curLine: 0,
    toView: '',
    showQueue: false,
    queue: [],
    curIndex: -1
  },
  onLoad(q) {
    // ?id=3：直接放某一首（队列为全部单曲）
    if (q && q.id) {
      const ids = D.SONGS.map(s => s.id)
      P.playList(ids, Math.max(0, ids.indexOf(Number(q.id))))
    }
    this.sub = snap => this.apply(snap)
  },
  onShow() {
    P.on(this.sub)
  },
  onHide() {
    P.off(this.sub)
  },
  onUnload() {
    P.off(this.sub)
  },
  apply(snap) {
    const s = snap.song
    const patch = { playing: snap.playing, mode: snap.mode, modeText: snap.modeText, modeIcon: snap.modeIcon, curIndex: snap.index }
    if (!s) return this.setData(patch)
    if (!this.data.song || this.data.song.id !== s.id) {
      // 换歌：歌词、收藏状态、队列跟着换
      Object.assign(patch, {
        song: s,
        lyric: D.lyric(s.id),
        fav: S.isFav(s.id)
      })
      wx.setNavigationBarTitle({ title: s.name })
    }
    const qkey = snap.queue.join(',')
    if (qkey !== this.qkey) {
      this.qkey = qkey
      patch.queue = snap.queue.map(id => D.song(id))
    }
    // 拖动进度条时不让播放推进把滑块拽回去
    if (!this.dragging) {
      const line = D.lineAt(snap.pos)
      Object.assign(patch, { pos: snap.pos, posText: snap.posText, curLine: line, toView: 'l' + Math.max(0, line - 2) })
    }
    this.setData(patch)
  },
  onSeeking(e) {
    this.dragging = true
    this.setData({ posText: D.fmt(e.detail.value) })
  },
  onSeek(e) {
    this.dragging = false
    P.seek(Number(e.detail.value))
  },
  onToggle() {
    P.toggle()
  },
  onPrev() {
    P.prev()
  },
  onNext() {
    P.next()
  },
  onMode() {
    const m = P.cycleMode()
    wx.showToast({ title: m.text, icon: 'none' })
  },
  onFav() {
    if (!this.data.song) return
    const on = S.toggleFav(this.data.song.id)
    this.setData({ fav: on })
    wx.showToast({ title: on ? '已加入我的收藏' : '已取消收藏', icon: 'none' })
  },
  // 点某行歌词：跳到这一行
  onLine(e) {
    P.seek(this.data.lyric[Number(e.currentTarget.dataset.index)].t)
  },
  onQueue() {
    this.setData({ showQueue: true })
  },
  onCloseQueue() {
    this.setData({ showQueue: false })
  },
  onPickQueue(e) {
    P.playList(this.data.queue.map(s => s.id), Number(e.currentTarget.dataset.index))
    this.setData({ showQueue: false })
  },
  noop() {}
})
