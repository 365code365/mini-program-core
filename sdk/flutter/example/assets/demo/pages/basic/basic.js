Page({
  data: {
    chips: ['view', 'text', 'rich-text', 'icon', 'image', 'progress', 'button', 'swiper', 'scroll-view'],
    icons: ['success', 'success_no_circle', 'info', 'warn', 'waiting', 'cancel', 'download', 'search', 'clear',
      'circle', 'close', 'arrow', 'back', 'plus', 'minus', 'star', 'star_o', 'heart', 'heart_o', 'chat'],
    percent: 40,
    nodes: [
      { name: 'span', children: [{ type: 'text', text: 'rich-text 支持' }] },
      { name: 'b', children: [{ type: 'text', text: '加粗' }] },
      { name: 'span', children: [{ type: 'text', text: '、' }] },
      { name: 'span', attrs: { style: 'color: #e64340;' }, children: [{ type: 'text', text: '红色' }] },
      { name: 'span', children: [{ type: 'text', text: '、' }] },
      { name: 'span', attrs: { style: 'font-size: 20px; color: #07c160;' }, children: [{ type: 'text', text: '大字号' }] },
      { name: 'span', children: [{ type: 'text', text: '，嵌套节点会继承外层的样式：' }] },
      {
        name: 'span', attrs: { style: 'color: #576b95;' },
        children: [
          { type: 'text', text: '蓝色里面 ' },
          { name: 'strong', children: [{ type: 'text', text: '再加粗' }] }
        ]
      }
    ]
  },
  inc() { this.setData({ percent: Math.min(100, this.data.percent + 10) }); },
  dec() { this.setData({ percent: Math.max(0, this.data.percent - 10) }); }
});
