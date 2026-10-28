// e-phaser（G9.1）：Phaser 3.80.1，query type=canvas | webgl（默认 webgl）—— 图形、文字、物理下落的方块、点按（pointerdown）
var T = require('_lib/t.js');
var canvas = require('_lib/adapter.js');
var Phaser = require('./libs/phaser.js');
var info = wx.getSystemInfoSync();
var n = 0, cnt = 0, last = Date.now();
var game = new Phaser.Game({
    type: T.q.type === 'canvas' ? Phaser.CANVAS : Phaser.WEBGL, canvas: canvas, width: info.windowWidth, height: info.windowHeight,
    backgroundColor: '#203040', audio: { noAudio: true }, banner: false,
    physics: { default: 'arcade', arcade: { gravity: { y: 300 } } },
    scene: {
        create: function () {
            this.add.rectangle(140, 180, 200, 120, 0xf0a020);
            this.add.circle(260, 380, 60, 0x40c0ff);
            this.add.text(40, 40, 'Phaser 小游戏', { fontSize: '28px', color: '#ffffff' });
            var b = this.add.rectangle(100, 100, 50, 50, 0x00ff00); this.physics.add.existing(b);
            this.box = b;
            this.input.on('pointerdown', function (p) { b.fillColor = 0xff0000; T.ev('tap', { x: Math.round(p.x), y: Math.round(p.y) }); });
            T.ev('phaser', { ver: Phaser.VERSION, renderer: game.renderer.type === Phaser.WEBGL ? 'webgl' : 'canvas', w: canvas.width, h: canvas.height });
        },
        update: function () { n++; cnt++;
            if (n === 20) T.ev('frames', { n: n, y: Math.round(this.box.y) });
            if (Date.now() - last >= 1000) { T.ev('fps', { fps: cnt }); cnt = 0; last = Date.now(); } }
    }
});
T.ready('e-phaser');
