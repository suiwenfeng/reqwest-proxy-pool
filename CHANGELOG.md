# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Todo]
- support caching request body

## [0.1.5] - 2026-03-18
### Changed
- Upgrade `reqwest` 0.12 → 0.13 (switches default TLS to rustls, adds HTTP/3 support)
- Upgrade `reqwest-middleware` 0.4 → 0.5 (tracks reqwest 0.13)
- Upgrade `rand` 0.9 → 0.10 (`RngExt` trait replaces `Rng` for `random_range`)
- Update MSRV to `1.85` (required by `getrandom` 0.4 via `rand` 0.10)
### Added
- CI: MSRV job to verify `rust-version` stays accurate on every PR
- CI: Build Examples job to catch public API breakage from user perspective
### Fixed
- README code block language tag for correct syntax highlighting

## [0.1.4] - 2026-03-18
### Fixed
- README installation example now uses semver-compatible version constraints
- Fix `license` field to `MIT OR Apache-2.0` to match existing LICENSE files

## [0.1.3] - 2026-03-18
### Changed
- Relax dependency version constraints to semver-compatible ranges (e.g. `"0.12"` instead of `"0.12.22"`) to reduce dependency conflicts for library users
- Fix `reqwest-middleware` constraint from unbounded `">=0.4.2"` to `"0.4"`
- Add `repository` field to Cargo.toml
### Added
- Declare MSRV via `rust-version = "1.75"`
- Add GitHub Actions CI workflow: test, clippy, and `cargo-semver-checks` on every push/PR
- Add Dependabot configuration for weekly automated dependency update PRs

## [0.1.2] - 2025-04-17
### Added
- 
### Changed
- fix bug in check_all_proxies 

## [0.1.0] - 2025-04-17
### Added
- support proxy parser and health check
- rate limiter for each proxy
- strategies for proxy selection
- example for quick start
### Changed
- 
