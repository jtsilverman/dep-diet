# dep-diet

Fast Rust CLI that analyzes your JavaScript project's dependencies, shows per-package install size, flags bloat, and suggests lighter alternatives. A diet plan for your node_modules.

## Demo

```
$ dep-diet --unused

  dep-diet report
  ───────────────────────────────────────
  Packages: 7    Total size: 9.1MB

╭─────────┬─────────┬───────┬───────┬──────┬──────────────────────────╮
│ Package │ Version │ Size  │ Files │ Deps │ Status                   │
├─────────┼─────────┼───────┼───────┼──────┼──────────────────────────┤
│ moment  │ 2.30.1  │ 4.4MB │ 539   │ 0    │ ⚠ 4248KB unpacked        │
│ axios   │ 1.13.6  │ 2.4MB │ 86    │ 3    │ ⚠ 2366KB unpacked        │
│ lodash  │ 4.17.23 │ 1.4MB │ 1051  │ 0    │ ⚠ 1378KB unpacked        │
│ express │ 5.2.1   │ 75KB  │ 10    │ 28   │ ⚠ 28 direct dependencies │
│ uuid    │ 13.0.0  │ 67KB  │ 73    │ 0    │ ✓ lean                   │
│ chalk   │ 5.6.2   │ 44KB  │ 12    │ 0    │ ✓ lean                   │
╰─────────┴─────────┴───────┴───────┴──────┴──────────────────────────╯

  Lighter alternatives:
  moment → dayjs  (4.2MB → 7KB gzip, mostly compatible API)
  axios → ky or undici  (Lighter HTTP clients, native fetch for simple cases)
  lodash → lodash-es or native JS  (Tree-shakeable, or most utils built into ES2022+)

  Possibly unused dependencies:
  - chalk
  - lodash
  - moment
```

## The Problem

JavaScript projects accumulate dependency bloat over time. `moment` ships 4.4MB for date formatting. `lodash` is 1.4MB when you use three functions. `request` is deprecated but still in dependency trees everywhere. Most developers never audit this because there's no fast, simple tool that shows the weight and suggests alternatives.

## How It Works

1. Reads `package.json` from your project directory
2. Queries the npm registry for each dependency's install size, file count, and dependency count (concurrent requests, ~1 second for 50 packages)
3. Flags packages as bloated if they exceed 500KB unpacked or have 20+ direct dependencies
4. Checks against a built-in map of 15+ common bloated packages with known lighter alternatives
5. Optionally scans your source files for `require()` and `import` statements to find unused deps

## Install

```bash
cargo install dep-diet
```

Or download a binary from [Releases](https://github.com/jtsilverman/dep-diet/releases).

## Usage

```bash
# Analyze current directory
dep-diet

# Analyze a specific project
dep-diet /path/to/project

# Include unused dependency detection
dep-diet --unused

# JSON output (for CI pipelines)
dep-diet --json
```

## Built-in Alternatives

| Package | Alternative | Why |
|---------|------------|-----|
| moment | dayjs | 4.2MB → 7KB gzip, mostly compatible |
| lodash | lodash-es / native JS | Tree-shakeable, or built into ES2022+ |
| request | undici / native fetch | Deprecated since 2020 |
| axios | ky / undici | Lighter HTTP clients |
| chalk | picocolors | 14KB → 2KB, 2x faster |
| uuid | crypto.randomUUID() | Built into Node 19+ |
| node-fetch | native fetch | Built into Node 18+ |
| commander | citty | Lighter CLI parser |
| left-pad | String.padStart() | Built into JS since ES2017 |

## Tech Stack

- **Rust** for fast execution and single-binary distribution
- **reqwest** for concurrent HTTP requests to npm registry
- **tokio** for async runtime
- **clap** for CLI argument parsing
- **tabled** for terminal table rendering
- **regex** for import/require pattern matching

## The Hard Part

Making the npm registry queries fast without hitting rate limits. The tool uses a semaphore-bounded concurrent request pool (10 simultaneous) with tokio, so analyzing 50 packages takes about 1-2 seconds. The trade-off is only counting direct dependencies (not full transitive trees), which keeps it fast while still being useful for bloat detection.

## License

MIT
