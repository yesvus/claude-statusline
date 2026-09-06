# claude-statusline

An ultra-fast, zero-overhead custom statusline binary for [Claude Code](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code), written in Rust.

```text
Claude 3.7 Sonnet medium · ~/src/lumen (main) · ctx 15% (30k/200k)
5h 82% (3h25m) · 7d 18% (1d0h)
```

## Highlights

- **Sub-Millisecond Execution (~0.5 ms):** Eliminates CPU spikes and terminal lag by replacing heavy shell scripts and multiple `jq`/`git`/`date` subprocesses with a native compiled binary.
- **Terminal Native Colors:** Uses standard 16-color ANSI escapes rather than hardcoded 24-bit RGB values, automatically matching your terminal's active color scheme and theme.
- **In-Process Git Resolution:** Detects the current Git branch and detached HEAD states directly in-process from `.git/HEAD` without spawning external `git` commands.
- **Smart Path Shortening:** Intelligently collapses deep directories into clean breadcrumbs (e.g. `…/repo/subproject`) while preserving home shortcuts (`~`).
- **Session-Safe Rate Limit Sync:** Synchronizes 5-hour and 7-day rate-limit budgets via file-locking (`flock`), preventing idle or stale sessions from overwriting fresh rate-limit telemetry.
- **Waybar Widget Integration:** Atomically updates `~/.cache/claude/ratelimits.json` on every render for seamless desktop bar status modules.

## Benchmarks

Measured on Linux (x86_64, 50 iterations):

| Implementation | Mean Latency | Min | Max | Subprocesses | CPU Overhead |
|---|---|---|---|---|---|
| Bash + jq script | 24.70 ms | 11.29 ms | 97.27 ms | 6-8 per render | High during streaming |
| **Rust binary** | **0.50 ms** | **0.43 ms** | **1.02 ms** | **0 (native)** | **~0%** |

**Result:** ~50x faster execution and zero subprocess forks per render tick.

## Installation

### From Source

Ensure you have Rust installed (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`):

```bash
git clone https://github.com/yesvus/claude-statusline.git
cd claude-statusline
cargo build --release
cp target/release/claude-statusline ~/.local/bin/
```

Or install directly with cargo:

```bash
cargo install --path .
```

## Configuration

Add or update the `statusLine` configuration in your `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/home/yesvus/.local/bin/claude-statusline",
    "padding": 0
  }
}
```

*(Alternatively, you can keep a shell wrapper script with `exec ~/.local/bin/claude-statusline "$@"`).*

## How It Works

1. Claude Code pipes session metrics JSON to `claude-statusline` on standard input (`stdin`).
2. `claude-statusline` extracts:
   - **Row 1:** Active model, effort level, shortened workspace path, active Git branch, and context window utilization (`used_percentage` and token ratio).
   - **Row 2:** 5-hour and 7-day rate limits with countdown timers until reset.
3. Rate limits are synchronized to `~/.cache/claude/ratelimits.json` with thread/process-safe locking so external status bars (such as Waybar) always reflect accurate budgets.

## License

MIT © [yesvus](https://github.com/yesvus)
