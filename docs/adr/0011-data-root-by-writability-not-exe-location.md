# 数据根落点按可写性决定，不按 exe 位置（便携优先、Program Files 回落 per-user）

修订 ADR-0003 的便携布局：数据根（`data_root`）的判据从「exe 在哪」改为「OS 担保当前用户可写」。`default_data_root()` 探可写性：`<exe 目录>/data` 可写 → 便携（贴 exe，保留 ADR-0003 的零安装、可搬运、可导出 = 拷 config.json 的体验）；exe 装在受保护位（`Program Files` 等）不可写 → 回落到 OS 担保的 per-user 本机数据目录（`dirs::data_dir()`：Win `%LOCALAPPDATA%\chameleon` 不漫游 / Lin `~/.local/share/chameleon` / Mac `~/Library/Application Support/chameleon`）。所有路径一律绝对。`build_config` 在交给 Chrome 前对 `profile_dir` 做可写性预检：不可写即硬错误并中文点明路径与原因，不再让 Chrome 弹生僻的 "cannot read and write to its data directory"。

背景：旧版 `GlobalConfig::default()` 把 `data_root` 硬编码为相对 `"data"`，`--user-data-dir` 以相对路径传给 Chrome，被其进程 CWD 解析到只读位（System32 / Program Files）而失败（已在 PR #24 修绝对化）。但「绝对化」只保证路径不再被 CWD 误解析，不保证落点可写——装在 `Program Files` 时 `<exe>/data` 虽绝对却仍只读。本 ADR 补上可写性这一层。

**Considered Options**: 始终用 per-user OS 目录（安装式）——否决，丢失 ADR-0003 便携体验，且 exe 贴 U 盘/普通文件夹时本可便携；首次运行弹窗让用户选——否决，对纯本地手工测试工具是多余摩擦；探可写 + 回落（采纳）——便携优先、不可写自动回落、零配置，两全。始终相对 exe + 信任 CWD——否决，即本次要根治的反模式。

**Consequences**: 装在 `Program Files` 不再失败（自动回落 per-user，用户无感）；便携用户不受影响（exe 可写 → 仍贴 exe）；config.json 显式指定不可写 `data_root` → 启动时清晰报错点名路径；`absolutize_paths` 仍把历史/手编遗留的相对 `data_root`/`profile_dir` 重定基到 `app_dir()`（保留便携意图，见 PR #24）；`dirs` 依赖此前已声明但从未被调用（死依赖），本 ADR 接上。
