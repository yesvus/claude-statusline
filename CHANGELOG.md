# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-09-09

### Changed

- Moved the context-window (`ctx`) indicator from row 1 to the end of row 2, after the 7-day quota segment.

## [0.1.0] - 2026-09-06

### Added

- Initial release: zero-overhead Rust statusline for Claude Code.
- Row 1: model, effort level, shortened workspace path, active Git branch, context window usage.
- Row 2: 5-hour and 7-day rate limits with countdown timers.
- Session-safe rate-limit sync to `~/.cache/claude/ratelimits.json` for external consumers (e.g. Waybar).

[Unreleased]: https://github.com/yesvus/claude-statusline/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/yesvus/claude-statusline/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/yesvus/claude-statusline/releases/tag/v0.1.0
