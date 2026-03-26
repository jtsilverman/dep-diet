# Dep Diet

## Overview

A fast Rust CLI that analyzes your JavaScript project's dependencies, shows per-package install size and transitive dependency count, flags bloated packages, and suggests lighter alternatives. Like a diet plan for your node_modules. Inspired by the Video.js 88% rewrite (314pts r/webdev) and the universal dev frustration with bloated dependencies.

## Scope

- **Timebox:** 1.5 days
- **Building:**
  - Read package.json from current directory or specified path
  - Query npm registry for each dependency: install size, transitive deps count
  - Build a size report: total weight, per-package breakdown sorted by size
  - Flag bloated packages (> 500KB unpacked or > 20 transitive deps)
  - Suggest lighter alternatives for common bloated packages (built-in mapping)
  - Detect unused dependencies by scanning import/require statements in src/
  - Terminal output: colored table with sizes, warnings, suggestions
  - JSON output mode for CI integration
- **Not building:**
  - Python/pip support (JS only for MVP)
  - Bundle size analysis (install size only, not webpack tree-shaken size)
  - Auto-fix / auto-replace dependencies
  - Lock file parsing (package.json only)
  - Monorepo support
- **Ship target:** GitHub + crates.io (`cargo install dep-diet`)

## Project Type

**Pure code** (Rust CLI)

## Stack

- **Language:** Rust
- **Key crates:** serde/serde_json (JSON), reqwest (HTTP), tokio (async), clap (CLI), tabled (terminal tables)
- **Why:** Rust adds language diversity to Jake's portfolio (first Rust project). Fast execution, single binary, zero runtime deps for users. Natural fit for a CLI tool that makes many concurrent HTTP requests.

## Architecture

### Directory Structure

```
dep-diet/
  src/
    main.rs          # CLI entry, arg parsing, orchestration
    registry.rs      # npm registry API client
    analyzer.rs      # Dependency analysis + bloat detection
    alternatives.rs  # Known bloated → lighter alternative mapping
    unused.rs        # Unused dependency detection (import scanning)
    report.rs        # Terminal + JSON output formatting
  Cargo.toml
  README.md
```

### Data Types

```rust
struct PackageInfo {
    name: String,
    version: String,
    unpacked_size: u64,      // bytes
    file_count: u32,
    dep_count: u32,          // direct dependencies
    transitive_deps: u32,    // total transitive
}

struct DietReport {
    total_packages: usize,
    total_size: u64,
    packages: Vec<PackageReport>,
    unused: Vec<String>,
    suggestions: Vec<Suggestion>,
}

struct PackageReport {
    info: PackageInfo,
    is_bloated: bool,
    bloat_reason: Option<String>,
}

struct Suggestion {
    package: String,
    alternative: String,
    size_savings: String,
    reason: String,
}
```

### npm Registry API

```
GET https://registry.npmjs.org/{package}/latest
Response includes:
  - dist.unpackedSize (bytes)
  - dist.fileCount
  - dependencies (map of direct deps)

Transitive dep count: recursively count unique dependencies (with depth limit of 3 to avoid explosion)
```

### Built-in Alternatives Map

```
moment        → dayjs          "4.2MB → 7KB gzip, same API"
lodash        → lodash-es      "Tree-shakeable, or use native JS"
underscore    → native JS      "Most utils are built into ES2022+"
request       → undici/fetch   "Deprecated, use built-in fetch"
axios         → ky/undici      "Lighter HTTP client"
chalk         → picocolors     "14KB → 2KB, faster"
uuid          → crypto.randomUUID  "Built into Node 19+"
node-fetch    → native fetch   "Built into Node 18+"
commander     → clipanion      "Lighter CLI parser"
```

## Task List

### Phase 1: Project Setup

#### Task 1.1: Scaffold Rust Project
**Files:** `Cargo.toml` (create), `src/main.rs` (create)
**Do:** Init Cargo project with deps: serde, serde_json, reqwest (features: json, rustls-tls), tokio (features: full), clap (features: derive), tabled. Create main.rs with clap CLI: positional arg for path (default "."), --json flag, --unused flag. Print "dep-diet v0.1.0" and exit.
**Validate:** `source ~/.cargo/env && cargo build --release 2>&1 | tail -3 && ./target/release/dep-diet --help`

### Phase 2: Core Analysis

#### Task 2.1: npm Registry Client
**Files:** `src/registry.rs` (create), `src/main.rs` (modify)
**Do:** Create async function `fetch_package(client: &Client, name: &str) -> Result<PackageInfo>`. Hits npm registry API, parses unpackedSize, fileCount, dependency count. Create `fetch_all(names: Vec<String>) -> Vec<PackageInfo>` that fetches concurrently (max 10 at a time using semaphore). Handle 404s and timeouts gracefully.
**Validate:** `cargo build && echo '{"dependencies":{"express":"*","lodash":"*"}}' > /tmp/test-pkg.json && ./target/debug/dep-diet /tmp` (should at minimum not crash)

