// <open-data>（官方 component/open-data，按「小程序用户信息相关接口调整」之后的现状）：
// userNickName 显示「微信用户」、userAvatarUrl 显示灰色头像（铺满组件；官方灰头像上的白色人形没画）；
// userGender / userCity / userProvince / userCountry / userLanguage 不再返回，显示为空；
// groupName 要宿主拉群名，引擎拿不到，也为空。为空时显示 default-text，并发 bind:error（官方：群名称或用户信息为空时触发；
// detail 官方没写，给 {type}）。default-avatar：官方是「头像为空时」用，而现状是灰头像不为空，所以不用它（待验）。
Component({
  properties: {
    type: { type: String, value: '' },
    openGid: { type: String, value: '' },
    lang: { type: String, value: 'en' },
    defaultText: { type: String, value: '' },
    defaultAvatar: { type: String, value: '' }
  },
  data: { avatar: false, text: '' },
  observers: {
    'type, defaultText': function () { this._sync(); }
  },
  lifetimes: {
    attached: function () { this._sync(); }
  },
  methods: {
    _sync: function () {
      var type = this.properties.type, dt = this.properties.defaultText;
      var avatar = type === 'userAvatarUrl';
      var text = type === 'userNickName' ? '微信用户' : '';
      var empty = !avatar && !text;
      var shown = empty ? (dt || '') : text;
      if (avatar !== this.data.avatar || shown !== this.data.text) this.setData({ avatar: avatar, text: shown });
      if (empty && type && this._reported !== type) {
        this._reported = type;
        var self = this;
        setTimeout(function () { self.triggerEvent('error', { type: type }); }, 0);
      }
    }
  }
});
