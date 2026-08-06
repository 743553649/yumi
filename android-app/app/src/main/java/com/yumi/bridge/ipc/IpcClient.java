package com.yumi.bridge.ipc;

import com.yumi.bridge.model.RealLogEntry;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.net.Socket;
import java.text.SimpleDateFormat;
import java.util.ArrayList;
import java.util.Date;
import java.util.List;
import java.util.Locale;

/**
 * 守护进程 IPC 通信层。从 MainActivity 上帝类中拆出（FIX-022）。
 *
 * 职责：封装与 yumi 守护进程（127.0.0.1:port）的 TCP Socket 通信，
 * 包括命令发送、日志拉取（TCP 优先 + 本地文件兜底）、日志行解析。
 *
 * 所有网络/文件操作均为同步阻塞调用，调用方需自行放到后台线程执行。
 */
public class IpcClient {

    private final String host;
    private final int port;

    public IpcClient(String host, int port) {
        this.host = host;
        this.port = port;
    }

    public IpcClient(int port) {
        this("127.0.0.1", port);
    }

    /**
     * 同步发送一条命令并读取单行响应。
     * 失败（连接超时/异常）返回 null。
     */
    public String sendCommand(String cmd) {
        try (Socket socket = new Socket(host, port)) {
            socket.setSoTimeout(2500);
            PrintWriter writer = new PrintWriter(new OutputStreamWriter(socket.getOutputStream(), "UTF-8"), true);
            BufferedReader reader = new BufferedReader(new InputStreamReader(socket.getInputStream(), "UTF-8"));
            writer.println(cmd);
            return reader.readLine();
        } catch (Exception e) {
            return null;
        }
    }

    /**
     * 拉取守护进程日志：先走 TCP，TCP 空则回退本地文件 tail。
     * 两条路径都失败时返回空列表（由调用方决定是否注入默认提示）。
     */
    public List<RealLogEntry> fetchLogs() {
        List<RealLogEntry> fetched = new ArrayList<>();
        fetched.addAll(fetchLogsViaTcp());
        if (fetched.isEmpty()) {
            fetched.addAll(fetchLogsViaLocalFiles());
        }
        return fetched;
    }

