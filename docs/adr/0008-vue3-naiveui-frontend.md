# 引入 Vue 3 + Naive UI 替换 vanilla 前端

chameleon 前端从 v1 起使用 vanilla HTML/JS/CSS（`src-tauri/www/`）。随着功能累积（接力 / 快照 / 登录辅助 / 设置 / 预设编辑），单文件 `app.js` 已达 25KB，每次视觉调整牵一发动全身；历史证据（v1 → apple-design 重构 → Mica → 按钮层级 → 细节打磨）表明无框架约束下设计决策必然漂移。经过 grilling 决议：用 Vue 3 + Vite + TypeScript + Naive UI 在 `/ui/` 子目录重写前端，构建产物输出到 `src-tauri/www/`。

**Considered Options**: Vanilla HTML/JS/CSS 继续——否决：反复重写证明无框架约束下设计决策漂移无法根治；React 18 + Fluent UI v9——否决：JSX 心智模型较重，中文生态不如 Vue，且 Fluent UI v9 组件视觉并非用户要的「一致即可」，Naive UI 的默认风格已足够；Svelte 5——否决：中文生态弱，组件库选择少；Vue 3 + Naive UI——采纳：SFC 结构对应现有 `index.html` / `app.js` / `style.css` 三分结构；Naive UI 提供完整组件 + zh-CN 本地化 + 主题系统（`n-config-provider`）；中国独立开发者生态最优；「拿来就用」符合用户「前期不想太多封装」的诉求。

**Consequences**: 新增 `ui/` 目录（Vite + Vue 3 + TS + Naive UI + pnpm）；`tauri.conf.json` 需配 `beforeDevCommand` / `beforeBuildCommand` 触发 pnpm；覆盖 spec-0007 Out of Scope 的「不迁移前端框架」决定；现有 `src-tauri/www/` 在迁移完成后删除；`@tauri-apps/api` 在 Vue 组件内直接调用现有 Tauri 命令（API 契约不变）；Naive UI 内置 `zh-CN` locale，前端文案本地化成本降低。
