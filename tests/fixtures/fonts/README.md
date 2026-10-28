# 测试用字体

- `NotoColorEmoji-subset.ttf`：Noto Color Emoji（COLRv1，取自 Android 系统字体 `/system/fonts/NotoColorEmoji.ttf`）的子集，
  只留 💰📅💧🌱🐛😀🔥❤⭐🌈🎉🍎 十二个字形，给 `src/emoji_colr` 的单测用（13KB）。
  Copyright 2022 Google Inc.，按 SIL Open Font License 1.1 授权（https://openfontlicense.org），允许随软件再分发。
  重新生成：fontTools `subset.Subsetter` 对原字体按上面的字符取子集（`layout_features=['*']`、`name_IDs=['*']`）。
