# 原生 Mica 效果 + 窗口拖动修复 + UI 偏好设置

## Problem Statement

当前 chameleon 主窗口存在三个相互关联的问题：

1. **窗口拖动失效**：用户无法通过 topbar 拖动窗口。原因是 `transparent(true)` + Windows WebView2 + CSS `backdrop-filter` 的组合导致 HTML 属性 `data-tauri-drag-region` 的命中测试不可靠。

2. **云母效果过于透明**：当前使用 CSS `backdrop-filter: blur(20px)` 模拟 Mica 效果，视觉效果不如 Windows 11 原生 Mica，且透明度太高（`rgba(39, 38, 46, 0.72)`）导致内容难以阅读。

3. **缺少 UI 偏好设置**：用户无法调整主题（深色/浅色）、面板透明度、Accent 颜色等视觉偏好，所有设置硬编码在 CSS 中。

## Solution

采用系统级原生效果替代 CSS 模拟，并提供用户可控的设置面板：

1. **原生 Mica/Acrylic**：使用 `window-vibrancy` crate 调用 Windows 原生 Mica（Win11）或 Acrylic（Win10）效果，替代 CSS `backdrop-filter`。

2. **JS 驱动拖动**：移除 HTML 属性 `data-tauri-drag-region`，改用 JavaScript `window.startDragging()` API 直接告诉操作系统开始拖动，绕过 WebView2 的命中测试问题。

3. **UI 偏好设置面板**：新增设置 dialog，支持主题切换（深色/浅色/跟随系统）、面板透明度滑块（0.5-1.0）、Accent 颜色选择（6 个预设颜色）。设置持久化到 `config.json` 的 `ui_preferences` 字段。

## User Stories

1. As a 测试人员, I want to 拖动主窗口到屏幕任意位置, so that 我可以把窗口放在方便操作的地方，不被其他窗口遮挡。

2. As a 测试人员, I want to 看到清晰的 Windows 11 原生 Mica 效果, so that 窗口背景与桌面融为一体，视觉效果更美观。

3. As a 使用 Windows 10 的测试人员, I want to 看到 Acrylic 模糊效果, so that 即使在旧系统上也有好看的半透明背景。

4. As a 测试人员, I want to 切换深色和浅色主题, so that 在不同光线环境下都能舒适地使用工具。

5. As a 测试人员, I want to 选择主题跟随系统设置, so that 工具自动适应系统的深色/浅色模式切换。

6. As a 测试人员, I want to 调整面板透明度, so that 我可以根据个人喜好控制背景的透明程度。

7. As a 测试人员, I want to 选择 Accent 颜色, so that 按钮和高亮元素符合我的视觉偏好。

8. As a 测试人员, I want to 设置自动保存, so that 我的偏好设置在重启工具后仍然保留。

9. As a 测试人员, I want to 设置可以导出和导入, so that 我可以在多台机器间同步我的偏好设置（随 config.json 一起）。

10. As a 测试人员, I want to 点击 topbar 空白区域拖动窗口, so that 拖动操作直观自然。

11. As a 测试人员, I want to 点击 topbar 上的按钮不会触发拖动, so that 按钮点击操作不被干扰。

12. As a 测试人员, I want to 在 Windows 11 上看到 Mica 效果、在 Windows 10 上看到 Acrylic 效果, so that 不同系统版本都能获得最佳视觉效果。

13. As a 测试人员, I want to 设置面板简洁易用, so that 我不会被过多选项困扰。

14. As a 测试人员, I want to 透明度调整立即生效, so that 我可以实时看到调整效果。

15. As a 测试人员, I want to 主题切换立即生效, so that 我可以实时看到切换效果。

16. As a 测试人员, I want to 旧版本配置文件能正常加载, so that 升级工具后不会丢失我的角色和系统配置。

17. As a 测试人员, I want to 设置面板可以通过 topbar 的齿轮图标打开, so that 设置入口容易找到。

18. As a 测试人员, I want to 设置面板可以关闭, so that 调整完设置后能继续使用主功能。

19. As a 测试人员, I want to 面板在深色主题下使用深色调, so that 整体视觉风格一致。

20. As a 测试人员, I want to 面板在浅色主题下使用浅色调, so that 浅色模式下内容清晰可读。

## Implementation Decisions

### 模块划分

**Rust 后端（chameleon-core）**：
- `model.rs`：新增 `UiPreferences` 结构体和 `ThemeMode` 枚举
- `config.rs`：扩展 `GlobalConfig` 添加 `ui_preferences` 字段（`#[serde(default)]` 保证向后兼容）
- `lib.rs`（Tauri 壳）：新增 `get_ui_preferences` 和 `set_ui_preferences` 命令

