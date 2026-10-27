            // ==================== 小游戏：InnerAudioContext 接 game/audio（game-doc/02 §8） ====================
            // audio.js 的 __makeAudio 按名字调 __native_audio（调用时才查全局），这里把它指到小游戏的混音器；
            // 事件仍由原生调 __resolveAudio(id, type, json)，监听表、属性读写、背景音频 / 旧接口全部沿用。
            if (typeof __game_audio === 'function') globalThis.__native_audio = __game_audio;
            // wx.setInnerAudioOption：三项记进原生（game/audio/session.rs），由宿主落到 Android 音频焦点 / iOS AVAudioSession；
            // 回调形状沿用 audio.js 的 __syncOk（success / complete，返回 Promise）
            if (typeof __game_audio_session === 'function') {
                wx.setInnerAudioOption = function(o) {
                    o = o || {};
                    var a = {};
                    ['mixWithOther', 'obeyMuteSwitch', 'speakerOn'].forEach(function(k) { if (typeof o[k] === 'boolean') a[k] = o[k]; });
                    __game_audio_session(JSON.stringify(a));
                    return __syncOk(o, 'setInnerAudioOption');
                };
            }
