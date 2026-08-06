package com.yumi.bridge.apps;

import java.io.BufferedReader;
import java.io.File;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * 应用规则 YAML 文件持久化。从 AppRulesManager 进一步拆出（FIX-022）。
 *
 * 职责：仅负责 rules.yaml 文件的读写——把 app_modes 段落解析进传入的 Map，
 * 或把 Map 中的 app_modes 写回多个候选路径的 yaml 文件。
 * 不涉及 SharedPreferences、守护进程命令或 UI（那些由 AppRulesManager 协调）。
 *
 * 候选路径覆盖 /storage/emulated/0/yumi 与 /data/adb/modules/yumi 两类部署位置，
 * 写入失败时回退到 su + heredoc 以处理受保护路径。
 */
public class RulesYamlStore {

    /**
     * 读取首个存在的 rules.yaml，解析 app_modes 段落，合并进 appModesMap。
     * 不清空传入 Map（调用方负责清空 + 预填 SharedPreferences）。
     */
    public void mergeAppModesFromYaml(Map<String, String> appModesMap) {
        File[] possibleFiles = new File[]{
                new File("/storage/emulated/0/yumi/module/rules.yaml"),
                new File("/storage/emulated/0/yumi/rules.yaml"),
                new File("/data/adb/modules/yumi/rules.yaml")
        };

        File targetFile = null;
        for (File f : possibleFiles) {
            if (f.exists()) {
                targetFile = f;
                break;
            }
        }

        if (targetFile != null) {
            try (BufferedReader br = new BufferedReader(new FileReader(targetFile))) {
                String line;
                boolean inAppModes = false;
                while ((line = br.readLine()) != null) {
                    String trimmed = line.trim();
                    if (trimmed.startsWith("app_modes:")) {
                        inAppModes = true;
                        continue;
                    }
                    if (inAppModes) {
                        if (trimmed.endsWith(":") && !trimmed.contains(" ")) {
                            inAppModes = false;
                            continue;
                        }
                        if (trimmed.contains(":")) {
                            String[] parts = trimmed.split(":", 2);
                            String pkg = parts[0].trim();
                            String mode = parts[1].trim();
                            if (!pkg.isEmpty() && !mode.isEmpty()) {
                                appModesMap.put(pkg, mode);
                            }
                        }
                    }
                }
            } catch (Exception ignored) {}
        }
    }

    /**
     * 把 appModesMap 写回所有候选 rules.yaml（含默认 SD 卡路径的按需创建）。
     * 仅负责文件写入；不触发 SharedPreferences/命令/Toast。
     */
    public void writeAppModesToYaml(Map<String, String> appModesMap) {
        File[] possibleFiles = new File[]{
                new File("/storage/emulated/0/yumi/rules.yaml"),
                new File("/storage/emulated/0/yumi/module/rules.yaml"),
                new File("/data/adb/modules/yumi/rules.yaml"),
                new File("/data/adb/modules/yumi/module/rules.yaml")
        };

        File defaultSdFile = new File("/storage/emulated/0/yumi/rules.yaml");
        try {
            if (!defaultSdFile.exists()) {
                if (defaultSdFile.getParentFile() != null) defaultSdFile.getParentFile().mkdirs();
                defaultSdFile.createNewFile();
            }
        } catch (Exception ignored) {}

        for (File targetFile : possibleFiles) {
            if (targetFile != null && (targetFile.exists() || targetFile.equals(defaultSdFile))) {
                List<String> lines = buildUpdatedYamlLines(targetFile, appModesMap);
                writeLinesToFile(targetFile, lines);
            }
        }
    }

    private List<String> buildUpdatedYamlLines(File targetFile, Map<String, String> appModesMap) {
        List<String> lines = new ArrayList<>();
        boolean hasAppModesSection = false;
        boolean inAppModesSection = false;

        if (targetFile.exists() && targetFile.length() > 0) {
            try (BufferedReader br = new BufferedReader(new FileReader(targetFile))) {
                String line;
                while ((line = br.readLine()) != null) {
                    String trimmed = line.trim();
                    if (trimmed.startsWith("app_modes:")) {
                        hasAppModesSection = true;
                        inAppModesSection = true;
                        lines.add("app_modes:");
                        for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                            lines.add("  " + entry.getKey() + ": " + entry.getValue());
                        }
                        continue;
                    }

                    if (inAppModesSection) {
                        if (line.startsWith("  ") || trimmed.isEmpty()) {
                            continue;
                        } else {
                            inAppModesSection = false;
                        }
                    }
                    lines.add(line);
                }
            } catch (Exception ignored) {}
        }

        if (!hasAppModesSection) {
            if (lines.isEmpty()) {
                lines.add("yumi_scheduler: true");
                lines.add("dynamic_enabled: true");
                lines.add("global_mode: balance");
            }
            lines.add("");
            lines.add("app_modes:");
            for (Map.Entry<String, String> entry : appModesMap.entrySet()) {
                lines.add("  " + entry.getKey() + ": " + entry.getValue());
            }
        }
        return lines;
    }

    private void writeLinesToFile(File file, List<String> lines) {
        boolean success = false;
        try (PrintWriter pw = new PrintWriter(new FileWriter(file))) {
            for (String l : lines) {
                pw.println(l);
            }
            success = true;
        } catch (Exception ignored) {}

        // Root fallback using standard stdin streaming (WebUI/Linux shell solution)
        if (!success || file.getAbsolutePath().startsWith("/data/adb") || file.getAbsolutePath().startsWith("/storage")) {
            try {
                Process p = Runtime.getRuntime().exec("su");
                try (java.io.DataOutputStream os = new java.io.DataOutputStream(p.getOutputStream())) {
                    os.writeBytes("mkdir -p " + file.getParent() + "\n");
                    os.writeBytes("cat << 'EOF' > " + file.getAbsolutePath() + "\n");
                    for (String l : lines) {
                        os.writeBytes(l + "\n");
                    }
                    os.writeBytes("EOF\n");
                    os.writeBytes("exit\n");
                    os.flush();
                }
                p.waitFor();
            } catch (Exception ignored) {}
        }
    }
}
