mod alternatives;
mod analyzer;
mod registry;
mod report;
mod unused;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "dep-diet", version, about = "Analyze JS dependency bloat and find lighter alternatives")]
struct Cli {
    /// Path to project directory (default: current dir)
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Scan for unused dependencies
    #[arg(long)]
    unused: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let pkg_path = cli.path.join("package.json");
    if !pkg_path.exists() {
        eprintln!("Error: No package.json found at {}", pkg_path.display());
        std::process::exit(1);
    }

    let content = std::fs::read_to_string(&pkg_path).unwrap_or_else(|e| {
        eprintln!("Error reading package.json: {e}");
        std::process::exit(1);
    });

    let pkg: serde_json::Value = serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("Error parsing package.json: {e}");
        std::process::exit(1);
    });

    let deps = pkg.get("dependencies")
        .and_then(|d| d.as_object())
        .map(|d| d.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    if deps.is_empty() {
        println!("No dependencies found in package.json");
        return;
    }

    if !cli.json {
        eprintln!("dep-diet: analyzing {} dependencies...", deps.len());
    }

    let packages = registry::fetch_all(&deps).await;
    let mut diet_report = analyzer::analyze(packages);

    diet_report.suggestions = alternatives::get_suggestions(
        &diet_report.packages.iter().map(|p| p.info.name.clone()).collect::<Vec<_>>()
    );

    if cli.unused {
        diet_report.unused = unused::find_unused(&cli.path, &deps);
    }

    report::print_report(&diet_report, cli.json);
}
