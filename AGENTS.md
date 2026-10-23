# Repository instructions

- Write every commit message in English and follow Conventional Commits: `<type>[optional scope]: <description>`, for example `feat: add release packaging`.
- Localize launcher console output to simplified Chinese for Chinese Windows locales and English for all other locales; default to simplified Chinese if locale detection fails. Preserve supported in-game translations. Do not write runtime log or command files: use memory IPC and console output. Show animated spinners during startup and exit ten seconds after success.
- Keep README.md and README.en.md aligned. Keep the antivirus warning at the beginning of both READMEs and repeat it in release notes.
- Target Windows x64 MSVC. Use one asInvoker launcher: standard permissions launch the game directly; administrator permissions launch through Steam. Preserve the complete five-file runtime package and Steam overlay guidance in both READMEs.
- Do not commit game binaries, generated packages, logs, credentials, or local installation paths. Builds must not require installed game assemblies.
- Validate with `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo xtask test-managed`, and `cargo xtask package` as appropriate. Do not start or inject into the game during automated build verification.
- Only inject into games launched by this tool after confirming no game is running. Reject existing instances with manual-close/reopen guidance; do not add PID selection or direct injection CLI commands.
