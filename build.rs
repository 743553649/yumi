use std::env;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let debug_path = out_dir.join("ebpf_target/bpfel-unknown-none/debug/yumi-ebpf");
    let release_path = out_dir.join("ebpf_target/bpfel-unknown-none/release/yumi-ebpf");
    
    if let Some(parent) = debug_path.parent() { let _ = std::fs::create_dir_all(parent); }
    if let Some(parent) = release_path.parent() { let _ = std::fs::create_dir_all(parent); }

    // 检查预编译的 eBPF 产物（由 xtask 编译）
    let prebuilt_ebpf = PathBuf::from("target/bpfel-unknown-none/release/yumi-ebpf");
    if prebuilt_ebpf.exists() {
        let _ = std::fs::copy(&prebuilt_ebpf, &release_path);
        let _ = std::fs::copy(&prebuilt_ebpf, &debug_path);
        println!("cargo:warning=yumi: using prebuilt eBPF program from target/bpfel-unknown-none");
        return;
    }

    if env::var("YUMI_SKIP_EBPF").map_or(false, |v| v == "1") {
        println!("cargo:warning=yumi: eBPF compilation skipped (YUMI_SKIP_EBPF=1)");
        let _ = std::fs::write(&debug_path, []);
        let _ = std::fs::write(&release_path, []);
        return;
    }

    panic!("Error: eBPF binary not found at target/bpfel-unknown-none/release/yumi-ebpf! Ensure bpf-linker is installed and eBPF is compiled.");
}
