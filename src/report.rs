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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1_000_000), "1.0MB");
        assert_eq!(format_size(1_500_000), "1.5MB");
        assert_eq!(format_size(10_000_000), "10.0MB");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1_000), "1KB");
        assert_eq!(format_size(500_000), "500KB");
        assert_eq!(format_size(999_999), "1000KB");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0B");
        assert_eq!(format_size(1), "1B");
        assert_eq!(format_size(999), "999B");
    }

    #[test]
    fn test_format_size_boundary_kb() {
        // Exactly 1000 bytes should show as KB
        assert_eq!(format_size(1_000), "1KB");
        // 999 bytes should show as B
        assert_eq!(format_size(999), "999B");
    }

    #[test]
    fn test_format_size_boundary_mb() {
        // Exactly 1,000,000 bytes should show as MB
        assert_eq!(format_size(1_000_000), "1.0MB");
        // 999,999 bytes should show as KB
        assert_eq!(format_size(999_999), "1000KB");
    }
}
