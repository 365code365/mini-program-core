// 图片详情：大图（点开全屏预览，urls 为发现页当前列表）、作者、标签、点赞 / 收藏（storage）、保存到相册、相关推荐
const { request } = require('../../utils/api.js');
const app = getApp();

Page({
  data: { status: 'loading', p: null, imgH: 0, liked: false, fav: false, likes: 0, following: false, related: [], saving: false },
  onLoad(q) {
    this.id = Number(q.id) || 1;
    this.load();
  },
  load() {
    this.setData({ status: 'loading' });
    request('/api/demo/photos/' + this.id, null, { silent: true }).then((p) => {
      const liked = app.isLiked(p.id);
      this.setData({
        status: 'ok', p, liked, fav: app.isFav(p.id), likes: p.likes + (liked ? 1 : 0),
        // 大图按真实宽高比铺满屏宽，太长的竖图限高（再长就点开预览看）
        imgH: Math.min(1100, Math.round(750 * p.ratio)),
        related: p.related.map((r) => ({ id: r.id, url: r.url, title: r.title, color: r.color }))
      });
      wx.setNavigationBarTitle({ title: p.title });
    }).catch((err) => {
      this.setData({ status: 'error', errMsg: err.statusCode === 404 ? '这张图片不存在' : '加载失败，请检查网络' });
    });
  },
  onRetry() { this.load(); },
  onPreview() {
    const cur = this.data.p.url;
    let urls = app.globalData.previewUrls || [];
    // 从收藏页 / 相关推荐进来的，当前图不一定在发现页列表里：用「当前 + 相关」兜底
    if (urls.indexOf(cur) < 0) urls = [cur].concat(this.data.related.map((r) => r.url));
    wx.previewImage({ urls, current: cur });
  },
  onLike() {
    const on = app.toggleLike(this.data.p.id);
    this.setData({ liked: on, likes: this.data.p.likes + (on ? 1 : 0) });
  },
  onFav() {
    const on = app.toggleFav(this.data.p);
    this.setData({ fav: on });
    wx.showToast({ title: on ? '已收藏' : '已取消收藏', icon: 'none' });
  },
  onFollow() {
    this.setData({ following: !this.data.following });
  },
  onTag(e) {
    app.globalData.pendingTag = e.currentTarget.dataset.tag;
    wx.switchTab({ url: '/pages/index/index' });
  },
  // 保存：先 downloadFile 拿本地临时文件，再存相册。两步各自可能失败（网络 / 没给相册权限），提示要分清
  onSave() {
    if (this.data.saving) return;
    this.setData({ saving: true });
    wx.showLoading({ title: '下载中', mask: true });
    wx.downloadFile({
      url: this.data.p.url,
      success: (res) => {
        if (res.statusCode && res.statusCode !== 200) {
          wx.hideLoading();
          this.setData({ saving: false });
          wx.showToast({ title: '下载失败（HTTP ' + res.statusCode + '）', icon: 'none' });
          return;
        }
        wx.saveImageToPhotosAlbum({
          filePath: res.tempFilePath,
          success: () => wx.showToast({ title: '已保存到相册', icon: 'success' }),
          fail: (err) => {
            const msg = (err && err.errMsg) || '';
            if (/auth|deny|denied|权限/.test(msg)) {
              wx.showModal({ title: '无法保存', content: '没有相册权限，请在设置里允许「拾光图库」访问相册', showCancel: false });
            } else {
              wx.showToast({ title: '保存失败：' + (msg.replace(/^saveImageToPhotosAlbum:fail\s*/, '') || '宿主不支持'), icon: 'none' });
            }
          },
          complete: () => { wx.hideLoading(); this.setData({ saving: false }); }
        });
      },
      fail: (err) => {
        wx.hideLoading();
        this.setData({ saving: false });
        wx.showToast({ title: '下载失败：' + ((err && err.errMsg) || '网络错误'), icon: 'none' });
      }
    });
  },
  onRelated(e) {
    wx.redirectTo({ url: '/pages/detail/detail?id=' + e.currentTarget.dataset.id });
  }
});
