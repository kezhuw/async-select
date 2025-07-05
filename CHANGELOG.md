# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0] - 2025-07-05
- Fix clippy::module-one error([405676ba7164a306d4143165c8c65c03712da478](https://github.com/kezhuw/async-select/commit/405676ba7164a306d4143165c8c65c03712da478))
- Rewrite `select!` with proc_macro ([#6](https://github.com/kezhuw/async-select/pull/6))

## [0.2.1] - 2025-07-02
- Fix clippy::module-one error([405676ba7164a306d4143165c8c65c03712da478](https://github.com/kezhuw/async-select/commit/405676ba7164a306d4143165c8c65c03712da478))

## [0.2.0] - 2024-05-10
- Add biased mode to do sequential polling ([#2](https://github.com/kezhuw/async-select/pull/2) [82adfca](https://github.com/kezhuw/async-select/commit/82adfcab100f9c0191e188f741315134398c5ef9))

## [0.1.1] - 2024-05-10
### Changed
- Drop `#[cfg(doc)]` for downstream doc inline [ad54992](https://github.com/kezhuw/async-select/commit/ad5499292f4d8c7fc2f9f3874b474634708e1522)

## [0.1.0] - 2024-05-09
### Added
- `select!` to multiplex asynchronous futures simultaneously

[0.3.0]: https://github.com/kezhuw/async-select/compare/v0.2.0...v0.3.0
[0.2.1]: https://github.com/kezhuw/async-select/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kezhuw/async-select/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/kezhuw/async-select/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/kezhuw/async-select/releases/tag/v0.1.0
