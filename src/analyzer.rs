use crate::registry::PackageInfo;
use serde::Serialize;

const BLOAT_SIZE_THRESHOLD: u64 = 500_000; // 500KB
const BLOAT_DEP_THRESHOLD: u32 = 20;

#[derive(Debug, Serialize)]
pub struct DietReport {
    pub total_packages: usize,
    pub total_size: u64,
    pub packages: Vec<PackageReport>,
    pub unused: Vec<String>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Serialize)]
pub struct PackageReport {
    pub info: PackageInfoSerializable,
    pub is_bloated: bool,
    pub bloat_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PackageInfoSerializable {
    pub name: String,
    pub version: String,
    pub unpacked_size: u64,
    pub file_count: u32,
    pub dep_count: u32,
}

#[derive(Debug, Serialize)]
pub struct Suggestion {
    pub package: String,
    pub alternative: String,
    pub reason: String,
}

pub fn analyze(packages: Vec<PackageInfo>) -> DietReport {
    let total_size: u64 = packages.iter().map(|p| p.unpacked_size).sum();
    let mut reports: Vec<PackageReport> = packages.iter().map(|p| {
        let mut bloat_reason = None;
        let is_bloated = if p.unpacked_size > BLOAT_SIZE_THRESHOLD {
            bloat_reason = Some(format!("{}KB unpacked", p.unpacked_size / 1024));
            true
        } else if p.dep_count > BLOAT_DEP_THRESHOLD {
            bloat_reason = Some(format!("{} direct dependencies", p.dep_count));
            true
        } else {
            false
        };

        PackageReport {
            info: PackageInfoSerializable {
                name: p.name.clone(),
                version: p.version.clone(),
                unpacked_size: p.unpacked_size,
                file_count: p.file_count,
                dep_count: p.dep_count,
            },
            is_bloated,
            bloat_reason,
        }
    }).collect();

    reports.sort_by(|a, b| b.info.unpacked_size.cmp(&a.info.unpacked_size));

    DietReport {
        total_packages: reports.len(),
        total_size,
        packages: reports,
        unused: Vec::new(),
        suggestions: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::PackageInfo;

    #[test]
    fn test_analyze_flags_bloated() {
        let packages = vec![
            PackageInfo { name: "big-pkg".into(), version: "1.0.0".into(), unpacked_size: 1_000_000, file_count: 50, dep_count: 5 },
            PackageInfo { name: "small-pkg".into(), version: "2.0.0".into(), unpacked_size: 10_000, file_count: 3, dep_count: 1 },
            PackageInfo { name: "many-deps".into(), version: "3.0.0".into(), unpacked_size: 50_000, file_count: 10, dep_count: 25 },
        ];

        let report = analyze(packages);
        assert_eq!(report.total_packages, 3);
        assert_eq!(report.total_size, 1_060_000);

        // Sorted by size desc: big-pkg, many-deps, small-pkg
        assert!(report.packages[0].is_bloated); // big-pkg (size)
        assert!(report.packages[1].is_bloated); // many-deps (dep count)
        assert!(!report.packages[2].is_bloated); // small-pkg
    }

    #[test]
    fn test_analyze_empty_packages() {
        let report = analyze(vec![]);
        assert_eq!(report.total_packages, 0);
        assert_eq!(report.total_size, 0);
        assert!(report.packages.is_empty());
        assert!(report.unused.is_empty());
        assert!(report.suggestions.is_empty());
    }

    #[test]
    fn test_analyze_all_clean() {
        let packages = vec![
            PackageInfo { name: "tiny-a".into(), version: "1.0.0".into(), unpacked_size: 1_000, file_count: 2, dep_count: 0 },
            PackageInfo { name: "tiny-b".into(), version: "1.0.0".into(), unpacked_size: 5_000, file_count: 3, dep_count: 2 },
            PackageInfo { name: "tiny-c".into(), version: "1.0.0".into(), unpacked_size: 100_000, file_count: 10, dep_count: 10 },
        ];

        let report = analyze(packages);
        assert_eq!(report.total_packages, 3);
        assert!(report.packages.iter().all(|p| !p.is_bloated));
    }

    #[test]
    fn test_analyze_boundary_size_exactly_500kb() {
        // Exactly 500KB should NOT be bloated (threshold is >500KB)
        let packages = vec![
            PackageInfo { name: "boundary".into(), version: "1.0.0".into(), unpacked_size: 500_000, file_count: 10, dep_count: 5 },
        ];
        let report = analyze(packages);
        assert!(!report.packages[0].is_bloated, "exactly 500KB should not be flagged");
    }

    #[test]
    fn test_analyze_boundary_size_just_over_500kb() {
        let packages = vec![
            PackageInfo { name: "over-boundary".into(), version: "1.0.0".into(), unpacked_size: 500_001, file_count: 10, dep_count: 5 },
        ];
        let report = analyze(packages);
        assert!(report.packages[0].is_bloated, "500,001 bytes should be flagged");
        assert!(report.packages[0].bloat_reason.as_ref().unwrap().contains("KB unpacked"));
    }

    #[test]
    fn test_analyze_boundary_deps_exactly_20() {
        // Exactly 20 deps should NOT be bloated (threshold is >20)
        let packages = vec![
            PackageInfo { name: "many-deps".into(), version: "1.0.0".into(), unpacked_size: 10_000, file_count: 5, dep_count: 20 },
        ];
        let report = analyze(packages);
        assert!(!report.packages[0].is_bloated, "exactly 20 deps should not be flagged");
    }

    #[test]
    fn test_analyze_boundary_deps_just_over_20() {
        let packages = vec![
            PackageInfo { name: "too-many-deps".into(), version: "1.0.0".into(), unpacked_size: 10_000, file_count: 5, dep_count: 21 },
        ];
        let report = analyze(packages);
        assert!(report.packages[0].is_bloated, "21 deps should be flagged");
        assert!(report.packages[0].bloat_reason.as_ref().unwrap().contains("direct dependencies"));
    }

    #[test]
    fn test_analyze_sorted_by_size_desc() {
        let packages = vec![
            PackageInfo { name: "small".into(), version: "1.0.0".into(), unpacked_size: 100, file_count: 1, dep_count: 0 },
            PackageInfo { name: "large".into(), version: "1.0.0".into(), unpacked_size: 999_999, file_count: 50, dep_count: 5 },
            PackageInfo { name: "medium".into(), version: "1.0.0".into(), unpacked_size: 50_000, file_count: 10, dep_count: 2 },
        ];
        let report = analyze(packages);
        assert_eq!(report.packages[0].info.name, "large");
        assert_eq!(report.packages[1].info.name, "medium");
        assert_eq!(report.packages[2].info.name, "small");
    }
}
