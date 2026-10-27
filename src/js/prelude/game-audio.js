            // ==================== 小游戏：InnerAudioContext 接 game/audio（game-doc/02 §8） ====================
            // audio.js 的 __makeAudio 按名字调 __native_audio（调用时才查全局），这里把它指到小游戏的混音器；
            // 事件仍由原生调 __resolveAudio(id, type, json)，监听表、属性读写、背景音频 / 旧接口全部沿用。
            if (typeof __game_audio === 'function') globalThis.__native_audio = __game_audio;
