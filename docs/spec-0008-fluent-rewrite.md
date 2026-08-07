# 前端 Full Rewrite：Vue 3 + Naive UI + Fluent + Hybrid 主题 + 布局 / 线性交互重构

## Problem Statement

chameleon 前端当前存在四个相互关联的问题：

1. **结构布局混乱**：Quick Links 表单把 URL 输入框、两个 checkbox、添加按钮挤在同一行（`display: flex` 水平布局）；角色卡内 name / swatch / badge / port / preset-chips / actions 平级竞争焦点；系统容器与角色卡之间的嵌套关系在视觉上不够清晰。

2. **交互流非线性**：每个视图多个操作平级呈现，没有明确的"主操作 vs 次操作"层级；删除与编辑 / 克隆等动作混在同一个 actions 行；接力流程需要用户在 dialog 里做多次选择但流程分支不清晰；表单填写方向不一致（有时横向、有时纵向）。

3. **浅色模式深色背景 bug**：Rust 端 `apply_mica` / `apply_acrylic` 只在启动时调用一次，不感知主题切换。Win11 上 Mica 跟随系统 app mode 但前端切主题时未同步；Win10 上 `apply_acrylic(&window, Some((33, 31, 41, 180)))` 硬编码深色 RGBA。结果：浅色模式下窗口整体仍显深色。

4. **vanilla 前端已到维护极限**：单文件 `app.js` 25KB，每次视觉调整牵一发动全身；历史证据（v1 → apple-design 重构 → Mica → 按钮层级 → 细节打磨）表明无框架约束下设计决策必然漂移。

**根因诊断**（详见 ADR-0010）：真正的不满不是视觉风格，而是布局与交互流未定型；视觉风格只要一致即可。

## Solution

用 Vue 3 + Vite + TypeScript + Naive UI 在 `/ui/` 子目录重写前端，构建产物输出到 `src-tauri/www/`。视觉语言采用 Fluent Design（通过 Naive UI theme override 注入 Fluent tokens）；主题采用 Hybrid 策略（深色 = 云母半透；浅色 = 实色不透明）；布局严格遵循垂直栈 + 卡片头 / 中 / 尾三段结构；交互严格遵循线性流程原则。Big Bang 一次性重写，按页面粒度拆 ticket。

### 核心设计原则（来自 ADR-0010）

**结构布局**：
1. 每个视图有且仅有一个主要焦点
2. 相关元素分组，无关元素分离
3. 垂直流优先（自上而下阅读顺序）
4. 留白是结构的一部分
5. 卡片内部结构固定：**头部（标识 + 状态）→ 中部（内容）→ 底部（操作）**

**线性交互**：
1. 每个流程 = 一条直线：step 1 → step 2 → step 3，无分支
2. 每个视图只暴露一个主操作
3. 表单字段垂直堆叠，从上到下填完即提交
4. 删除 / 危险操作 = 独立的确认步骤
5. 不引入「模式」切换

**Windows 10 一等公民**：
Windows 10 是主力用户群（与 Windows 11 并列，不是 fallback）。Hybrid 主题在 Win10 上必须表现良好：深色 Acrylic tint 需仔细调校视觉质量（不能「凑合」）；浅色实色背景在 Win10 上必须与 Win11 表现一致。所有设计决策与测试覆盖必须同等对待 Win10 + Win11 两套环境。

## User Stories

复用 spec-0007 的 20 条 user stories（拖动 / Mica / Acrylic / 主题切换 / 透明度 / accent / 持久化 / 导出导入 / 旧配置兼容 / 设置面板 / 浅色主题可读性 等）。新增：

21. As a 测试人员, I want 每个视图的主操作一眼可见, so that 我不必扫视全图寻找下一步该做什么.

22. As a 测试人员, I want 表单字段垂直堆叠、从上到下填完即提交, so that 表单填写符合自然阅读顺序.

23. As a 测试人员, I want 危险操作（删除、批量关闭）有独立的二次确认步骤, so that 我不会在主流程中误触.

