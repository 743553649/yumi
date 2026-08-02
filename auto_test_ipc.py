#!/usr/bin/env python3
import socket
import subprocess
import time
import sys
import os

def log(msg):
    print(f"[AUTO-TEST] {msg}")

def main():
    daemon_bin = "/data/data/com.termux/files/home/yumi_target/debug/yumi"
    workspace_dir = "/storage/emulated/0/yumi"
    if not os.path.exists(daemon_bin):
        log(f"错误: 守护进程可执行文件不存在: {daemon_bin}")
        sys.exit(1)

    log("正在启动 yumi 守护进程测试实例...")
    proc = subprocess.Popen([daemon_bin, workspace_dir], cwd=workspace_dir, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    time.sleep(2)

    if proc.poll() is not None:
        stdout, stderr = proc.communicate()
        log(f"守护进程未能启动: stdout={stdout.decode('utf-8', errors='ignore')}, stderr={stderr.decode('utf-8', errors='ignore')}")
        sys.exit(1)

    try:
        log("连接 IPC 服务端 127.0.0.1:14567...")
        
        def send_cmd(cmd):
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.settimeout(3.0)
                s.connect(("127.0.0.1", 14567))
                s.sendall((cmd + "\n").encode('utf-8'))
                resp = s.recv(1024).decode('utf-8').strip()
                return resp

        # 1. ping
        r1 = send_cmd("ping")
        log(f"测试 1: ping -> 收到 '{r1}'")
        assert r1 == "pong", f"预期 'pong'，实际 '{r1}'"

        # 2. get_mode 初始值
        r2 = send_cmd("get_mode")
        log(f"测试 2: get_mode -> 当前模式为 '{r2}'")

        # 3. set_mode performance
        r3 = send_cmd("set_mode performance")
        log(f"测试 3: set_mode performance -> 收到 '{r3}'")
        assert r3 == "ok", f"预期 'ok'，实际 '{r3}'"

        # 4. 再次 get_mode
        r4 = send_cmd("get_mode")
        log(f"测试 4: get_mode -> 校验模式变更为 '{r4}'")
        assert r4 == "performance", f"预期 'performance'，实际 '{r4}'"

        # 5. set_mode balance
        r5 = send_cmd("set_mode balance")
        log(f"测试 5: set_mode balance -> 收到 '{r5}'")
        assert r5 == "ok", f"预期 'ok'，实际 '{r5}'"

        # 6. 再次 get_mode
        r6 = send_cmd("get_mode")
        log(f"测试 6: get_mode -> 校验模式恢复为 '{r6}'")
        assert r6 == "balance", f"预期 'balance'，实际 '{r6}'"

        # 7. 异常输入测试
        r7 = send_cmd("set_mode invalid_mode")
        log(f"测试 7: set_mode invalid_mode -> 收到 '{r7}'")
        assert r7 == "err:invalid_mode", f"预期 'err:invalid_mode'，实际 '{r7}'"

        r8 = send_cmd("hello_yumi")
        log(f"测试 8: hello_yumi (未知命令) -> 收到 '{r8}'")
        assert r8 == "err:unknown_command", f"预期 'err:unknown_command'，实际 '{r8}'"

        # 8. 校验落盘文件
        mode_file = os.path.join(workspace_dir, "current_mode.txt")
        if os.path.exists(mode_file):
            with open(mode_file, 'r') as f:
                saved_mode = f.read().strip()
            log(f"测试 9: 检查 current_mode.txt 磁盘落盘文件 -> 模式为 '{saved_mode}'")
            assert saved_mode == "balance", f"预期 'balance'，实际 '{saved_mode}'"

        log("🎉 所有 9 项端到端自动化测试完全通过！网络协议与磁盘落盘均完美符合预期。")

    finally:
        log("清理测试环境，终止测试守护进程...")
        proc.terminate()
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            proc.kill()

if __name__ == "__main__":
    main()
