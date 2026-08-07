---
target: ui/src/views/MainView.vue
total_score: 21
max_score: 40
na_heuristics: 
p0_count: 1
p1_count: 2
timestamp: 2026-08-07T06-43-02Z
slug: ui-src-views-mainview-vue
---
# chameleon（变色龙）设计评审

Method: dual-agent (A: CritiqueA_DesignReview · B: CritiqueB_Detector)

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 2 | 运行状态仅小型 pill + CDP 端口；批量启动/关闭无结果反馈（成功几个/失败几个） |
| 2 | Match System / Real World | 3 | 术语一致（接力/快照/沙箱对齐 CONTEXT.md），但「并行/接力模式」无通俗解释 |
| 3 | User Control and Freedom | 2 | 「关闭所有」「导入」无确认/撤销；删除确认层级与破坏程度不匹配 |
| 4 | Consistency and Standards | 3 | dialog 宽度/按钮/表单统一，但删除确认两种实现混用（popconfirm vs DeleteConfirm） |
| 5 | Error Prevention | 2 | 关闭所有零确认；导入零确认；重名/URL 格式/选择器无校验 |
| 6 | Recognition Rather Than Recall | 3 | 色块常驻可辨、quick link 直接暴露；但预设 name=url，识别全靠 URL |
| 7 | Flexibility and Efficiency of Use | 2 | 无快捷键、无拖拽排序、无批量选择；power user 逐个点击 |
| 8 | Aesthetic and Minimalist Design | 2 | Topbar 11+ 按钮墙；窗口控制用字符(─/□/×)非图标 |
| 9 | Error Recovery | 1 | 所有错误仅 console.error，n-message-provider 已包裹但从未调用，用户零感知 |
| 10 | Help and Documentation | 1 | 仅空状态一行引导；接力/沙箱/CSS 选择器全零解释 |
| **Total** | | **21/40** | **Needs work** |

## Design Specificity Verdict

**LLM**: 功能与领域术语高度具体（CDP 端口、色块、登录辅助字段），但视觉上是「通用 Naive UI 管理面板 + 中文标签」。变色龙隐喻只在 🦎 商标区被陈述，从未进入构图、动效或情绪。色块身份系统是唯一强设计线索（luma-aware 文字色），但被 Naive UI 卡片 chrome 淹没。

**Deterministic scan**: detector 在 `src-tauri/www/` 发现 4 条 warning（side-tab accent border、layout-transition），全部映射回 Naive UI 依赖内部样式，**0 条真实应用问题**。应用源码无副作用 border、无 layout 属性动画。

**Browser evidence**: 无 console 错误；深色对比度全部通过（body #E8E8E8 on dark Mica ~12.6:1，主按钮 #1abc9c 黑字 ~8.3:1）。透明 body 是 Tauri 原生 Mica 设计，非缺陷。

## Overall Impression

功能逻辑扎实、术语严谨、深色可用性已修好；但「工具面板」质感压过了产品个性，且**错误反馈完全静默**是最大短板。最大机会：把「变色龙色块」从 12px 圆点升级为贯穿运行状态的产品级视觉主线。

## What's Working

- **色块身份系统**（RoleCard `readableOn` luma-aware 文字色）：无论角色颜色深浅，运行标签始终可读；swatch+名称+端口在头部构成紧凑的 at-a-glance 身份。
- **领域模型→UI 映射干净**：系统分组→角色卡片即测试人员的心智模型（按被测系统，再按角色）。
- **系统级删除保护有设计**：两级删除（仅系统 / 系统+角色）用独立 DeleteConfirm + 红色确认按钮。

## Priority Issues

**[P0] 所有错误静默吞入 console.error，用户零感知** — RoleDialog/SystemDialog/HandoffDialog/LinksDialog/SandboxesPanel/SnapshotsPanel/Topbar 所有 catch 块仅 console.error；`n-message-provider` 已包裹但从未调用。失败时 loading 消失、状态不变，无处诊断。→ 每个 catch 调 `useMessage().error()`，成功操作调 `success()`。`$impeccable clarify`

**[P1] 「关闭所有」无确认直接执行** — Topbar:159 直接调 `tauri.closeAll()`，一次误点关闭全部精心布置的测试窗口。对比「清理」有 popconfirm、「删除系统+角色」有 DeleteConfirm——保护等级与破坏程度严重不匹配。「导入」同样零确认。→ 加确认，文案写明影响范围「确定关闭全部 N 个运行中角色窗口？」。`$impeccable harden`

**[P1] Topbar 按钮墙——11+ 按钮水平排列无层级** — 新建角色/系统/启动所有/关闭所有/沙箱/快照/清理/导出/导入/设置 + 窗口控制，全部同视觉权重。→ 拆为品牌+主操作 / 上下文相关(有运行角色才显示关闭所有) / 工具 dropdown。`$impeccable distill`

**[P2] 「并行模式/接力模式」零解释** — 产品独有领域概念，选择错误会引发非预期窗口关闭或双窗并排。→ radio 旁加一行说明。`$impeccable clarify`

**[P2] LinksDialog 表单+列表混装，认知负荷过重** — 520px modal 塞 7 个表单字段 + 预设列表；且预设 name=url，无法起有意义的名字。→ 拆分区域，预设加可编辑 name。`$impeccable layout`

## Persona Red Flags

**Alex（赶工的一线测试工程师）**: 「关闭所有」无确认一次误点丢全部现场；Topbar 按钮墙要搜索目标；「启动所有」无结果反馈，不知几成功几失败。

**Jordan（细致的测试组长）**: 所有错误仅 console.error，launchRole 失败只看 loading 消失无原因；「并行/接力模式」黑话需猜；CSS 选择器输入零引导；快照恢复无时间戳/预览，无法判断恢复哪个。

## Minor Observations

- Topbar 窗口控制用字符(─/□/×)非 SVG，多 DPI 下可能模糊
- RoleDialog 8 色与 SettingsDialog 6 色两个调色板完全独立、硬编码
- LinksDialog 预设 name=url，违背「命名预设」初衷
- 快照恢复是破坏性操作，仅 popconfirm 偏轻
- SandboxesPanel 沙箱 ID 只显示前 8 字符，难区分
- BrowserBar.vue 已实现但未被引用——死代码
- 所有 dialog mask-closable=true，编辑中误点遮罩丢失未保存更改

## Questions to Consider

- 🦎 emoji 承担了全部品牌工作——色块隐喻能否成为屏幕上的主导视觉（运行角色卡片厚色边框/左栏）而非 12px 圆点？
- 「关闭所有」爆炸半径等同「删除全部角色」却零防护——是有意为之还是疏漏？若非有意，shift+click 模式能否兼顾速度与安全？
- n-message-provider 接线了却从未用——团队是否知道界面一直在零错误反馈下发布？
- LinksDialog 以 URL 为主键（name=url）——是 Rust 数据模型约束还是 UI 偷懒？这根本动摇了「常用 URL 预设」概念。
- Topbar 无响应式折叠——1366px 企业笔记本上 11 个按钮如何排布？无 @media 查询。
