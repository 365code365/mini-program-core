const logBehavior = require('../behaviors/log.js');

// 受控计数器：值由页面传进来（properties.value），点按钮只 triggerEvent，页面 setData 后再流回来
Component({
  behaviors: [logBehavior],
  properties: {
    label: { type: String, value: '计数' },
    value: { type: Number, value: 0 },
    step: { type: Number, value: 1 },
    max: { type: Number, value: 10 }
  },
  data: {
    atMax: false,
    atMin: true
  },
  observers: {
    // 多个字段一起监听：任何一个变了都会进来
    'value, max': function (value, max) {
      this.setData({ atMax: value >= max, atMin: value <= 0 });
    }
  },
  lifetimes: {
    attached() { this.log('lifetimes.attached'); },
    ready() { this.log('lifetimes.ready'); },
    detached() { console.log('demo-counter detached：' + this.data.label); }
  },
  pageLifetimes: {
    show() { this.log('pageLifetimes.show'); },
    hide() { this.log('pageLifetimes.hide'); }
  },
  methods: {
    inc() {
      if (this.data.atMax) return;
      this.emit(Math.min(this.data.value + this.data.step, this.data.max));
    },
    dec() {
      if (this.data.atMin) return;
      this.emit(Math.max(this.data.value - this.data.step, 0));
    },
    // 页面通过 selectComponent('#id').reset() 直接调
    reset() {
      this.emit(0);
      this.log('reset() 被页面调用');
    },
    emit(value) {
      this.triggerEvent('change', { value, label: this.data.label });
      this.log('triggerEvent change ' + value);
    }
  }
});
