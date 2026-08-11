# CI 质量门：clippy/fmt + cargo-deny(advisories/licenses/bans) + Windows cargo check + 集成测试串行

定稿 #32 的四项 CI 决议，落地为 `.github/workflows/test.yml` 重写 + `deny.toml` 新增。CI 从「仅 `cargo test`」升级为五 job 并行的质量门。

**扫描器选 cargo-deny（非 cargo-audit / cargo-vet）**：cargo-deny 一工具覆盖 advisories + licenses + bans 三项，正合 #32 配置范围；cargo-audit 只查 advisories，licenses/bans 仍需另配工具；cargo-vet 是 Mozilla 信任链审计流程，需维护 vetting 记录，对离线小工具团队过重。配置范围 = advisories + licenses + bans（**sources 不入**：`.cargo/config.toml` 用 rsproxy 源替换，sources 检查会把 rsproxy 当未知 registry 告警，且 #32 未列 sources）。

**cargo-deny 细节**（`deny.toml`）：advisories——vulnerability / unsound 默认即报错（不可降级，只能按 ID `ignore`）；`unmaintained = "none"`——chameleon 直接依赖 chromiumoxide 等可能被 RustSec 标「未维护」，但未维护 ≠ 可利用漏洞、平台条件依赖无法即时替换，改 review 把关、不阻断 CI；真正的 RustSec 漏洞仍 deny。licenses——`allow` 白名单覆盖当前依赖树全部许可证（MPL-2.0 = chromiumoxide、Unicode-3.0、Apache-2.0 WITH LLVM-exception 等）；唯一含 LGPL 的是 r-efi（`MIT OR Apache-2.0 OR LGPL-2.1-or-later`），由 MIT/Apache 分支满足，无需放行 copyleft；未在 allow 集合且无法由 OR 分支满足 → deny（等同拒绝 copyleft / 未知许可证）。bans——`multiple-versions = "warn"`（Rust 依赖图重复版本极常见，deny 过度阻断且常无法消除，留为信号）、`wildcards = "allow"`（workspace 无通配依赖，保留默认）。audit 节奏 = **per-push**（随质量门一道跑，给即时反馈；不另设 scheduled job——小项目低频推送，下次推送即兜住新增漏洞）。

**Windows 矩阵**：`windows-latest` 跑 `cargo check`（workspace 全成员，已定，rule #5）。**不扩展到跑 test**——Tier-1 痛点是 `#[cfg(windows)]` 代码（Mica / 注册表 / MessageBox / vibrancy）在 ubuntu 上是空 no-op、零编译覆盖；`cargo check` 在 windows 上真编译这些路径即满足（`frontendDist: "www"` 已提交，`cargo check` 无需构建前端）。test 仍只在 ubuntu 跑（已装 Chrome、单线程稳定）；扩展到 windows 翻倍 Windows runner 成本 + 翻倍 flaky-test 面，性价比不足。windows job 不跑 clippy——尊重「cargo check 已定」范围，clippy 已在 ubuntu job 全覆盖非 windows 路径。

**集成测试串行**：`cargo test -p chameleon-core -- --test-threads=1`。选 `--test-threads=1` 而非 `#[serial]`——零依赖、零代码改动、一个 CLI flag；handoff §6 gotcha：集成测试并行多 Chrome 在 WSL2 snap 上 `LaunchIo` flaky、单线程即绿。`#[serial]`（serial_test crate）要加依赖 + 逐个标注，而本 crate 集成测试本就全碰 Chrome，无「只串行部分」的精确化收益。代价（单测也串行）对小 crate 可忽略。

**Considered Options**: cargo-audit——否决，只查 advisories，licenses/bans 缺位，#32 三项配置范围无法一工具覆盖；cargo-vet——否决，需维护 vetting 记录，离线小工具团队无力承担；`#[serial]`——否决，加依赖 + 标注且无精确化收益；scheduled audit——暂缓，per-push 已给即时反馈，scheduled 为可能不走的分支预置；Windows 跑 test——否决，成本翻倍 + flaky 翻倍，cfg(windows) 编译覆盖已由 cargo check 满足；cargo-deny `unmaintained = "all"`（默认）——否决，transitive 未维护 crate 阻断 CI 且无法即时修，chromiumoxide 风险；cargo-deny sources 检查——不入，rsproxy 源替换会误报。

**Consequences**: CI 门 = fmt + clippy(`-D warnings`) + cargo-deny(advisories/licenses/bans) + ubuntu test(单线程) + windows cargo check，五 job 并行；新增依赖 = 无（cargo-deny 经 CI action 安装，不进 Cargo.toml）；`deny.toml` 为唯一新增仓库文件；clippy / fmt gate 要求代码清洁，本 PR 一并清理 master 现存 lint 与格式（field_reassign_with_default / single_match / derivable_impls / unnecessary_lazy_evaluations / io_other_error / too_many_arguments[`#[allow]`]）；feature/logging-infra rebase 到本 PR 后须同样 clippy/fmt-clean，gate 自此强制前向。升级路径：`unmaintained` 可由 none 收紧到 workspace；sources 检查可在去掉 rsproxy 源替换后启用；Windows 可由 cargo check 扩到 clippy / test。
