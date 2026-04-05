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
    // For @scope/pkg, extract the base package name (e.g., @babel/core → core)
    // This handles cases where a scoped package is imported by its unscoped name
    if let Some(rest) = name.strip_prefix('@') {
        if let Some((_scope, pkg)) = rest.split_once('/') {
            return pkg.to_string();
        }
    }
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

    fn setup_test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dep-diet-test-{}", name));
        let _ = fs::remove_dir_all(&dir);
        let src = dir.join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(dir.join("package.json"), "{}").unwrap();
        dir
    }

    // --- scoped_base tests ---

    #[test]
    fn test_scoped_base_extracts_package_name() {
        assert_eq!(scoped_base("@babel/core"), "core");
        assert_eq!(scoped_base("@emotion/styled"), "styled");
        assert_eq!(scoped_base("@vue/compiler-sfc"), "compiler-sfc");
    }

    #[test]
    fn test_scoped_base_returns_unscoped_unchanged() {
        assert_eq!(scoped_base("express"), "express");
        assert_eq!(scoped_base("lodash"), "lodash");
    }

    #[test]
    fn test_scoped_base_handles_bare_scope() {
        // Malformed: just @scope with no slash
        assert_eq!(scoped_base("@babel"), "@babel");
    }

    // --- scoped package detection in find_unused ---

    #[test]
    fn test_scoped_package_detected_via_import() {
        let dir = setup_test_dir("scoped-import");
        fs::write(dir.join("src/index.js"), r#"
            import { transform } from '@babel/core';
            import styled from '@emotion/styled';
        "#).unwrap();

        let deps = vec!["@babel/core".into(), "@emotion/styled".into(), "@types/node".into()];
        let unused = find_unused(&dir, &deps);

        assert!(!unused.contains(&"@babel/core".to_string()));
        assert!(!unused.contains(&"@emotion/styled".to_string()));
        assert!(unused.contains(&"@types/node".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_scoped_package_detected_via_require() {
        let dir = setup_test_dir("scoped-require");
        fs::write(dir.join("src/index.js"), r#"
            const core = require('@babel/core');
        "#).unwrap();

        let deps = vec!["@babel/core".into()];
        let unused = find_unused(&dir, &deps);
        assert!(!unused.contains(&"@babel/core".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_scoped_package_deep_import() {
        let dir = setup_test_dir("scoped-deep");
        fs::write(dir.join("src/index.js"), r#"
            import types from '@babel/core/lib/types';
        "#).unwrap();

        let deps = vec!["@babel/core".into()];
        let unused = find_unused(&dir, &deps);
        assert!(!unused.contains(&"@babel/core".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_scoped_base_fallback_matches_unscoped_import() {
        // If source imports "styled" and dep is "@emotion/styled",
        // scoped_base fallback should match
        let dir = setup_test_dir("scoped-fallback");
        fs::write(dir.join("src/index.js"), r#"
            import x from 'styled';
        "#).unwrap();

        let deps = vec!["@emotion/styled".into()];
        let unused = find_unused(&dir, &deps);
        assert!(!unused.contains(&"@emotion/styled".to_string()),
            "scoped_base fallback should match unscoped import 'styled' to '@emotion/styled'");

        fs::remove_dir_all(&dir).ok();
    }

    // --- import pattern tests ---

    #[test]
    fn test_find_unused_require() {
        let dir = setup_test_dir("require");
        fs::write(dir.join("src/index.js"), r#"
            const express = require('express');
        "#).unwrap();

        let deps = vec!["express".into(), "lodash".into()];
        let unused = find_unused(&dir, &deps);
        assert!(!unused.contains(&"express".to_string()));
        assert!(unused.contains(&"lodash".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_unused_es6_import() {
        let dir = setup_test_dir("es6");
        fs::write(dir.join("src/index.js"), r#"
            import dayjs from 'dayjs';
            import { ref } from 'vue';
        "#).unwrap();

        let deps = vec!["dayjs".into(), "vue".into(), "moment".into()];
        let unused = find_unused(&dir, &deps);
        assert!(!unused.contains(&"dayjs".to_string()));
        assert!(!unused.contains(&"vue".to_string()));
        assert!(unused.contains(&"moment".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_unused_dynamic_require() {
        let dir = setup_test_dir("dynamic-require");
        fs::write(dir.join("src/index.js"), r#"
            const chalk = require("chalk");
            const path = require('path');
        "#).unwrap();

        let deps = vec!["chalk".into()];
        let unused = find_unused(&dir, &deps);
        assert!(!unused.contains(&"chalk".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_unused_side_effect_import() {
        let dir = setup_test_dir("side-effect");
        fs::write(dir.join("src/index.js"), r#"
            import 'dotenv/config';
        "#).unwrap();

        let deps = vec!["dotenv".into()];
        let unused = find_unused(&dir, &deps);
        assert!(!unused.contains(&"dotenv".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_find_unused_mixed_quotes() {
        let dir = setup_test_dir("mixed-quotes");
        fs::write(dir.join("src/index.js"), r#"
            import express from "express";
            const chalk = require('chalk');
        "#).unwrap();

        let deps = vec!["express".into(), "chalk".into()];
        let unused = find_unused(&dir, &deps);
        assert!(unused.is_empty());

        fs::remove_dir_all(&dir).ok();
    }

    // --- original test ---

    #[test]
    fn test_find_unused() {
        let dir = setup_test_dir("basic-unused");
        fs::write(dir.join("src/index.js"), r#"
            const express = require('express');
            import dayjs from 'dayjs';
        "#).unwrap();

        let deps = vec!["express".into(), "dayjs".into(), "moment".into(), "lodash".into()];
        let unused = find_unused(&dir, &deps);

        assert!(unused.contains(&"moment".to_string()));
        assert!(unused.contains(&"lodash".to_string()));
        assert!(!unused.contains(&"express".to_string()));
        assert!(!unused.contains(&"dayjs".to_string()));

        fs::remove_dir_all(&dir).ok();
    }
}
