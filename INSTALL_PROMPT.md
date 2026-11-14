# Set up claude-statusline with an AI coding agent

Copy this into your coding agent:

~~~text
Set up the lightweight native Claude Code statusline:
https://github.com/yesvus/claude-statusline

1. Read the latest README.md and INSTALL_PROMPT.md from the repo.
2. Identify macOS/Linux and architecture; Windows is not supported.
   Check whether I already have Claude Code, Rust and a statusline.
3. Choose together between prebuilt release and building from
   source. The project currently does not publish independent
   release checksum/signature files; show this risk explicitly and
   ask before installing an executable.
4. If building, follow cargo install --locked --path . from the repo.
   If using the release, inspect the exact GitHub release/tag, expected
   target archive and final install path first.
5. Preserve ~/.claude/settings.json and any existing statusLine
   configuration. Back it up and merge only the necessary property
   after my approval; never overwrite or print unrelated settings.
6. Use an absolute executable path for Claude Code settings and verify
   the binary's --version or documented smoke test as applicable.
7. Report changes, actual verification results and rollback steps.

Do not change API credentials, run unauthorized shell installers,
access production, overwrite account files or assume downloads are
cryptographically signed when they are not.
~~~

[Installation options](README.md#install-in-under-a-minute).
