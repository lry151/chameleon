fn main() {
    tauri_build::build();
    // 裸 cargo build 不会像 tauri build 那样把 WebView2Loader.dll 放到 exe 同目录，
    #[cfg(all(target_os = "windows", target_env = "gnu"))]
    {
        if let Err(e) = stage_webview2_loader() {
            println!("cargo:warning=无法自动放置 WebView2Loader.dll：{e}（可改用 tauri build，或手动从 webview2-com-sys crate 取该 DLL 放到 exe 同目录）");
        }
    }
}

#[cfg(target_os = "windows")]
fn stage_webview2_loader() -> std::io::Result<()> {
    use std::{env, fs, path::PathBuf};

    let arch = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        other => other,
    };

    // 在 cargo registry 源里找 webview2-com-sys 自带的 WebView2Loader.dll。
    // crate 版本可能变，故按前缀匹配。
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))
        .expect("无法定位 CARGO_HOME / HOME");
    let registry_src = cargo_home.join("registry").join("src");
    let mut found: Option<PathBuf> = None;
    if let Ok(vendors) = fs::read_dir(&registry_src) {
        for vendor in vendors.flatten() {
            if let Ok(crates) = fs::read_dir(vendor.path()) {
                for c in crates.flatten() {
                    let name = c.file_name();
                    if name.to_string_lossy().starts_with("webview2-com-sys-") {
                        let dll = c.path().join(arch).join("WebView2Loader.dll");
                        if dll.exists() {
                            found = Some(dll);
                            break;
                        }
                    }
                }
            }
            if found.is_some() {
                break;
            }
        }
    }
    let Some(dll) = found else {
        return Ok(()); // 找不到不阻断构建（用户可用 tauri build 兜底）
    };

    // OUT_DIR = target/<triple>/<profile>/build/<crate>-<hash>/out
    // 往上三级 = target/<triple>/<profile>/（exe 所在目录）
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR 未设置"));
    let profile_dir = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("OUT_DIR 层级异常");
    fs::create_dir_all(profile_dir).ok();
    let dest = profile_dir.join("WebView2Loader.dll");
    fs::copy(&dll, &dest)?;
    println!("cargo:warning=已放置 WebView2Loader.dll -> {}", dest.display());
    Ok(())
}