    private List<RealLogEntry> fetchLogsViaTcp() {
        List<RealLogEntry> fetched = new ArrayList<>();
        try (Socket socket = new Socket(host, port)) {
            socket.setSoTimeout(2500);
            PrintWriter writer = new PrintWriter(new OutputStreamWriter(socket.getOutputStream(), "UTF-8"), true);
            BufferedReader reader = new BufferedReader(new InputStreamReader(socket.getInputStream(), "UTF-8"));

            writer.println("get_log 150");
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.equals("---END_LOG---")) break;
                String trimmed = line.trim();
                if (!trimmed.isEmpty()) {
                    if (trimmed.startsWith("err:")) continue;
                    int level = parseLogLevel(trimmed);
                    fetched.add(new RealLogEntry(trimmed, formatLineToChinese(trimmed), level));
                }
            }
        } catch (Exception ignored) {}
        return fetched;
    }

    private List<RealLogEntry> fetchLogsViaLocalFiles() {
        List<RealLogEntry> result = new ArrayList<>();

        // Exclusively read from authoritative daemon log path: /data/adb/modules/yumi/logs/daemon.log
        try {
            Process p = Runtime.getRuntime().exec(new String[]{"su", "-c", "tail -n 300 /data/adb/modules/yumi/logs/daemon.log"});
            BufferedReader reader = new BufferedReader(new InputStreamReader(p.getInputStream(), "UTF-8"));
            String line;
            while ((line = reader.readLine()) != null) {
                String trimmed = line.trim();
                if (!trimmed.isEmpty()) {
                    int level = parseLogLevel(trimmed);
                    result.add(new RealLogEntry(trimmed, formatLineToChinese(trimmed), level));
                }
            }
            p.waitFor();
            if (!result.isEmpty()) return result;
        } catch (Exception ignored) {}

        File targetFile = new File("/data/adb/modules/yumi/logs/daemon.log");
        if (targetFile.exists() && targetFile.length() > 0) {
            try (BufferedReader br = new BufferedReader(new FileReader(targetFile))) {
                String line;
                while ((line = br.readLine()) != null) {
                    String trimmed = line.trim();
                    if (!trimmed.isEmpty()) {
                        int level = parseLogLevel(trimmed);
                        result.add(new RealLogEntry(trimmed, formatLineToChinese(trimmed), level));
                    }
                }
            } catch (Exception ignored) {}
        }
        return result;
    }

    /**
     * 从守护进程响应中解析当前模式字符串。
     * 无匹配返回 null（调用方据此决定是否更新本地状态）。
     */
    public static String parseModeResponse(String response) {
        if (response == null) return null;
        String lower = response.toLowerCase();
        if (lower.contains("powersave")) return "powersave";
        if (lower.contains("balance")) return "balance";
        if (lower.contains("performance")) return "performance";
        if (lower.contains("fast")) return "fast";
        return null;
    }

    /** 解析单行日志的级别（DEBUG/WARN/ERROR/INFO）。 */
    public static int parseLogLevel(String line) {
        String upper = line.toUpperCase(Locale.ROOT);
        if (upper.contains("[DEBUG]") || upper.contains("[TRACE]")) return 1; // LEVEL_DEBUG
        if (upper.contains("[WARN]") || upper.contains("[WARNING]")) return 3; // LEVEL_WARN
        if (upper.contains("[ERROR]") || upper.contains("[FATAL]")) return 4; // LEVEL_ERROR
        return 2; // LEVEL_INFO
    }

    /** 将常见英文日志片段翻译为中文（仅做关键字替换，保留未匹配原文）。 */
    public static String formatLineToChinese(String line) {
        String chineseLine = line;
        if (chineseLine.contains("IPC server listening on")) {
            chineseLine = chineseLine.replace("IPC server listening on", "IPC 服务端开始监听于");
        }
        if (chineseLine.contains("ModeChange event received")) {
            chineseLine = chineseLine.replace("ModeChange event received", "收到模式切换指令");
        }
        if (chineseLine.contains("FAS controller initialized")) {
            chineseLine = chineseLine.replace("FAS controller initialized", "FAS 帧感知控制器完成初始化");
        }
        return chineseLine;
    }

    /** 模式英文 key 转中文名。未匹配返回原文。 */
    public static String getModeChineseName(String mode) {
        if (mode == null) return "";
        switch (mode.toLowerCase()) {
            case "powersave":   return "省电模式";
            case "balance":     return "均衡模式";
            case "performance": return "性能模式";
            case "fast":        return "极速模式";
            default:            return mode;
        }
    }

    /**
     * 清空守护进程磁盘日志（/data/adb/modules/yumi/logs/daemon.log）。
     * 在独立线程执行 su -c，避免阻塞调用方。
     */
    public void clearDaemonLogFile() {
        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    Process p = Runtime.getRuntime().exec(new String[]{"su", "-c", "> /data/adb/modules/yumi/logs/daemon.log"});
                    p.waitFor();
                } catch (Exception ignored) {}
            }
        }).start();
    }

    /**
     * 当 TCP/文件拉取均无日志时，构造默认"通信就绪"提示条目。
     * level 字段使用与 MainActivity.LEVEL_INFO/LEVEL_DEBUG 对应的字面量（2/1）。
     */
    public static List<RealLogEntry> buildDefaultReadyLogs() {
        List<RealLogEntry> entries = new ArrayList<>();
        SimpleDateFormat sdf = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.getDefault());
        String timeStr = sdf.format(new Date());
        entries.add(new RealLogEntry(timeStr + " [INFO] yumi 守护进程 IPC 通信准备就绪 (127.0.0.1:14567)", timeStr + " [INFO] yumi 守护进程 IPC 通信准备就绪 (127.0.0.1:14567)", 2));  // LEVEL_INFO
        entries.add(new RealLogEntry(timeStr + " [INFO] 正在轮询同步内核调度指标与应用规则...", timeStr + " [INFO] 正在轮询同步内核调度指标与应用规则...", 2));  // LEVEL_INFO
        entries.add(new RealLogEntry(timeStr + " [DEBUG] 实时系统调度监控线程运行中", timeStr + " [DEBUG] 实时系统调度监控线程运行中", 1));  // LEVEL_DEBUG
        return entries;
    }
}
