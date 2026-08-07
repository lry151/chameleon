# 采用 Fluent Design 视觉语言与 Hybrid 主题策略，并承诺 6 个月视觉冻结

chameleon 历史经历了 v1 → apple-design 重构 → Mica/Acrylic 引入 → 按钮层级优化 → 细节打磨 的多次视觉迭代，每次改完都不满意。根因诊断：(1) 没有稳定的视觉锚点（每次从零判断）；(2) Rust 端 `apply_mica` 启动后不感知主题切换，Win10 `apply_acrylic` 硬编码深色 RGBA `(33,31,41,180)`，导致浅色模式下仍显示深色背景；(3) 熟悉度疲劳（每 3 周就觉得丑）。经过 grilling 决议：(1) 视觉语言采用 Fluent Design（WinUI 3 风格），通过 Naive UI 的 `n-config-provider` theme override 注入 Fluent tokens 实现——视觉风格的核心诉求是「一致即可」，Fluent 是与 Windows 原生 Mica 同向走的最自然选择；(2) 主题采用 Hybrid 策略——深色模式保留原生 Mica 半透，浅色模式切换为实色不透明背景（参考 Windows 11 自家设置应用的做法）；(3) 承诺 6 个月视觉冻结。

**Considered Options**: Apple HIG——否决：与 Windows 原生 Mica 冲突，Windows 上只能做到 60% 像，剩下 40% 显廉价；Material Design 3——否决：与 Mica 不搭，需纯 CSS 重写；Neutral minimal (shadcn/Radix)——否决：浪费已投入的 `window-vibrancy` 原生效果；全程云母（Adaptive Mica）——否决：Win10 Acrylic 不支持 `SetPreferredAppMode` API，浅色云母视觉过淡；全程实色——否决：丢掉了已做对的 Win11 原生质感。

**Consequences**:
- Naive UI 通过 `n-config-provider` theme override 注入 Fluent tokens（4/8px radius、4px spacing grid、Accent/Neutral/Subtle 色板）
- Rust 端 `set_ui_preferences` 命令扩展：切换主题时同步调用 `clear_vibrancy` + 重新 `apply_mica` 或 `apply_acrylic`（浅色主题 Win10 用浅色 Acrylic tint，不再是硬编码深色）
- 保留 `Microsoft YaHei` 字体（中文场景比 Segoe UI Variable 自然，中文回退不会混排）
- 保留 🦎 emoji logo（产品标识，非 Fluent 范畴）
- **Windows 10 一等公民**：Windows 10 是主力用户群（与 Windows 11 并列，不是 fallback 或二等公民）。Hybrid 主题在 Win10 上必须表现良好：深色 Acrylic tint 需仔细调校视觉质量（不能「凑合」）；浅色实色背景在 Win10 上必须与 Win11 表现一致。所有设计决策与测试覆盖必须同等对待 Win10 + Win11 两套环境。
- **6 个月视觉冻结承诺**：本 ADR 采纳后，除非满足以下条件之一，不做任何视觉层改动：
  - 用户明确报告「无法完成某任务」（不是「不好看」）
  - 无障碍审计发现对比度/焦点问题
- 「想再美化一下」的冲动记录到 `docs/future-design-ideas.md` 但不执行
- 历史证据表明，没有冻结承诺的重设计会无限循环，且每次循环降低用户与开发者信任
