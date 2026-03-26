use std::collections::HashSet;
use std::fs;
use std::path::Path;

use regex::Regex;

pub fn find_unused(project_path: &Path, dependencies: &[String]) -> Vec<String> {
    let mut used = HashSet::new();

    // Scan all JS/TS files in the project
    let dirs_to_scan = ["src", "lib", "app", "."];
    for dir in &dirs_to_scan {
        let scan_path = project_path.join(dir);
        if scan_path.is_dir() {
            scan_directory(&scan_path, &mut used, 3);
        }
    }

    // Also scan root-level JS/TS files
    if let Ok(entries) = fs::read_dir(project_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_js_file(&path) {
                scan_file(&path, &mut used);
            }
        }
    }

    // Find deps that aren't imported anywhere
    dependencies.iter()
        .filter(|dep| !used.contains(dep.as_str()) && !used.contains(&scoped_base(dep)))
        .cloned()
        .collect()
}

fn scoped_base(name: &str) -> String {
    // For @scope/pkg, check if any import starts with @scope/pkg
    name.to_string()
}

fn is_js_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("js" | "ts" | "jsx" | "tsx" | "mjs" | "cjs")
    )
}

fn scan_directory(dir: &Path, used: &mut HashSet<String>, depth: u32) {
    if depth == 0 { return; }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name != "node_modules" && name != ".git" && name != "dist" && name != "build" {
                scan_directory(&path, used, depth - 1);
            }
        } else if path.is_file() && is_js_file(&path) {
            scan_file(&path, used);
        }
    }
}

fn scan_file(path: &Path, used: &mut HashSet<String>) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    // Match: require('pkg'), require("pkg"), from 'pkg', from "pkg", import 'pkg', import "pkg"
    let re = Regex::new(r#"(?:require\s*\(\s*['"]|from\s+['"]|import\s+['"])([^'"./][^'"]*?)['"]"#).unwrap();

    for cap in re.captures_iter(&content) {
        let pkg = &cap[1];
        // Extract package name (handle scoped: @scope/pkg/path → @scope/pkg)
        let name = if pkg.starts_with('@') {
            let parts: Vec<&str> = pkg.splitn(3, '/').collect();
            if parts.len() >= 2 { format!("{}/{}", parts[0], parts[1]) } else { pkg.to_string() }
        } else {
            pkg.split('/').next().unwrap_or(pkg).to_string()
        };
        used.insert(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_find_unused() {
        let dir = std::env::temp_dir().join("dep-diet-test-unused");
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();

        fs::write(dir.join("package.json"), "{}").unwrap();
        fs::write(src.join("index.js"), r#"
            const express = require('express');
            import dayjs from 'dayjs';
        "#).unwrap();

        let deps = vec!["express".to_string(), "dayjs".to_string(), "moment".to_string(), "lodash".to_string()];
        let unused = find_unused(&dir, &deps);

        assert!(unused.contains(&"moment".to_string()));
        assert!(unused.contains(&"lodash".to_string()));
        assert!(!unused.contains(&"express".to_string()));
        assert!(!unused.contains(&"dayjs".to_string()));

        fs::remove_dir_all(&dir).ok();
    }
}
