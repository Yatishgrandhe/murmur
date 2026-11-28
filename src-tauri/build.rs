fn main() {
    // The settings UI is plain files embedded at compile time: rebuild when any of them changes,
    // or an install from source can ship a stale copy of the page.
    println!("cargo:rerun-if-changed=../ui");
    if !std::path::Path::new("resources/ggml-tiny.en-q5_1.bin").exists() {
        panic!("The bundled speech model is missing. Run scripts/fetch-model.sh first.");
    }
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        link_clang_runtime();
    }
    tauri_build::build()
}

/// whisper.cpp's Metal backend uses `@available(macOS ..)` checks, which compile to calls to
/// `__isPlatformVersionAtLeast` when the deployment target (12.0 for the bundle) is older than
/// the checked version. That symbol lives in clang's runtime (libclang_rt.osx.a), which rustc
/// does not link (`-nodefaultlibs`), so link it explicitly.
fn link_clang_runtime() {
    let out = std::process::Command::new("xcrun")
        .args(["clang", "--print-resource-dir"])
        .output()
        .or_else(|_| std::process::Command::new("clang").arg("--print-resource-dir").output());
    let Ok(out) = out else { return };
    let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let lib = std::path::Path::new(&dir).join("lib/darwin/libclang_rt.osx.a");
    if lib.exists() {
        println!("cargo:rustc-link-arg={}", lib.display());
    } else {
        println!("cargo:warning=libclang_rt.osx.a not found under {dir}; Metal availability checks may fail to link");
    }
}
