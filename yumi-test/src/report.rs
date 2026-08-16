use crate::TestResult;
use chrono::Local;

pub fn generate_html(results: &[TestResult]) -> String {
    let passed = results.iter().filter(|r| matches!(r.status, crate::TestStatus::Pass)).count();
    let failed = results.iter().filter(|r| matches!(r.status, crate::TestStatus::Fail(_))).count();
    let skipped = results.iter().filter(|r| matches!(r.status, crate::TestStatus::Skip(_))).count();
    let total = results.len();
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");

    let mut modules = std::collections::HashMap::new();
    for result in results {
        modules.entry(result.module.clone()).or_insert_with(Vec::new).push(result);
    }

    let mut module_html = String::new();
    for (module_name, tests) in &modules {
        module_html.push_str(&format!(
            r#"
    <div class="card">
        <div class="module">{}</div>
"#,
            module_name
        ));

        for test in tests {
            let (icon, class) = match &test.status {
                crate::TestStatus::Pass => ("✓", "pass"),
                crate::TestStatus::Fail(_) => ("✗", "fail"),
                crate::TestStatus::Skip(_) => ("⊘", "skip"),
            };
            let detail = match &test.status {
                crate::TestStatus::Pass => String::new(),
                crate::TestStatus::Fail(msg) => format!(" - {}", msg),
                crate::TestStatus::Skip(msg) => format!(" - {}", msg),
            };

            module_html.push_str(&format!(
                r#"
        <div class="test-item">
            <span class="{}">{}</span>
            <span class="test-name">{}{}</span>
            <span class="test-time">{}ms</span>
        </div>
"#,
                class, icon, test.name, detail, test.duration_ms
            ));
        }

        module_html.push_str("    </div>\n");
    }

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>yumi 测试报告</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; padding: 16px; background: #f5f5f5; color: #333; }}
        h1 {{ text-align: center; margin: 16px 0; font-size: 24px; }}
        .card {{ background: white; border-radius: 12px; padding: 16px; margin: 12px 0; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
        .pass {{ color: #4caf50; font-weight: bold; }}
        .fail {{ color: #f44336; font-weight: bold; }}
        .skip {{ color: #ff9800; font-weight: bold; }}
        .summary {{ font-size: 18px; text-align: center; padding: 16px; }}
        .summary span {{ margin: 0 8px; }}
        .module {{ font-size: 18px; font-weight: 600; margin: 12px 0 8px; padding-bottom: 8px; border-bottom: 1px solid #eee; }}
        .test-item {{ padding: 10px 0; border-bottom: 1px solid #f0f0f0; display: flex; align-items: center; }}
        .test-item:last-child {{ border-bottom: none; }}
        .test-name {{ flex: 1; margin-left: 8px; }}
        .test-time {{ color: #999; font-size: 14px; }}
        .footer {{ text-align: center; color: #999; font-size: 14px; margin-top: 24px; }}
    </style>
</head>
<body>
    <h1>yumi 自动化测试报告</h1>
    <div class="card summary">
        <span class="pass">✓ 通过: {}</span>
        <span class="fail">✗ 失败: {}</span>
        <span class="skip">⊘ 跳过: {}</span>
        <span>总计: {}</span>
    </div>
{}
    <p class="footer">生成时间: {}</p>
</body>
</html>
"#,
        passed, failed, skipped, total, module_html, now
    )
}