**前端（vanilla HTML/JS/CSS）**：
- `index.html`：topbar 添加齿轮按钮，新增设置 `<dialog>`
- `style.css`：添加浅色主题 CSS 变量、设置表单样式、移除 `backdrop-filter`
- `app.js`：设置读写逻辑、主题切换、透明度实时应用、`startDragging()` 拖动处理器

### 数据模型

```rust
pub struct UiPreferences {
    pub theme: ThemeMode,        // Dark | Light | System
    pub panel_opacity: f32,      // 0.5 - 1.0
    pub accent_color: String,    // "#1abc9c"
}

pub enum ThemeMode {
    Dark,
    Light,
    System,
}
```

`GlobalConfig` 新增字段：
```rust
pub struct GlobalConfig {
    // ... 现有字段 ...
    #[serde(default)]
    pub ui_preferences: UiPreferences,
}
```

### API 契约

**新增 Tauri 命令**：
- `get_ui_preferences() -> UiPreferences`：读取当前 UI 偏好
- `set_ui_preferences(prefs: UiPreferences) -> ()`：保存 UI 偏好

**前端 ↔ 后端通信**：
- 启动时调用 `get_ui_preferences` 读取设置
- 设置面板修改后调用 `set_ui_preferences` 保存
- 透明度/主题变化时实时更新 CSS 变量（无需重启）

### 原生 Mica 集成

**依赖**：
- `window-vibrancy = "0.8.0"`（支持 Tauri 2）
- `windows-version = "0.1"`（检测 Windows 版本）

**逻辑**：
```rust
#[cfg(target_os = "windows")]
{
    use window_vibrancy::{apply_mica, apply_acrylic};
    use windows_version::OsVersion;
    
    let version = OsVersion::current();
    let window = app.get_webview_window("main").unwrap();
    
    // Windows 11 (build 22000+) → Mica
    if version.major >= 10 && version.build >= 22000 {
        apply_mica(&window).expect("apply_mica failed");
    } else {
        // Windows 10 → Acrylic fallback
        apply_acrylic(&window, Some((33, 31, 41, 180)))
            .expect("apply_acrylic failed");
    }
}
```

**CSS 调整**：
- 移除 `.topbar` 的 `backdrop-filter` 和 `-webkit-backdrop-filter`
- 调整 `.topbar` 背景为 `rgba(33, 31, 41, var(--panel-opacity, 0.72))`
- 其他面板（`.sys-box`, `.role-card`, `.panel`）同理使用 CSS 变量控制透明度

### 拖动修复

**权限**：`capabilities/default.json` 添加 `"core:window:allow-start-dragging"`

**前端**：
```javascript
const { getCurrentWindow } = window.__TAURI__.window;

document.querySelector('.topbar').addEventListener('mousedown', (e) => {
    // 排除按钮、输入框、窗口控制按钮
    if (e.target.closest('button, input, select, .win-controls')) return;
    getCurrentWindow().startDragging();
});
```

**HTML**：移除 topbar 的 `data-tauri-drag-region` 属性

### 主题切换

**CSS 变量**：
```css
:root {
    --bg: transparent;
    --panel: rgba(39, 38, 46, var(--panel-opacity, 0.72));
    --text: #ecebf1;
    --muted: #9a98a8;
    /* ... 深色主题默认值 ... */
}

[data-theme="light"] {
    --panel: rgba(255, 255, 255, var(--panel-opacity, 0.85));
    --text: #1a1a1a;
    --muted: #666666;
    /* ... 浅色主题值 ... */
}
```

**JS 逻辑**：
- 读取 `ui_preferences.theme`
- 如果是 `System`，监听 `prefers-color-scheme` 媒体查询
- 设置 `document.documentElement.dataset.theme`

### 设置面板 UI

**布局**：
- 齿轮图标按钮在 topbar 右侧（窗口控制按钮左边）
- 点击打开 `<dialog class="dialog">` 设置面板
- 面板内容：
  - 主题切换：3 个 radio button（深色 / 浅色 / 跟随系统）
  - 透明度：`<input type="range" min="0.5" max="1.0" step="0.05">`
  - Accent 颜色：6 个颜色圆点（点击选中）
  - 关闭按钮

**交互**：
- 透明度滑块拖动时实时更新 CSS 变量
- 主题切换时立即应用
- Accent 颜色点击后立即应用
- 关闭 dialog 时自动保存（或实时保存）

