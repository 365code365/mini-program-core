const G = require('../../utils/game.js')
const RNG = require('../../utils/rng.js')
const S = require('../../utils/store.js')

const BOARD_RPX = 690 // 棋盘边长
const GAP_RPX = 20 // 格子间距

Page({
  data: {
    size: 4,
    board: [], // 二维数组，给逻辑与测试看
    rows: [], // 渲染用：[[{ v, cls }]]
    cell: 147, // 格子边长（rpx）
    fontRpx: 64,
    score: 0,
    best: 0,
    gained: 0, // 上一步得分（「+8」飘字）
    canUndo: false,
    won: false,
    over: false,
    moves: 0,
    theme: S.theme('classic'),
    anim: true
  },

  onLoad(query) {
    // ?seed=7：固定随机种子（测试与「每日挑战」用）；不给就按时间
    this.seed = query && query.seed ? Number(query.seed) : Date.now() % 1000000007
    this.rng = RNG.create(this.seed)
    this.applySettings()
    this.newGame()
  },

  onShow() {
    // 从设置页回来：尺寸变了就重开一局，主题 / 动画直接换
    if (!this.rng) return
    const oldSize = this.data.size
    this.applySettings()
    if (this.data.size !== oldSize) this.newGame()
    else this.render(this.data.board, -1, -1)
  },

  applySettings() {
    const s = S.loadSettings()
    const n = s.size
    const cell = Math.floor((BOARD_RPX - GAP_RPX * (n + 1)) / n)
    this.setData({
      size: n,
      cell,
      fontRpx: n === 3 ? 80 : n === 4 ? 64 : 50,
      theme: S.theme(s.theme),
      anim: !!s.anim,
      best: S.loadBest(n)
    })
  },

  // 渲染用的格子：新出的那一格带 pop（开了动画时）
  render(board, pr, pc) {
    const rows = board.map((row, r) => row.map((v, c) => {
      let cls = 'tile t' + (v > 2048 ? 'super' : v)
      if (v >= 1024) cls += ' small'
      if (this.data.anim && r === pr && c === pc) cls += ' pop'
      return { v, cls, key: r * 10 + c }
    }))
    this.setData({ board, rows })
  },

  newGame() {
    const board = G.empty(this.data.size)
    G.spawn(board, this.rng)
    const p = G.spawn(board, this.rng)
    this.prev = null
    this.recorded = false
    this.setData({ score: 0, gained: 0, canUndo: false, won: false, over: false, moves: 0 })
    this.render(board, p.r, p.c)
  },

  onTouchStart(e) {
    const t = e.touches[0] || e.changedTouches[0]
    this.start = { x: t.clientX, y: t.clientY }
  },

  // 棋盘上的 touchmove 截住：滑棋盘时页面不跟着滚
  noop() {},

  onTouchEnd(e) {
    if (!this.start) return
    const t = e.changedTouches[0]
    const dir = G.direction(t.clientX - this.start.x, t.clientY - this.start.y)
    this.start = null
    if (dir) this.doMove(dir)
  },

  doMove(dir) {
    if (this.data.over) return
    const board = this.data.board
    const res = G.move(board, dir)
    if (!res.moved) {
      if (!G.canMove(board)) this.gameOver()
      return
    }
    this.prev = { board: G.clone(board), score: this.data.score, moves: this.data.moves }
    const p = G.spawn(res.board, this.rng)
    const score = this.data.score + res.gained
    const patch = { score, gained: res.gained, canUndo: true, moves: this.data.moves + 1 }
    if (score > this.data.best) {
      patch.best = score
      S.saveBest(this.data.size, score)
    }
    this.setData(patch)
    this.render(res.board, p ? p.r : -1, p ? p.c : -1)
    if (!this.data.won && G.maxTile(res.board) >= 2048) {
      this.setData({ won: true })
      wx.showModal({
        title: '你赢了！',
        content: '合成了 2048，本局 ' + score + ' 分',
        confirmText: '再来一局',
        cancelText: '继续挑战',
        success: r => {
          if (r.confirm) {
            this.record()
            this.newGame()
          }
        }
      })
    } else if (!G.canMove(res.board)) {
      this.gameOver()
    }
  },

  gameOver() {
    this.setData({ over: true, canUndo: false })
    this.record()
    wx.showModal({
      title: '游戏结束',
      content: '没有可以移动的方块了，本局 ' + this.data.score + ' 分',
      confirmText: '再来一局',
      showCancel: false,
      success: () => this.newGame()
    })
  },

  // 一局只记一次（胜利后继续玩再失败，不重复记）
  record() {
    if (this.recorded) return
    this.recorded = true
    S.addRecord({ score: this.data.score, maxTile: G.maxTile(this.data.board), size: this.data.size, moves: this.data.moves })
  },

  onUndo() {
    if (!this.prev) {
      wx.showToast({ title: '只能撤销一步', icon: 'none' })
      return
    }
    const p = this.prev
    this.prev = null
    this.setData({ score: p.score, moves: p.moves, gained: 0, canUndo: false })
    this.render(p.board, -1, -1)
  },

  onNewGame() {
    if (!this.data.score || this.data.over) {
      this.newGame()
      return
    }
    wx.showModal({
      title: '新游戏',
      content: '放弃当前这局（' + this.data.score + ' 分）重新开始？',
      confirmText: '重新开始',
      success: r => {
        if (!r.confirm) return
        this.record()
        this.newGame()
      }
    })
  },

  go(e) {
    wx.navigateTo({ url: '/pages/' + e.currentTarget.dataset.p + '/' + e.currentTarget.dataset.p })
  }
})
