use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // 设置 OUT_DIR
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // 编译 eBPF 程序
    let ebpf_dir = PathBuf::from("yumi-ebpf");
    
    // 确定编译模式 (debug/release)
    let profile = if env::var("PROFILE").unwrap() == "release" {
        "release"
    } else {
        "debug"
    };

    // 本地开发环境（如 Termux）可能没有 nightly + bpf-linker，
    // 允许通过环境变量 YUMI_SKIP_EBPF=1 跳过 eBPF 编译（仅作语法检查）
    if env::var("YUMI_SKIP_EBPF").map_or(false, |v| v == "1") {
        println!("cargo:warning=yumi: eBPF compilation skipped (YUMI_SKIP_EBPF=1)");
        // 生成占位 eBPF 文件，使 include_bytes! 编译期检查通过（运行时加载会失败，仅用于本地检查）
        let debug_path = out_dir.join("ebpf_target/bpfel-unknown-none/debug/yumi-ebpf");
        let release_path = out_dir.join("ebpf_target/bpfel-unknown-none/release/yumi-ebpf");
        if let Some(parent) = debug_path.parent() { let _ = std::fs::create_dir_all(parent); }
        if let Some(parent) = release_path.parent() { let _ = std::fs::create_dir_all(parent); }
        let _ = std::fs::write(&debug_path, []);
        let _ = std::fs::write(&release_path, []);
        return;
    }
    
    // 编译 eBPF 程序
    let status = Command::new("cargo")
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .args([
            "build",
            "--target",
            "bpfel-unknown-none",
            "-Z",
            "build-std=core",
            "--profile",
            profile,
            "--manifest-path",
            ebpf_dir.join("Cargo.toml").to_str().unwrap(),
            "--target-dir",
            out_dir.join("ebpf_target").to_str().unwrap(),
        ])
        .status()
        .expect("Failed to compile eBPF program");
    
    if !status.success() {
        panic!("eBPF compilation failed");
    }
    
    // 通知 cargo 重新运行此脚本的条件
    println!("cargo:rerun-if-changed=yumi-ebpf/src/");
    println!("cargo:rerun-if-changed=yumi-ebpf/Cargo.toml");
}
