Page({
  data: {
    banners: [
      { src: '/images/banner.jpg', title: '本地图片 aspectFill' },
      { src: '/images/coffee.jpg', title: '咖啡' },
      { src: '/images/sneakers.jpg', title: '运动鞋' },
      { src: '/images/cake.jpg', title: '蛋糕' }
    ],
    bannerCurrent: 0,
    bannerSource: '',
    colors: ['#07c160', '#10aeff', '#fa9d3b', '#e64340', '#8a5cf5'],
    current: 0,
    duration: 500,
    autoplay: false,
    finishCount: 0,
    products: [
      { name: '咖啡', src: '/images/coffee.jpg' },
      { name: '耳机', src: '/images/earbuds.jpg' },
      { name: '手机', src: '/images/phone.jpg' },
      { name: '手表', src: '/images/watch.jpg' },
      { name: '球鞋', src: '/images/sneakers.jpg' },
      { name: '蛋糕', src: '/images/cake.jpg' }
    ],
    notices: ['swiper 竖向自动播放', '衔接模式从最后一条连续滚到第一条', '外层页面照常可以上下滑动']
  },
  onBanner(e) {
    this.setData({ bannerCurrent: e.detail.current, bannerSource: e.detail.source });
  },
  onCtrl(e) {
    // 手指拖动 / 自动播放翻页时把受控值同步回来，否则下一页会从旧值算、或被拽回去。
    // 页面自己改 current 引起的 change（source 为空串）不回写，免得 setData 循环
    if (e.detail.source) this.setData({ current: e.detail.current });
  },
  onFinish() {
    this.setData({ finishCount: this.data.finishCount + 1 });
  },
  prev() {
    const n = this.data.colors.length;
    this.setData({ current: (this.data.current + n - 1) % n });
  },
  next() {
    this.setData({ current: (this.data.current + 1) % this.data.colors.length });
  },
  jump() {
    this.setData({ current: 0 });
  },
  toggleDuration() {
    this.setData({ duration: this.data.duration === 500 ? 1200 : 500 });
  },
  toggleAutoplay() {
    this.setData({ autoplay: !this.data.autoplay });
  }
});
