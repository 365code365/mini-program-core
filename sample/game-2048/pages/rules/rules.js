Page({
  data: {
    open: 0, // 展开的那一节
    sections: [
      { title: '怎么玩', body: '在棋盘上向上、下、左、右滑动，所有方块会朝该方向移动到底。' },
      { title: '合并', body: '两个数字相同的方块撞在一起会合成一个，数值相加。同一次移动里，每个方块最多合并一次：2 2 2 2 向左滑得到 4 4，而不是 8。' },
      { title: '新方块', body: '每次有效移动后，会在随机空格里出现一个新方块：90% 是 2，10% 是 4。' },
      { title: '得分', body: '每次合并得到的新数值计入分数。例如 8 和 8 合成 16，得 16 分。' },
      { title: '胜负', body: '合成 2048 即获胜，可以选择继续挑战更高分；棋盘填满且没有可以合并的方块时游戏结束。' },
      { title: '撤销', body: '每一步之后可以撤销一次，回到上一步的棋盘与分数。连续撤销只能撤一步。' }
    ]
  },

  onToggle(e) {
    const i = Number(e.currentTarget.dataset.i)
    this.setData({ open: this.data.open === i ? -1 : i })
  },

  onPlay() {
    wx.navigateBack()
  }
})
