# Tauri 2 桌面端日志栈事实

> Research for [wayfinder map #28](https://github.com/lry151/chameleon/issues/28) / [research ticket #30](https://github.com/lry151/chameleon/issues/30)
> Date: 2026-08-11
> Scope: `tracing` + `tracing-subscriber` + `tracing-appender` structured logging on Tauri 2 desktop

---

## TL;DR 选型推荐

| 子问题 | 结论 |
|--------|------|
| (a) tauri-plugin-log 定位 | **互补但非必须**。与 `tracing` 共存需 `skip_logger()` 避免 `log::set_boxed_logger` 冲突。chameleon 场景可完全不用。 |
| (b) idiomatic 层栈 | `Registry` + `fmt::layer` (stderr, `EnvFilter` per-layer) + `fmt::layer` (rolling file, **无** EnvFilter, 常写) + `tracing-appender::non_blocking` |
| (c) panic hook | **`tracing-panic`**：最小(单函数)、直接进 tracing pipeline、自动落 rolling file。无需 `human-panic` 或自建 hook。 |

---

## (a) tauri-plugin-log 与 tracing 的关系

### 结论：互补，不冲突，但需小心配置

`tauri-plugin-log` v2.9.0 基于 `fern` + `log` facade，核心行为：
- 在 `setup()` 阶段调用 `log::set_boxed_logger(log)` 注册全局 `log` logger
- 支持 `TargetKind::LogDir` 按大小滚动文件（默认 40KB）
- 支持 `TargetKind::Webview` 将日志转发到前端（`log://log` 事件）

### 冲突点

`log::set_boxed_logger()` 全局只能调用一次。若应用先用 `tracing-log::LogTracer::init()` 注册了 `log → tracing` 桥接，再注册 `tauri-plugin-log` 的 fern logger 会返回 `SetLoggerError`。

**官方解决方案**（源码 lib.rs L597-604）：

```rust
/// For interacting with `tracing`, you can leverage the `tracing-log` logger
/// to forward logs to `tracing` or enable the `tracing` feature for this plugin
/// to emit events directly to the tracing system.
/// Both scenarios require calling this method.
pub fn skip_logger(mut self) -> Self {
    self.is_skip_logger = true;
    self
}
```

### 三种共存模式

| 模式 | 配置 | 适用场景 |
|------|------|----------|
| **纯 tracing** | 不用 `tauri-plugin-log`，纯 `tracing-subscriber` 栈 | chameleon 推荐：结构化日志、span 追踪、rolling file |
| **tauri-plugin-log 主导** | 用 `tauri-plugin-log` 的 fern logger，`tracing` 通过 `log-always` feature 降级为 `log` records | 简单应用、不需要 tracing spans |
| **混合** | `tauri-plugin-log::Builder::skip_logger()` + `tracing-log::LogTracer::init()` + `tracing-subscriber` | 需要 webview target 但不需要 fern 文件输出 |

### 已知坑

1. **无限递归**（tracing-log 文档明确警告）：
   > `log::Logger` implementations that convert log records to trace events should not be used with `Subscriber`s that convert trace events _back_ into `log` records, as doing so will result in the event recursing between the subscriber and the logger forever

   即：`tracing-log::LogTracer`（log→tracing）+ `tracing` crate 的 `log-always` feature（tracing→log）= 死循环。必须二选一。

2. **`tauri-plugin-log` 的 `tracing` feature**（Cargo.toml L47）：
   - 启用后，JS 调用 `log()` 命令会同时 emit `tracing::event!` 和 `log::logger().log()`
   - 仍需 `skip_logger()` 避免 fern logger 与 `LogTracer` 冲突

3. **文件滚动策略差异**：
   - `tauri-plugin-log`：按**大小**滚动（默认 40KB），`RotationStrategy::KeepOne/KeepAll/KeepSome`
   - `tracing-appender`：按**时间**滚动（`Rotation::DAILY/HOURLY/MINUTELY`）
   - chameleon 需求"按日滚动" → `tracing-appender::rolling::daily` 更合适

### 推荐

**chameleon 用纯 tracing 栈**，不引入 `tauri-plugin-log`。理由：
- 需要结构化日志（JSON）+ span 追踪 → tracing 原生支持
- 需要按日滚动 → `tracing-appender` 原生支持
- webview target 非必需（开发时用 DevTools 即可）

若未来需要 webview target，可加 `tauri-plugin-log` + `skip_logger()`。

---

## (b) Idiomatic tracing-subscriber 层栈

### 设计原则

1. **`run()` 最早初始化**：在 `main()` 中、`tauri::Builder::default().run()` 之前调用
2. **`RUST_LOG` 只影响 dev stderr**：`EnvFilter` 作为 **per-layer filter** 只挂到 stderr layer
3. **文件常写**：rolling file layer 无 filter，始终写入

### 最小可用示例

```rust
// src-tauri/src/main.rs
use tracing_subscriber::{prelude::*, EnvFilter, fmt, layer::SubscriberExt};
use tracing_appender::rolling;
use std::path::PathBuf;

/// 初始化日志栈，返回 WorkerGuard（必须在 main 生命周期内持有）
fn init_logging(log_dir: PathBuf) -> tracing_appender::non_blocking::WorkerGuard {
    // 1. 按日滚动文件 appender
    let file_appender = rolling::daily(&log_dir, "chameleon");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // 2. stderr layer：人类可读，受 RUST_LOG 控制
    let stderr_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .with_filter(EnvFilter::from_default_env()); // RUST_LOG

    // 3. 文件 layer：JSON 结构化，常写（无 filter）
    let file_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking);

    // 4. 组合：Registry + 两层
    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();

    guard
}

fn main() {
    // 日志目录：优先 app_log_dir，回落当前目录
    let log_dir = dirs::data_local_dir()
        .map(|p| p.join("chameleon").join("logs"))
        .unwrap_or_else(|| PathBuf::from("./logs"));
    std::fs::create_dir_all(&log_dir).ok();

    // 最早初始化（在 tauri::Builder 之前）
    let _guard = init_logging(log_dir);

    // panic hook：将 panic 写入 tracing pipeline（→ rolling file）
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

    tauri::Builder::default()
        // .plugin(...)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Cargo.toml 依赖

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
tracing-panic = "0.1"
dirs = "5"  # 跨平台目录路径
```

### 关键点

1. **`EnvFilter::from_default_env()`** 读取 `RUST_LOG`，作为 per-layer filter 只影响 stderr
2. **文件 layer 无 filter**：所有 tracing events 都写入，不受 `RUST_LOG` 影响
3. **`_guard` 必须持有**：`WorkerGuard` drop 时会 flush buffer，提前 drop 会丢日志
4. **`tracing_subscriber::registry()`** 是 span ID 的权威来源，两层都挂在它下面

### 运行时行为

| 场景 | stderr | rolling file |
|------|--------|--------------|
| `RUST_LOG` 未设置 | 静默（默认 ERROR） | 写入所有 events |
| `RUST_LOG=info` | INFO+ | 写入所有 events |
| `RUST_LOG=chameleon=debug` | chameleon crate DEBUG+ | 写入所有 events |
| release build | 无 stderr（无 console） | 写入所有 events |

---

## (c) Panic Hook 选型

### 对比

| crate | 大小 | 集成 | 写 rolling file | 评价 |
|-------|------|------|-----------------|------|
| **`tracing-panic`** 0.1.2 | 单函数 ~50 行 | 直接 emit `tracing::error!` | ✅ 自动（进 tracing pipeline） | **推荐**：最小、最直接 |
| `human-panic` 2.0.8 | ~500 行 | 写临时文件 + 打印用户友好消息 | ❌ 不进 tracing | 适合 CLI 工具，不适合 GUI |
| 自建 hook | ~20 行 | 需手动 `tracing::error!` | ✅ 需自己写 | 不必要，`tracing-panic` 已做好 |

### `tracing-panic` 源码分析

```rust
// tracing-panic 0.1.2 src/lib.rs（简化）
pub fn panic_hook(panic_info: &PanicInfo) {
    let payload = /* 提取 panic message */;
    let location = panic_info.location().map(|l| l.to_string());
    let backtrace = /* 可选，feature = "capture-backtrace" */;

    tracing::error!(
        panic.payload = payload,
        panic.location = location,
        panic.backtrace = backtrace.map(tracing::field::display),
        "A panic occurred",
    );
}
```

- 直接 emit `tracing::error!` event
- 自动进入已配置的 tracing subscriber → rolling file
- 结构化字段：`panic.payload`、`panic.location`、`panic.backtrace`
- 可选 `capture-backtrace` feature（需 `RUST_BACKTRACE=1`）

### 推荐

```rust
// Cargo.toml
tracing-panic = { version = "0.1", features = ["capture-backtrace"] }

// main.rs
std::panic::set_hook(Box::new(tracing_panic::panic_hook));
```

无需 `human-panic`（GUI 应用不需要打印到 stderr），无需自建 hook。

---

## 完整示例：chameleon 日志栈

```rust
// src-tauri/src/main.rs
use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

fn main() {
    // 1. 日志目录
    let log_dir = dirs::data_local_dir()
        .map(|p| p.join("chameleon").join("logs"))
        .unwrap_or_else(|| std::path::PathBuf::from("./logs"));
    std::fs::create_dir_all(&log_dir).ok();

    // 2. 初始化 tracing 栈
    let file_appender = rolling::daily(&log_dir, "chameleon");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_writer(std::io::stderr)
                .with_filter(EnvFilter::from_default_env()),
        )
        .with(
            fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_writer(non_blocking),
        )
        .init();

    // 3. Panic hook（在 tracing init 之后）
    std::panic::set_hook(Box::new(tracing_panic::panic_hook));

    // 4. Tauri
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
tracing-panic = { version = "0.1", features = ["capture-backtrace"] }
dirs = "5"
```

---

## 依据与官方源链接

### tauri-plugin-log
- 源码：https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/log/src/lib.rs
- `skip_logger()` 文档：L597-604
- `tracing` feature：Cargo.toml L47, commands.rs L42-56
- README：https://github.com/tauri-apps/tauri-plugin-log

### tracing 生态
- `tracing-subscriber` Layer 文档：https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/index.html
- `EnvFilter` 文档：https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html
- `tracing-appender` rolling：https://docs.rs/tracing-appender/latest/tracing_appender/rolling/index.html
- `tracing-appender` non-blocking：https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/index.html
- `tracing-log` 桥接：https://docs.rs/tracing-log/latest/tracing_log/
  - **无限递归警告**：见 "Caution: Mixing both conversions" 段落

### panic hook
- `tracing-panic`：https://docs.rs/tracing-panic/latest/tracing_panic/
- `human-panic`：https://docs.rs/human-panic/latest/human_panic/

### Tauri 官方
- Tauri v2 开发文档：https://v2.tauri.app/develop/
- Tauri 无官方日志栈推荐（文档未提及）

---

## 未验证假设

1. **`tracing-appender` 在非阻塞模式下的 flush 语义**：`WorkerGuard` drop 时 flush，但未测试高频写入时的 backpressure
2. **JSON layer 的 `with_writer(non_blocking)` 是否需要 `Arc`**：示例未包 `Arc`，需编译验证
3. **`dirs` crate vs `tauri::Manager::path().app_log_dir()`**：示例用 `dirs`，但 Tauri 提供跨平台路径 API，可考虑统一

---

## 下一步

1. 在 `src-tauri/src/main.rs` 实现上述日志栈
2. 验证 `RUST_LOG` per-layer filter 行为
3. 验证 panic hook 写入 rolling file
4. 评估是否需要 `tauri-plugin-log` 的 webview target
