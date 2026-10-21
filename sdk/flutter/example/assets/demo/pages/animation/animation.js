Page({
  data: {
    anim: null
  },
  onLoad() {
    this.a = wx.createAnimation({ duration: 500, timingFunction: 'ease' });
    // 记住累计位置：每次动画都从上一次的终态接着走
    this.x = 0;
    this.deg = 0;
    this.s = 1;
  },
  play() {
    this.setData({ anim: this.a.export() });
  },
  move() {
    this.x = this.x === 0 ? 180 : 0;
    this.a.translateX(this.x).step();
    this.play();
  },
  rotate() {
    this.deg += 180;
    this.a.rotate(this.deg).step({ duration: 700 });
    this.play();
  },
  scale() {
    this.s = this.s === 1 ? 1.6 : 1;
    this.a.scale(this.s).step({ timingFunction: 'ease-out' });
    this.play();
  },
  combo() {
    this.a.translateX(160).step({ duration: 400 });
    this.a.rotate(this.deg + 90).scale(1.3).step({ duration: 400 });
    this.a.translateX(0).scale(1).step({ duration: 400 });
    this.deg += 90;
    this.x = 0;
    this.s = 1;
    this.play();
  },
  fade() {
    this.a.opacity(0.2).backgroundColor('#e64340').step({ duration: 400 });
    this.a.opacity(1).backgroundColor('#07c160').step({ duration: 400, delay: 200 });
    this.play();
  },
  reset() {
    this.x = 0;
    this.deg = 0;
    this.s = 1;
    this.a.translateX(0).rotate(0).scale(1).opacity(1).backgroundColor('#07c160').step({ duration: 300 });
    this.play();
  }
});
