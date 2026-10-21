Page({
  data: {
    cells: ['1', '2', '3', '4', '5', '6', '7', '8', '9'],
    rows: [1, 2, 3, 4],
    width: 0,
    safeBottom: 0
  },
  onLoad() {
    const i = wx.getSystemInfoSync();
    this.setData({ width: i.windowWidth, safeBottom: Math.max(0, i.screenHeight - i.safeArea.bottom) });
  }
});
