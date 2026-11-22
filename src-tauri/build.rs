fn main() {
    // The settings UI is plain files embedded at compile time: rebuild when any of them changes,
    // or an install from source can ship a stale copy of the page.
    println!("cargo:rerun-if-changed=../ui");
    tauri_build::build()
}
