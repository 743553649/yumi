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
    
    // 编译 eBPF 程序
    let status = Command::new("cargo")
        .args([
            "+nightly",
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