24. As a 测试人员, I want 角色卡内部结构（头 / 中 / 尾）在所有卡片中一致, so that 我看一张卡就懂所有卡.

25. As a 测试人员, I want 浅色模式下窗口背景是实色浅色（不是云母深色）, so that 在明亮环境下内容清晰可读.

26. As a 测试人员, I want UI 在 6 个月内保持稳定, so that 我不必每次更新后重新学习界面.

27. As a 使用 Windows 10 的测试人员, I want 本工具在 Win10 上的视觉质量与 Win11 一致（深色 Acrylic 精心调校、浅色实色背景相同）, so that 我不因操作系统版本而感到二等公民.

## Implementation Decisions

### 模块划分

**`ui/` 子目录（Vite + Vue 3 + TS + Naive UI）**：
```
ui/
├── index.html
├── package.json
├── pnpm-lock.yaml
├── tsconfig.json
├── vite.config.ts
├── src/
│   ├── main.ts              # 入口：createApp + Naive UI 注册
│   ├── App.vue              # 根组件：n-config-provider + 主布局
│   ├── theme/
│   │   ├── fluent.ts        # Fluent tokens → Naive UI theme override
│   │   └── hybrid.ts        # Hybrid 主题：dark=云母 / light=实色
│   ├── composables/
│   │   ├── useTauri.ts      # invoke 包装
│   │   ├── usePrefs.ts      # UI 偏好读写
│   │   └── useTheme.ts      # 主题切换 + Rust 端 vibrancy 同步
│   ├── components/
│   │   ├── Topbar.vue       # 顶部工具栏
│   │   ├── SystemBox.vue    # 系统容器
│   │   ├── RoleCard.vue     # 角色卡
│   │   ├── BrowserBar.vue   # 浏览器选择栏
│   │   ├── SettingsDialog.vue
│   │   ├── SystemDialog.vue
│   │   ├── RoleDialog.vue
│   │   ├── HandoffDialog.vue
│   │   ├── LinksDialog.vue  # Quick Links 管理（垂直栈表单）
│   │   ├── SandboxesPanel.vue
│   │   ├── SnapshotsPanel.vue
│   │   └── DeleteConfirm.vue # 通用危险操作确认
│   ├── views/
│   │   └── MainView.vue     # 主视图（main 区域）
│   └── types/
│       └── api.ts           # Tauri 命令类型
└── dist/                    # 构建产物 → 软链或复制到 src-tauri/www/
```

**`src-tauri/` 调整**：
- `tauri.conf.json` 新增 `beforeDevCommand: "pnpm --dir ui dev"` / `beforeBuildCommand: "pnpm --dir ui build"`
- `build.frontendDist` 指向 `www`（保持）
- `src/lib.rs` 扩展 `set_ui_preferences` 命令：主题切换时同步调用 `clear_vibrancy` + 重新应用 Mica/Acrylic

### 数据模型

`UiPreferences` 结构不变：
```rust
pub struct UiPreferences {
    pub theme: ThemeMode,        // Dark | Light | System
    pub panel_opacity: f32,      // 0.5 - 1.0
    pub accent_color: String,    // "#1abc9c"
}
```

仅扩展 `set_ui_preferences` 的副作用：主题切换时同步 Rust 端 vibrancy。

### API 契约

**现有 Tauri 命令（不变）**：
- `get_state`, `launch_role`, `close_role`, `launch_all`, `close_all`
- `launch_sandbox`, `close_sandbox`, `cleanup_temp`
- `save_snapshot`, `restore_snapshot`, `delete_snapshot`
- `create_system`, `update_system`, `delete_system`
- `create_role`, `update_role`, `delete_role`, `clone_role`
- `add_quick_link`, `update_quick_link`, `remove_quick_link`
- `handoff`, `pick_browser`, `get_ui_preferences`, `set_ui_preferences`
- `export_config`, `import_config`, `app_minimize`, `app_maximize`, `app_hide`