### 向后兼容

- `UiPreferences` 使用 `#[serde(default)]`，旧配置文件加载时自动填充默认值
- 不修改现有字段，只新增 `ui_preferences` 字段
- 导出/导入功能自动包含 UI 偏好（随 `GlobalConfig` 一起序列化）

## Testing Decisions

### 测试原则

只测试外部行为，不测试实现细节。优先使用现有测试基础设施（`ConfigStore` 单元测试、集成测试）。

### 测试模块

**1. `UiPreferences` 序列化/反序列化（单元测试）**
- 测试默认值正确填充
- 测试旧配置（无 `ui_preferences` 字段）能正常加载
- 测试新配置（含 `ui_preferences`）能正常保存和加载
- 测试各种 `ThemeMode` 值的序列化

**2. `ConfigStore` 扩展（单元测试）**
- 测试 `get_ui_preferences` 返回正确值
- 测试 `set_ui_preferences` 正确保存
- 测试修改 UI 偏好不影响角色和系统配置

**3. 集成测试（可选）**
- 测试完整的设置读写流程
- 测试导出/导入包含 UI 偏好

**4. 前端手动测试**
- 拖动窗口（topbar 空白区域 vs 按钮区域）
- Mica/Acrylic 效果在 Win11/Win10 上的表现
- 主题切换（深色/浅色/跟随系统）
- 透明度滑块实时效果
- Accent 颜色切换
- 设置持久化（重启后检查）
- 旧配置升级（删除 `ui_preferences` 字段后重启）

### 现有测试基础设施

- `crates/core/src/config.rs` 已有 `ConfigStore` 单元测试
- `tests/integration.rs` 已有集成测试框架
- 前端无自动化测试（手动测试）

## Out of Scope

以下内容明确不在本次 spec 范围内：

1. **前端框架迁移**：不迁移到 Vue/React/Svelte，保持 vanilla HTML/JS/CSS
2. **高级设置**：字体大小、blur 强度、快捷键、语言切换、自动启动
3. **macOS/Linux 支持**：本次只针对 Windows（Mica/Acrylic），macOS Vibrancy 和 Linux 后续考虑
4. **多窗口设置**：不为角色窗口/沙箱窗口单独设置主题
5. **设置面板动画**：不添加 dialog 打开/关闭动画（保持简单）
6. **设置重置功能**：不提供"恢复默认设置"按钮（用户可手动编辑 config.json）
7. **设置分类/分组**：不做复杂的设置分类 UI（当前只有 3 个选项，平铺即可）

## Further Notes

### 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| `window-vibrancy` 在某些 Windows 版本不兼容 | 添加 fallback：Mica 失败 → Acrylic → 保持当前 CSS blur |
| `startDragging()` 在某些情况下不工作 | 保留 CSS `-webkit-app-region: drag` 作为备选（但不作为主方案） |
| 设置面板 UI 复杂度超出预期 | 严格控制范围：只做 3 个控件，不做分组/搜索/重置 |
| 浅色主题下某些元素对比度不足 | 手动测试所有元素在浅色/深色主题下的可读性 |

### 工时估计

- 阶段 1（修复拖动）：1-2 小时
- 阶段 2（原生 Mica）：2-3 小时
- 阶段 3（设置面板）：3-4 小时
- **总计：6-9 小时（1-2 个工作日）**

### 依赖变更

**新增**：
- `window-vibrancy = "0.8.0"`（Windows 原生效果）
- `windows-version = "0.1"`（版本检测）

**无移除**。

### 配置结构变更

`config.json` 新增 `ui_preferences` 字段：
```json
{
  "browser_path": null,
  "data_root": "data",
  "roles": [...],
  "systems": [...],
  "ui_preferences": {
    "theme": "Dark",
    "panel_opacity": 0.72,
    "accent_color": "#1abc9c"
  }
}
```

旧配置（无 `ui_preferences`）自动填充默认值，向后兼容。

### 权限变更

`capabilities/default.json` 新增：
```json
"core:window:allow-start-dragging"
```

### CONTEXT.md 更新

新增术语：**UI 偏好 (UiPreferences)**

```
**UI 偏好 (UiPreferences)**:
用户界面设置：主题模式（深色/浅色/跟随系统）+ 面板透明度（0.5-1.0）+ Accent 颜色。持久化到 config.json 的 ui_preferences 字段，跨重启保留。
_Avoid_: 外观设置、主题配置
```
