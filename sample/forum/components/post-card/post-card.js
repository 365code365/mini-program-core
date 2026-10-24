// 帖子卡片：只负责画与抛事件，点赞 / 收藏的乐观更新由页面做（页面持有列表数据）
Component({
  properties: { item: { type: Object, value: {} }, showCircle: { type: Boolean, value: true } },
  methods: {
    open() { this.triggerEvent('open', { id: this.data.item.id }); },
    like() { this.triggerEvent('like', { id: this.data.item.id }); },
    fav() { this.triggerEvent('fav', { id: this.data.item.id }); },
    circle() { this.triggerEvent('circle', { id: this.data.item.id }); }
  }
});