#### Task 2.2: Dependency Analyzer
**Files:** `src/analyzer.rs` (create)
**Do:** Create `analyze(packages: Vec<PackageInfo>) -> DietReport`. Sorts by size descending. Flags packages as bloated if unpackedSize > 500KB OR dep_count > 20. Computes total size. Returns DietReport.
**Validate:** `cargo test` (add unit test in analyzer.rs with mock PackageInfo data)

#### Task 2.3: Alternatives Map
**Files:** `src/alternatives.rs` (create)
**Do:** Create static HashMap of package name → Suggestion. Include 10+ common bloated packages with their lighter alternatives and size comparison. Function `get_suggestions(packages: &[PackageInfo]) -> Vec<Suggestion>` returns suggestions for packages that have known alternatives.
**Validate:** `cargo test` (test that "moment" returns dayjs suggestion)

### Phase 3: Unused Detection

#### Task 3.1: Import Scanner
**Files:** `src/unused.rs` (create)
**Do:** Create `find_unused(project_path: &Path, dependencies: &[String]) -> Vec<String>`. Scans all .js, .ts, .jsx, .tsx files in src/ and root. Regex matches `require('name')`, `from 'name'`, `import 'name'`. Compares against dependency list. Returns deps that are never imported. Skip devDependencies. Handle scoped packages (@org/name).
**Validate:** `cargo test` (test with mock file content and dependency list)

### Phase 4: Output

#### Task 4.1: Report Formatter
**Files:** `src/report.rs` (create), `src/main.rs` (modify)
**Do:** Create `print_report(report: &DietReport, json: bool)`. Terminal mode: colored table using tabled crate. Columns: Package, Size, Files, Deps, Status. Red for bloated, yellow for has-alternative, green for lean. Separate sections for suggestions and unused deps. JSON mode: serialize DietReport to stdout. Wire everything into main.rs: read package.json, fetch all deps, analyze, format report.
**Validate:** `cargo build && cd /tmp && echo '{"dependencies":{"express":"^4.18.0","moment":"^2.29.0","lodash":"^4.17.0","dayjs":"^1.11.0"}}' > package.json && mkdir -p src && echo 'const express = require("express"); const dayjs = require("dayjs");' > src/index.js && ~/Rock/projects/dep-diet/target/debug/dep-diet /tmp`

### Phase 5: Integration Test

#### Task 5.1: End-to-End Test
**Files:** `tests/integration.rs` (create)
**Do:** Create integration test that: 1) Creates a temp directory with a package.json containing known dependencies (express, moment, dayjs). 2) Runs dep-diet on it. 3) Verifies: moment flagged as bloated, dayjs suggestion appears, express appears in report. 4) Test --json output parses as valid JSON. 5) Test --unused with a mock src/index.js.
**Validate:** `cargo test --test integration`

### Phase 6: Ship

#### Task 6.1: README and Publish Config
**Files:** `README.md` (create), `.gitignore` (create)
**Do:** Portfolio-ready README: problem statement, demo output, install, usage, alternatives list, how it works, tech stack, the hard part, license. .gitignore for target/.
**Validate:** `cargo build --release && ./target/release/dep-diet --help`

## The One Hard Thing

**Efficiently calculating transitive dependency counts without exploding.**

Why it's hard: npm dependency trees can be enormous (express has 57 transitive deps). Naively fetching every transitive dep's metadata would mean hundreds of HTTP requests per package. The tree can also have cycles and diamonds (A depends on B and C, both depend on D).

Proposed approach: Only count direct dependencies from the registry (don't recurse). Show `dep_count` as direct deps only, which is still useful for bloat detection. This keeps the tool fast (one HTTP request per dependency in package.json).

Fallback: If users want full transitive counts, add a `--deep` flag that recursively fetches (with a depth limit of 3 and a cache to avoid re-fetching). Both approaches work independently.

## Risks

- **npm registry rate limiting (medium):** 60 req/hr unauthenticated. For a project with > 60 deps, we'd hit the limit. Mitigation: use concurrent requests within a short burst (registry allows bursts), and add a note about using an npm token for heavy use.
- **Scope (low):** JS-only, no lock file parsing, no bundle analysis. Tight and shippable.
- **First Rust project (low):** Jake hasn't shipped Rust before. The code is straightforward (HTTP + JSON + CLI), no complex ownership patterns. Interview talking point: "I chose Rust for the performance and single-binary distribution."
