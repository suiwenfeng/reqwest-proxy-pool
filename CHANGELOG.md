# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Todo]
- support caching request body

## [0.3.0] - 2026-03-28
### Added
- `HostConfig` for per-host pool policies (one host = one pool).
- `RetryStrategy` with `DefaultSelection` and `NewProxyOnRetry`.
- `primary` flag on `HostConfig` to define fallback pool for unknown hosts.
- `min_request_interval_ms` to configure per-proxy minimum request interval.
### Changed
- **Breaking:** top-level `ProxyPoolConfig` now focuses on shared `sources` and `hosts: Vec<HostConfig>`.
- **Breaking:** middleware now routes by request host to host-specific pools.
- Unknown request hosts now always route to the unique `primary=true` host pool.
- `min_available_proxies` is now actively checked and warns when healthy pool size drops below threshold.
- README and example updated to new host-centric mental model.
### Removed
- **Breaking:** removed single-pool `ProxyPoolConfig` fields (e.g. `health_check_url`, `retry_count`, `selection_strategy`) from top-level config.
- **Breaking:** removed `max_requests_per_second`; replaced by `min_request_interval_ms`.
- **Breaking:** removed `default_host`; fallback is now determined by `HostConfig.primary`.
### Migration
- Move per-target policy from `ProxyPoolConfig::builder()` into `HostConfig::builder("<host>")`.
- Keep proxy list source URLs in top-level `ProxyPoolConfig::builder().sources(...)`.
- Mark exactly one host as `.primary(true)`.

## [0.2.1] - 2026-03-20
### Changed
- README installation snippet now uses `reqwest-proxy-pool = "0.2"` to match APIs used in examples.
- README now consolidates `ResponseClassifier` usage into the main `Usage` section and removes duplicated classifier subsection.
- README `Usage` example aligns with `src/examples/simple.rs` (classifier + `danger_accept_invalid_certs` + updated proxy sources).
- `simple` example removes `env_logger::init()` for cleaner out-of-box execution.

## [0.2.0] - 2026-03-20
### Added
- `ResponseClassifier` trait and `ProxyResponseVerdict` enum for business-level proxy health feedback (anti-bot/captcha detection)
- `response_classifier` option in `ProxyPoolConfig` builder
- Middleware automatically retries with another proxy when classifier returns `ProxyBlocked`
- `DefaultResponseClassifier` as default (HTTP success = Success, otherwise Passthrough)
- `danger_accept_invalid_certs` option for free SOCKS5 proxies that perform TLS interception
- GitHub Actions workflow for automatic crates.io publish on PR merge to main
### Changed
- **Breaking:** `ProxyPoolConfig` fields are now crate-private; construct via `ProxyPoolConfig::builder()` and read via getters.
- Support parsing `socks5h://` proxy addresses (remote DNS resolution)
- Health check now accepts invalid TLS certificates (only tests connectivity)
- Updated example to demonstrate `ResponseClassifier` usage
### Fixed
- Health check no longer panics on invalid proxy URL (safe error handling)
- Proxy list parser no longer misidentifies non-SOCKS protocols as plain host:port
### Migration
- Replace direct struct-literal construction with `ProxyPoolConfig::builder()...build()`.

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
