# GUI 采用 Tauri（Web 前端）而非 egui

用户明确选择 Tauri。前端为 Web 技术（HTML/JS），后端 Rust 承载 chromiumoxide 的全部 CDP 逻辑——CDP 接线不随 GUI 选型变化。

**Considered Options**: egui（推荐项：真单 exe、零运行时依赖、体积小，但 UI 上限低）；Tauri（采纳：UI 上限高，色块、拖拽、导入导出向导等傻瓜化界面更易实现）。

**Consequences**: Windows 依赖 WebView2 运行时（Win11/新 Win10 自带，个别精简企业镜像需兜底）；构建链为 pnpm + Rust；单 exe 体积与打包比 egui 重。