# 01 — 骨架 + 角色创建 + 首个隔离窗口

**What to build:** 从零搭起 Tauri 壳（Web 前端 + Rust 后端）与核心库的分层骨架；界面上创建角色（名称 + 颜色）→ 配置落盘为明文 JSON；一键启动该角色 → 弹出真实隔离 Chrome 窗口（独立数据目录 + CDP 端口），角色卡片以色块展示；指向 Chrome/Edge 默认配置目录的角色被拒绝启动并给出中文提示。

**Blocked by:** None — can start immediately

**Status:** ready-for-agent

- [ ] 创建角色（中文名 + 颜色）后配置持久化，重启工具角色仍在
- [ ] 一键启动后弹出真实 Chrome 窗口：数据目录不在默认配置目录，且与日常浏览器互不影响
- [ ] 指向 Chrome/Edge 默认配置目录的角色被拒绝启动并给出中文提示
- [ ] 角色卡片以指定颜色色块展示，一眼可辨