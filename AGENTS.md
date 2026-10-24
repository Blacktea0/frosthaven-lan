# Repository instructions

- Write every commit message in English and follow Conventional Commits: `<type>[optional scope]: <description>`, for example `feat: add release packaging`.
- Localize launcher console output to simplified Chinese for Chinese Windows locales and English for all other locales; default to simplified Chinese if locale detection fails. Preserve supported in-game translations. Do not write runtime log or command files: use memory IPC and console output. Show animated spinners during startup and exit ten seconds after success. On failure, print complete raw errors, causes, OS codes, managed stack traces, and the in-memory diagnostic history; wait for Enter for every invocation and never replace or truncate the original error with a localized summary.
- Keep README.md and README.en.md aligned. Keep the antivirus warning at the beginning of both READMEs and repeat it in release notes.
- Release CI uses Windows x64 MSVC; local builds also support GNU with GCC/windres. Respect the configured local toolchain instead of overriding it to MSVC without an installed Windows SDK. Use one asInvoker launcher: standard permissions launch the game directly; administrator permissions launch through Steam. Preserve the complete five-file runtime package and Steam overlay guidance in both READMEs.
- Do not commit game binaries, generated packages, logs, credentials, or local installation paths. Builds must not require installed game assemblies.
- Validate with `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo xtask test-managed`, and `cargo xtask package` as appropriate. Do not start or inject into the game during automated build verification.
- Only inject into games launched by this tool after confirming no game is running. Reject existing instances with manual-close/reopen guidance; do not add PID selection or direct injection CLI commands.

## Commit conventions

- Use English Conventional Commits: `<type>[optional scope]: <description>`. Use `fix` for bug fixes, `feat` for new behavior, `docs` for documentation, `test` for tests, and `chore(release)` for version bumps.
- Keep commits focused. Commit implementation and regression tests together; make the version bump and version-specific release notes a separate release commit.
- Inspect the full staged diff and exclude game assemblies, generated binaries/packages, runtime files, credentials, and machine-specific installation paths. Do not discard unrelated user changes.
- Keep contributor commit conventions and release procedures in this file, not in either README. READMEs remain aligned user/build documentation.

## Release procedure

1. Read the current version and inspect repository status, branch, remote state and unreleased changes. Follow the user's requested semantic version increment; update both `Cargo.toml` and the root package version in `Cargo.lock`.
2. Write `docs/releases/v<version>.md` in simplified Chinese and English. Every release must describe its actual changes, relevant fixes, validation and material limitations; generic installation text alone is insufficient. Distinguish real-game validation from unit tests and avoid claiming untested multiplayer scenarios.
3. Preserve the bilingual antivirus warnings at the beginning of the published notes, together with complete-ZIP extraction instructions and administrator/Steam overlay guidance. The release workflow prepends this content to the version-specific changes file. Require that file to exist before publication.
4. Close test game instances before building. Respect the local toolchain and run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo xtask test-managed`, `cargo xtask package`, and `powershell.exe -NoProfile -NonInteractive -File tools/tests/LauncherFailureTests.ps1`. Stop on failure. Automated verification must never start or inject into the game; manual game validation is separate and requires user authorization.
5. Inspect the final diff, commit all intended source/documentation changes with English Conventional Commits, and ensure the release tag points to the version-bump commit. Do not commit `dist/` or `artifacts/` outputs.
6. When publication is authorized, push commits to the intended remote branch and push the annotated `v<version>` tag. Do not overwrite existing tags or releases. The tag must match the Cargo version.
7. Let `.github/workflows/release.yml` build and publish the official Windows x64 MSVC ZIP and SHA256 file. Do not replace official CI assets with locally built GNU binaries. Wait for the entire workflow to complete; investigate and resolve failures instead of reporting an in-progress run as published.
8. Verify the GitHub Release is published with the correct tag, ZIP and SHA256 attachments, bilingual warnings and concrete change notes. Report the Release link and remaining validation limitations, and check the final working-tree status.
