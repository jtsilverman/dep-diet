use crate::analyzer::DietReport;
use tabled::{Table, Tabled, settings::Style};

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "Package")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Files")]
    files: String,
    #[tabled(rename = "Deps")]
    deps: String,
    #[tabled(rename = "Status")]
    status: String,
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1}MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.0}KB", bytes as f64 / 1_000.0)
    } else {
        format!("{}B", bytes)
    }
}

pub fn print_report(report: &DietReport, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(report).unwrap());
        return;
    }

    println!();
    println!("  dep-diet report");
    println!("  ───────────────────────────────────────");
    println!("  Packages: {}    Total size: {}", report.total_packages, format_size(report.total_size));
    println!();

    let rows: Vec<Row> = report.packages.iter().map(|p| {
        let status = if p.is_bloated {
            format!("⚠ {}", p.bloat_reason.as_deref().unwrap_or("bloated"))
        } else {
            "✓ lean".to_string()
        };

        Row {
            name: p.info.name.clone(),
            version: p.info.version.clone(),
            size: format_size(p.info.unpacked_size),
            files: p.info.file_count.to_string(),
            deps: p.info.dep_count.to_string(),
            status,
        }
    }).collect();

    let table = Table::new(rows).with(Style::rounded()).to_string();
    println!("{table}");

    if !report.suggestions.is_empty() {
        println!();
        println!("  Lighter alternatives:");
        for s in &report.suggestions {
            println!("  {} → {}  ({})", s.package, s.alternative, s.reason);
        }
    }

    if !report.unused.is_empty() {
        println!();
        println!("  Possibly unused dependencies:");
        for u in &report.unused {
            println!("  - {u}");
        }
    }

    println!();
}