**扩展**：
- `set_ui_preferences` 实现增加：当 `prefs.theme` 改变时，调用 `clear_vibrancy` + 按当前系统版本 + 新主题重新 `apply_mica` 或 `apply_acrylic`（浅色主题 Win10 用浅色 tint，不再硬编码深色）

### 视觉语言（Fluent via Naive UI theme override）

`ui/src/theme/fluent.ts` 导出 Fluent tokens 映射：
```ts
// Fluent Design tokens → Naive UI theme override
export const fluentThemeOverride = {
  common: {
    borderRadius: '4px',       // small
    borderRadiusMedium: '8px', // medium
    fontSize: '14px',
    fontFamily: '"Microsoft YaHei", system-ui, sans-serif',
    primaryColor: '#0078D4',   // Fluent accent blue (或保持用户 accent)
    // ...
  },
  // Button, Input, Card, Dialog, Form 等组件级 override
}
```

**注意**：Accent 色（用户可选）通过 Naive UI 的 `themeOverrides.common.primaryColor` 注入，保持 D4 决策（accent 扩散到 primary button / running badge / slider / focus 边框）。

### Hybrid 主题实现

**前端**：
```vue
<n-config-provider :theme="isDark ? darkTheme : null" :theme-overrides="fluentThemeOverride">
  <n-loading-bar-provider>
    <n-dialog-provider>
      <n-message-provider>
        <App />
      </n-message-provider>
    </n-dialog-provider>
  </n-loading-bar-provider>
</n-config-provider>
```

- 深色模式：`theme = darkTheme`，body 背景透明，让原生 Mica 透出
- 浅色模式：`theme = null`（Naive UI 默认 light），body 背景设为 `#F3F3F3`（实色，参考 Windows 11 设置应用）

**Rust 端**（`src-tauri/src/lib.rs` 扩展）：
```rust
// 在 set_ui_preferences 中
if theme_changed {
    let window = app.get_webview_window("main").unwrap();
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{clear_vibrancy, apply_mica, apply_acrylic};
        use windows_version::OsVersion;
        let _ = clear_vibrancy(&window);
        let ver = OsVersion::current();
        match prefs.theme {
            ThemeMode::Dark => {
                if ver.major >= 10 && ver.build >= 22000 {
                    let _ = apply_mica(&window, None);
                } else {
                    let _ = apply_acrylic(&window, Some((33, 31, 41, 180)));
                }
            }
            ThemeMode::Light => {
                // 浅色模式不用 vibrancy，由前端 body 实色背景填充
                // 不调用任何 apply_*，clear_vibrancy 已足够
            }
            ThemeMode::System => {
                // 跟随系统 app mode
                // ... 查询系统当前主题并应用对应策略
            }
        }
    }
}
```

### 布局原则落地

**Topbar（顶部工具栏）**：
```
[🦎 chameleon]  [新建角色] [新建系统] | [启动所有] [关闭所有] [沙箱] [清理] | [导出] [导入] ... [设置] [─] [□] [×]
 ^brand           ^主操作区          ^批量区          ^工具区                                    ^窗口控制
```
- 通过 `n-space` 分组，分组间视觉留白区分
- 主操作（新建角色）用 primary 按钮；其他用 default / subtle

**角色卡（RoleCard）**：固定三段结构
```
┌─────────────────────────────────────┐
│  ● 角色名        [运行中]  :9222   │  ← 头部：swatch + name + badge + port
├─────────────────────────────────────┤
│  [http://...] [http://...]         │  ← 中部：preset chips
│  [http://...]                      │
├─────────────────────────────────────┤
│  [启动] [预设] [接力] [⋯]          │  ← 底部：actions（主操作左对齐，菜单右对齐）
└─────────────────────────────────────┘
```
- 编辑 / 克隆 / 删除放进 `[⋯]` menu，不直接暴露在 actions 行
- 删除 = menu 内条目 + `n-popconfirm` 二次确认

