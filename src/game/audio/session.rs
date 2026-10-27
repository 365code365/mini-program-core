//! 音频会话（game-doc/02 §8.1、04 G5.5）：系统打断与 `wx.setInnerAudioOption`。
//!
//! - 打断开始（来电、闹钟、别的 App 拿走独占焦点；宿主经 `mr_game_audio_interruption(1)` 告诉引擎）：
//!   在播的 InnerAudio 全部 `pause`、混音器挂起、running 的 WebAudioContext 变成 `suspended`（发 statechange），
//!   然后才派发 `wx.onAudioInterruptionBegin`（游戏在回调里读到的已经是暂停后的状态）；
//! - 打断期间 `play()` / `WebAudioContext.resume()` 不生效（官方：收到结束事件之后才可再次播放成功）；
//! - 打断结束：混音器恢复（不在后台时），派发 `wx.onAudioInterruptionEnd`。**不自动续播**，由游戏在回调里自己 play / resume；
//! - setInnerAudioOption 的三项交给宿主落到平台：Android 的 `mixWithOther: false` = 申请独占焦点（别的音乐 App 停）、
//!   true = 不抢焦点；iOS 的 `obeyMuteSwitch` 决定 AVAudioSession 类别（遵守静音键 ambient，否则 playback）、
//!   `mixWithOther` 决定 `.mixWithOthers`、`speakerOn` 决定 `.defaultToSpeaker`。引擎自己只记下来、发 `audioSession` 宿主事件。
//!
//! 打断时序、打断期间 play 的回调形状（失败还是静默）、默认值都按官方文档的说法做，**待真机取证**（D-7）。

use super::mixer::Cmd;
use super::GameAudio;
use serde_json::{json, Value};

/// `wx.setInnerAudioOption` 的三项（官方默认 mixWithOther / obeyMuteSwitch = true、speakerOn = true）
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionOptions {
    pub mix_with_other: bool,
    pub obey_mute_switch: bool,
    pub speaker_on: bool,
}

impl Default for SessionOptions {
    fn default() -> Self {
        Self { mix_with_other: true, obey_mute_switch: true, speaker_on: true }
    }
}

impl SessionOptions {
    pub fn to_json(self) -> Value {
        json!({ "type": "audioSession", "mixWithOther": self.mix_with_other, "obeyMuteSwitch": self.obey_mute_switch, "speakerOn": self.speaker_on })
    }
}

impl GameAudio {
    /// 混音器挂不挂：后台或打断中都挂
    pub(super) fn apply_suspend(&mut self) {
        let s = self.suspended || self.interrupted;
        self.send(Cmd::Suspend(s));
    }

    /// 系统打断开始 / 结束。返回状态有没有变（重复的开始 / 结束不再派发）
    pub fn set_interrupted(&mut self, on: bool) -> bool {
        if on == self.interrupted {
            return false;
        }
        if on {
            let playing: Vec<u32> = self.ctxs.iter().filter(|(_, c)| c.playing).map(|(&id, _)| id).collect();
            for id in playing {
                self.pause(id);
            }
            // 等着解码完就开播的那些也不开了
            for c in self.ctxs.values_mut() {
                c.play_when_ready = false;
            }
            self.web_interrupt();
        }
        self.interrupted = on;
        self.apply_suspend();
        let msg = if on { "🔇 音频被系统打断：全部暂停" } else { "🔈 音频打断结束（不自动续播）" };
        self.logs.push(msg.into());
        true
    }

    pub fn interrupted(&self) -> bool {
        self.interrupted
    }

    /// `wx.setInnerAudioOption`：只认布尔值，没给的保持原样；返回要交给宿主的事件
    pub fn set_session_options(&mut self, a: &Value) -> Value {
        let b = |k: &str| a.get(k).and_then(Value::as_bool);
        let o = &mut self.session;
        if let Some(v) = b("mixWithOther") {
            o.mix_with_other = v;
        }
        if let Some(v) = b("obeyMuteSwitch") {
            o.obey_mute_switch = v;
        }
        if let Some(v) = b("speakerOn") {
            o.speaker_on = v;
        }
        self.session_dirty = true;
        self.session.to_json()
    }

    /// 改过的会话设置（宿主事件 `audioSession`），每次改动交一次
    pub fn take_session_event(&mut self) -> Option<Value> {
        std::mem::take(&mut self.session_dirty).then(|| self.session.to_json())
    }

    pub fn session_options(&self) -> SessionOptions {
        self.session
    }
}
