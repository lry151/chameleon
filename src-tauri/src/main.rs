// 释放构建隐藏控制台窗口（Windows GUI 子系统）。
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // 缺少 WebView2 运行时给出中文安装提示而非白屏。
    #[cfg(windows)]
    if !chameleon_app_lib::webview2_installed() {
        chameleon_app_lib::show_error_box(
            "缺少 WebView2 运行时",
            "变色龙需要微软 WebView2 运行时才能运行。\n请访问 https://developer.microsoft.com/microsoft-edge/webview2/ 下载安装「常青版」运行时后重试。",
        );
        std::process::exit(1);
    }
    chameleon_app_lib::run();
}