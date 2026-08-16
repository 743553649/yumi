use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

mod report;
mod suite;
mod utils;

#[derive(Parser)]
#[command(name = "yumi-test", about = "yumi daemon testing tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all tests or specific module
    Run {
        /// Test specific module only
        #[arg(short, long)]
        module: Option<String>,

        /// Generate HTML report
        #[arg(short, long)]
        report: bool,

        /// Output report path
        #[arg(short, long, default_value = "test_report/index.html")]
        output: PathBuf,
    },
    /// List available test modules
    List,
}

#[derive(Debug, Clone)]
pub enum TestStatus {
    Pass,
    Fail(String),
    Skip(String),
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Pass => write!(f, "{}", "PASS".green()),
            TestStatus::Fail(msg) => write!(f, "{}: {}", "FAIL".red(), msg),
            TestStatus::Skip(msg) => write!(f, "{}: {}", "SKIP".yellow(), msg),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestResult {
    pub module: String,
    pub name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            module,
            report,
            output,
        } => {
            let results = run_tests(module.as_deref())?;
            print_summary(&results);

            if report {
                let html = report::generate_html(&results);
                std::fs::create_dir_all(output.parent().unwrap())?;
                std::fs::write(&output, html)?;
                println!("\n{}", format!("Report saved to: {}", output.display()).cyan());
            }
        }
        Commands::List => {
            println!("{}", "Available test modules:".bold());
            println!("  - process  : Process status checks");
            println!("  - sysfs    : Sysfs node read/write");
            println!("  - fas      : FAS frame-aware scheduling");
            println!("  - clg      : CLG CPU load governor");
            println!("  - idle_dive: CPU idle dive");
            println!("  - touch    : Touch boost");
            println!("  - config   : Config hot-reload");
        }
    }

    Ok(())
}

fn run_tests(module_filter: Option<&str>) -> Result<Vec<TestResult>> {
    let modules = if let Some(m) = module_filter {
        vec![m]
    } else {
        vec!["process", "sysfs", "fas", "clg", "idle_dive", "touch", "config"]
    };

    let mut results = Vec::new();

    for module in modules {
        println!("\n{}", format!("Running {} tests...", module).bold());

        let module_results = match module {
            "process" => suite::process::run()?,
            "sysfs" => suite::sysfs::run()?,
            "fas" => suite::fas::run()?,
            "clg" => suite::clg::run()?,
            "idle_dive" => suite::idle_dive::run()?,
            "touch" => suite::touch_boost::run()?,
            "config" => suite::config::run()?,
            _ => {
                println!("{}: Unknown module '{}'", "Warning".yellow(), module);
                continue;
            }
        };

        for result in &module_results {
            let status_icon = match &result.status {
                TestStatus::Pass => "✓".green(),
                TestStatus::Fail(_) => "✗".red(),
                TestStatus::Skip(_) => "⊘".yellow(),
            };
            println!(
                "  {} {} ({})",
                status_icon,
                result.name,
                format!("{}ms", result.duration_ms).dimmed()
            );
        }

        results.extend(module_results);
    }

    Ok(results)
}

fn print_summary(results: &[TestResult]) {
    let passed = results.iter().filter(|r| matches!(r.status, TestStatus::Pass)).count();
    let failed = results.iter().filter(|r| matches!(r.status, TestStatus::Fail(_))).count();
    let skipped = results.iter().filter(|r| matches!(r.status, TestStatus::Skip(_))).count();
    let total = results.len();

    println!("\n{}", "=".repeat(50));
    println!("{}", "Test Summary".bold());
    println!("{}", "=".repeat(50));
    println!(
        "  {} passed, {} failed, {} skipped, {} total",
        passed.to_string().green(),
        failed.to_string().red(),
        skipped.to_string().yellow(),
        total
    );

    if failed > 0 {
        println!("\n{}", "Some tests failed!".red().bold());
    } else {
        println!("\n{}", "All tests passed!".green().bold());
    }
}