**系统容器（SystemBox）**：
```
┌──────────────────────────────────────┐
│  系统名 (2 个角色)      [⋯] [启动组] │  ← sys-head
├──────────────────────────────────────┤
│  [http://...] [http://...]          │  ← sys-links (Quick Links)
├──────────────────────────────────────┤
│  ┌────────┐ ┌────────┐ ┌────────┐  │  ← role-grid (auto-fill minmax 240px)
│  │RoleCard│ │RoleCard│ │RoleCard│  │
│  └────────┘ └────────┘ └────────┘  │
└──────────────────────────────────────┘
```

**Quick Links 管理（LinksDialog）**：严格垂直栈
```
┌─────────────────────────────────────┐
│  管理预设                            │
├─────────────────────────────────────┤
│  URL                                │
│  [________________________]         │
│                                     │
│  ☐ 启动时自动打开                    │
│  ☐ 含登录辅助                        │
│                                     │
│  （登录辅助展开后，缩进）             │
│    用户名                            │
│    [________________]               │
│    输入框选择器                       │
│    [________________]               │
│    [测试登录]                        │
│                                     │
│  [添加预设]                          │
├─────────────────────────────────────┤
│  已有预设列表                         │
│  ┌─────────────────────────────┐    │
│  │ http://...  ☐启动  [编辑][×]│    │
│  │ http://...          [编辑][×]│    │
│  └─────────────────────────────┘    │
└─────────────────────────────────────┘
```

### 按钮规范（严格 Fluent Button）

| 类型 | Naive UI 用法 | 用途 |
|---|---|---|
| Accent (实心品牌色) | `<n-button type="primary">` | 主操作：新建、启动、添加、保存 |
| Standard (中性) | `<n-button>` | 次操作：预设、接力、导出、导入 |
| Subtle (无底无边) | `<n-button text>` | 低频操作：帮助链接、menu 内条目 |
| Danger | `<n-button type="error">` | 危险操作：删除（仅在确认 dialog 内） |

### 表单规范（严格垂直栈）

- 所有表单使用 `<n-form>` + `<n-form-item label="...">` 默认垂直布局
- 每个字段独占一行
- Checkbox 独占一行（不与 input 同行）
- 提交按钮在最底部，主操作右对齐
- 条件展开的字段缩进一级（左侧 padding 增加 16px）

### 迁移步骤（Big Bang）

1. 在 `ui/` 搭 Vite + Vue 3 + TS + Naive UI 脚手架
2. 实现 Fluent theme override + Hybrid 主题（前端 + Rust 端 vibrancy 同步）
3. 重构 Topbar + 主布局
4. 重构 RoleCard + SystemBox
5. 重构 SettingsDialog（含 accent picker / opacity slider / 主题切换）
6. 重构 LinksDialog（垂直栈表单）
7. 重构其余 dialogs（接力 / 沙箱 / 快照 / 系统编辑 / 角色编辑 / 浏览器选择）
8. 切换 `tauri.conf.json` 指向新 UI + 删除 `src-tauri/www/`
9. 集成测试 + 手动走一遍所有用户流程

每步对应一个 ticket（详见 tickets 列表），按顺序执行。第 8 步之前保持旧 UI 运行；第 8 步完成 = 一次性切换。

## Testing Decisions

### 测试原则

只测外部行为，不测实现细节。复用现有 `ConfigStore` 单元测试 + `tests/integration.rs` 集成测试基础设施。

### 测试模块

**1. `UiPreferences` 序列化（单元测试，沿用）**：
- 默认值正确填充
- 旧配置（无 `ui_preferences`）正常加载
- 新配置正常保存
- 各 `ThemeMode` 序列化

**2. Rust 端 vibrancy 同步（新增单元测试）**：
- 测试 `set_ui_preferences` 主题变化时 vibrancy 重应用逻辑（mock window）
- 测试 Win11 / Win10 / 浅色 / 深色四种组合

**3. 集成测试（沿用 + 扩展）**：
- `tests/integration.rs` 现有用例全绿
- 新增：启动→切主题→切回→验证无崩溃

