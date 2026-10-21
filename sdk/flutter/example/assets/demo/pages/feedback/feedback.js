const BAR_COLORS = [
  { frontColor: '#ffffff', backgroundColor: '#576b95' },
  { frontColor: '#000000', backgroundColor: '#ffc300' },
  { frontColor: '#ffffff', backgroundColor: '#e64340' },
  { frontColor: '#ffffff', backgroundColor: '#07c160' }
];

Page({
  data: {
    modalResult: '',
    sheetResult: '',
    barLoading: false
  },
  colorAt: 0,
  titleSeq: 0,

  // ── Toast / Loading ──
  toastSuccess() { wx.showToast({ title: '操作成功', icon: 'success' }); },
  toastNone() { wx.showToast({ title: '这是一条只有文字的提示，文字长了会换行显示', icon: 'none' }); },
  toastLong() { wx.showToast({ title: '3 秒后消失', icon: 'success', duration: 3000, mask: true }); },
  hideToast() { wx.hideToast(); },
  loading() {
    wx.showLoading({ title: '加载中' });
    setTimeout(() => wx.hideLoading(), 2000);
  },
  loadingMask() {
    wx.showLoading({ title: '处理中，请稍候', mask: true });
    setTimeout(() => {
      wx.hideLoading();
      wx.showToast({ title: '处理完成', icon: 'success' });
    }, 2000);
  },

  // ── Modal ──
  modal() {
    wx.showModal({
      title: '删除确认',
      content: '删除后不可恢复，确定要删除这条记录吗？',
      success: (r) => this.setData({ modalResult: 'confirm=' + r.confirm + '，cancel=' + r.cancel })
    });
  },
  modalSingle() {
    wx.showModal({
      title: '提示',
      content: '只有一个按钮（showCancel: false）',
      showCancel: false,
      success: (r) => this.setData({ modalResult: '单按钮 confirm=' + r.confirm })
    });
  },
  modalCustom() {
    wx.showModal({
      title: '开启通知',
      content: '及时收到订单状态变化',
      cancelText: '以后再说',
      confirmText: '去开启',
      success: (r) => this.setData({ modalResult: r.confirm ? '点了「去开启」' : '点了「以后再说」' })
    });
  },
  modalChain() {
    wx.showModal({
      title: '第一步',
      content: '点确定后再弹第二个',
      success: (r) => {
        if (!r.confirm) return;
        wx.showModal({
          title: '第二步',
          content: '回调里再调 showModal',
          showCancel: false,
          success: () => this.setData({ modalResult: '两个弹窗都走完了' })
        });
      }
    });
  },

  // ── ActionSheet ──
  sheet() {
    wx.showActionSheet({
      itemList: ['拍照', '从相册选择'],
      success: (r) => this.setData({ sheetResult: 'tapIndex=' + r.tapIndex + '（' + typeof r.tapIndex + '）' }),
      fail: (e) => this.setData({ sheetResult: e.errMsg })
    });
  },
  sheetAlert() {
    wx.showActionSheet({
      alertText: '退出后不会删除任何历史数据',
      itemList: ['退出登录'],
      itemColor: '#e64340',
      success: () => this.setData({ sheetResult: '确认退出' }),
      fail: (e) => this.setData({ sheetResult: e.errMsg })
    });
  },
  sheetTooMany() {
    wx.showActionSheet({
      itemList: ['1', '2', '3', '4', '5', '6', '7'],
      fail: (e) => this.setData({ sheetResult: 'fail：' + e.errMsg })
    });
  },
  sheetPromise() {
    // 一个回调都不给时返回 Promise
    wx.showActionSheet({ itemList: ['A', 'B', 'C'] })
      .then((r) => this.setData({ sheetResult: 'Promise resolve tapIndex=' + r.tapIndex }))
      .catch((e) => this.setData({ sheetResult: 'Promise reject ' + e.errMsg }));
  },

  // ── 导航栏 ──
  setTitle() {
    this.titleSeq++;
    wx.setNavigationBarTitle({ title: '新标题 #' + this.titleSeq });
  },
  setColor() {
    const c = BAR_COLORS[this.colorAt++ % BAR_COLORS.length];
    wx.setNavigationBarColor(c);
  },
  barLoading() {
    const on = !this.data.barLoading;
    this.setData({ barLoading: on });
    if (on) wx.showNavigationBarLoading(); else wx.hideNavigationBarLoading();
  }
});
