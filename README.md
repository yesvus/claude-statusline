# claude-statusline

Statusline command for [Claude Code](https://docs.anthropic.com/en/docs/agents-and-tools/claude-code), written in Rust. Replaces a bash + jq script with one native binary, so a render doesn't fork `jq`/`git`/`date` subprocesses.

![claude-statusline preview](assets/statusline.png)

## What it does

- reads Claude Code's session JSON from stdin
- row 1: model, effort level, shortened workspace path, git branch (read directly from `.git/HEAD`, no `git` subprocess)
- row 2: 5h/7d rate limits with reset countdowns, plus context window usage
- writes rate limits to `~/.cache/claude/ratelimits.json` under a file lock, so something like Waybar can read them without racing a session
- uses 16-color ANSI so it follows the terminal's theme instead of hardcoded RGB

## Comparison

Measured on Linux, x86_64, 50 iterations:

| | bash + jq script | claude-statusline |
|---|---|---|
| mean latency | 24.70 ms | 0.50 ms |
| min / max | 11.29 ms / 97.27 ms | 0.43 ms / 1.02 ms |
| subprocesses per render | 6-8 | 0 |

## Install

### Prebuilt binary

```bash
os=$(uname -s); arch=$(uname -m)
case "$os" in
  Linux)  target="$([ "$arch" = aarch64 ] && echo aarch64 || echo x86_64)-unknown-linux-gnu" ;;
  Darwin) target="$([ "$arch" = arm64 ] && echo aarch64 || echo x86_64)-apple-darwin" ;;
esac
curl -fsSL "https://github.com/yesvus/claude-statusline/releases/latest/download/claude-statusline-$target.tar.gz" \
  | tar -xz -C ~/.local/bin
chmod +x ~/.local/bin/claude-statusline
```

### Via AI agent

Paste this into Claude Code (or any coding agent with shell access):

```
Install yesvus/claude-statusline: download the binary matching my OS/arch from the latest
release at https://github.com/yesvus/claude-statusline/releases/latest, extract it to
~/.local/bin/claude-statusline, make it executable, then update the statusLine block in
~/.claude/settings.json to point at it with "padding": 0.
```

### From source

Requires Rust (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`):

```bash
git clone https://github.com/yesvus/claude-statusline.git
cd claude-statusline
cargo install --path .
```

## Configuration

```json
{
  "statusLine": {
    "type": "command",
    "command": "/home/yesvus/.local/bin/claude-statusline",
    "padding": 0
  }
}
```

## License

MIT © [yesvus](https://github.com/yesvus)