**4. 前端手动测试清单**：
- 所有 20 + 6 = 26 条 user stories 逐条走一遍
- 浅色 / 深色 / 系统主题切换各走一遍
- Win10 / Win11 各走一遍（Hybrid 主题）
- 旧配置升级（删除 `ui_preferences` 后重启）
- 导出 / 导入含 UI 偏好
- 每个 dialog 的打开 / 关闭 / 提交流程
- 每个危险操作的二次确认

## Out of Scope

以下内容明确不在本次 spec 范围内：

1. **macOS / Linux 原生效果**：本次只针对 Windows（Mica/Acrylic）；macOS / Linux 用实色背景 fallback
2. **高级设置**：字体大小、blur 强度、快捷键、语言切换、自动启动
3. **多窗口主题独立**：不为角色窗口 / 沙箱窗口单独设置主题（它们继承主窗口）
4. **自定义主题编辑器**：用户不能自定义 tokens，只能用 accent 色
5. **动效 / 动画**：仅用 Naive UI 默认过渡，不额外添加
6. **拖拽排序 / 双面板联动 / 复杂交互**：违反线性交互原则（ADR-0010）
7. **6 个月内的任何视觉层改动**：参见 ADR-0009 视觉冻结承诺

## Further Notes

### 风险与缓解

| 风险 | 缓解措施 |
|---|---|
| `window-vibrancy` 浅色模式在某些 Windows 版本行为不一致 | Hybrid 策略：浅色模式完全不用 vibrancy，由前端实色背景填充；规避此风险 |
| Naive UI Fluent theme override 与 Naive UI 默认冲突 | Naive UI theme override 是官方 API；仅覆盖必要 tokens，不强行改所有样式 |
| Big Bang 重写期间旧 UI 不可用 | 旧 `src-tauri/www/` 保留直到第 8 步；第 8 步前 `tauri.conf.json` 仍指向旧 UI |
| Vue 3 学习曲线（用户自承不懂前端框架）| Naive UI 「拿来就用」策略：不深度自定义，跟随默认 + 官方文档 |
| 6 个月视觉冻结承诺难以遵守 | 设立 `docs/future-design-ideas.md` 作为冲动缓冲区；PR 模板加 checklist 自检 |

### 工时估计

| 步骤 | 估计 |
|---|---|
| 1. `ui/` 脚手架 | 2-3h |
| 2. Fluent theme + Hybrid 主题（含 Rust 端） | 4-5h |
| 3. Topbar + 主布局 | 2-3h |
| 4. RoleCard + SystemBox | 3-4h |
| 5. SettingsDialog | 2-3h |
| 6. LinksDialog（垂直栈表单） | 2-3h |
| 7. 其余 dialogs | 4-5h |
| 8. 切换 + 清理 | 1-2h |
| 9. 集成测试 + 手动验证 | 2-3h |
| **总计** | **22-31h（3-5 个工作日）** |

### 依赖变更

**新增**（`ui/package.json`）：
- `vue` ^3.5
- `@tauri-apps/api` ^2
- `naive-ui` ^2.38
- `vite` ^5
- `typescript` ^5
- `@vicons/fluent` (Fluent System Icons, 仅在需要 icon 时按需引入)

**新增**（`src-tauri/Cargo.toml`）：
- 无（`window-vibrancy` 已存在）

**移除**：
- 第 8 步完成后：整个 `src-tauri/www/` 目录（`index.html` / `app.js` / `style.css` / `logo.svg`）
- `logo.svg` 如需保留，迁移到 `ui/src/assets/`

### 视觉冻结承诺（ADR-0009）

本 spec 实施完成后，团队承诺 6 个月内不做任何视觉层改动，除非：
- 用户明确报告「无法完成某任务」（不是「不好看」）
- 无障碍审计发现对比度 / 焦点问题

「想再美化一下」的冲动记录到 `docs/future-design-ideas.md` 但不执行。
