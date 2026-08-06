# 原生 Mica/Acrylic 效果 + UI 偏好设置 + JS 拖动

主窗口当前用 CSS `backdrop-filter` 模拟 Mica，在 WebView2 透明窗口中拖动失效（`data-tauri-drag-region` 命中测试不可靠）。改用 `window-vibrancy` crate 调用系统原生 Mica（Win11）/ Acrylic（Win10），移除 CSS blur；拖动改用 JS `window.startDragging()`（需 `core:window:allow-start-dragging` 权限），绕过 HTML 属性的命中测试。新增 UI 偏好（主题/透明度/Accent 色）持久化到 `config.json` 的 `ui_preferences` 字段（`#[serde(default)]` 向后兼容），前端启动时读取并应用 CSS 变量。

**Considered Options**: CSS -only 修复（降低透明度 + 保留 `backdrop-filter`）——否决，视觉效果不如系统级 Mica 且拖动问题在 WebView2 中无法可靠解决；`tauri-plugin-vibrancy`（Tauri 插件形式）——否决，`window-vibrancy` 是底层 crate 更灵活，且 Tauri 2 官方示例推荐此方式；`tauri-plugin-store` 存 UI 偏好——否决，现有 `ConfigStore` + `config.json` 已经够用，新增插件是过度工程。

**Consequences**: 新增依赖 `window-vibrancy = "0.6"` + `windows-version = "0.1"`；`GlobalConfig` 新增 `ui_preferences` 字段（旧配置自动填充默认值，导出/导入随行）；移除 `.topbar` 的 `backdrop-filter` 和 `data-tauri-drag-region`；capabilities 新增 `core:window:allow-start-dragging`；前端新增设置 dialog + 齿轮按钮 + 浅色主题 CSS 变量。
