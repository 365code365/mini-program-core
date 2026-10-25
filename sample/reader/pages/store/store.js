const B = require('../../utils/books.js')

// 轮播位：主编推荐
const BANNERS = [
  { id: 'b1', slogan: '十万年的图书馆里，有你的名字', c0: '#355c7d', c1: '#c06c84' },
  { id: 'b8', slogan: '一座四合院，十年老北平', c0: '#6b4226', c1: '#d4a373' },
  { id: 'b3', slogan: '每一位证人，都坐过那班渡轮', c0: '#2f4858', c1: '#86bbd8' }
]

Page({
  data: {
    banners: BANNERS.map(x => Object.assign({ title: B.get(x.id).title }, x)),
    banner: 0, // 轮播当前页（bindchange 同步）
    bannerSource: '',
    categories: B.CATEGORIES,
    cat: 0,
    list: [],
    keyword: ''
  },

  onLoad() {
    this.filter()
  },

  // 榜单：按评分从高到低，分类过滤
  filter() {
    const c = B.CATEGORIES[this.data.cat]
    const list = B.BOOKS.filter(b => this.data.cat === 0 || b.category === c)
      .slice()
      .sort((a, b) => b.score - a.score)
      .map((b, i) => ({ id: b.id, title: b.title, author: b.author, category: b.category, score: b.score, words: b.words, reads: b.reads, c0: b.c0, c1: b.c1, rank: i + 1, brief: b.intro.slice(0, 34) + '…' }))
    this.setData({ list })
  },

  onBanner(e) {
    this.setData({ banner: e.detail.current, bannerSource: e.detail.source || '' })
  },

  onCat(e) {
    const i = Number(e.currentTarget.dataset.i)
    if (i === this.data.cat) return
    this.setData({ cat: i })
    this.filter()
  },

  onBook(e) {
    wx.navigateTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id })
  }
})
