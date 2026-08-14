# Troubleshooting

TechJobsNL can find jobs through its direct title/company search or through matching evidence for a selected skill or market fact. Start troubleshooting with the exact error and absolute path it prints. Configuration and source failures are designed to include the field, company, or diagnostic that failed.

## The app does not start

Verify the toolchain and compile the project:

```bash
rustc --version
cargo build --release
```

TechJobsNL uses Rust edition 2024. If the compiler rejects the edition, update the Rust toolchain.

Then validate the full project:

```bash
make check
```

## `make run` says it requires an interactive terminal

Run it in a real terminal window. `make run` deliberately rejects redirected input or output because the Ratatui interface needs a TTY [interactive terminal].

```bash
make run
```

## Configuration fails to load

Read the reported absolute `config.toml` path and field. Common causes:

- `schema_version` is not `1`;
- a country is not an uppercase two-letter code;
- a title regular expression is invalid;
- a numeric scan or analytics limit is zero;
- a source URL is not HTTPS or does not match the trusted official endpoint;
- a keybinding has more than one character, duplicates another action, or uses `j`, `k`, `J`, or `K`;
- a theme override is not a supported ANSI name or `#RRGGBB`.

Compare the affected section with [configuration documentation](CONFIGURATION.md) or the shipped [config.toml](../config.toml).

## No jobs appear

Check in this order:

1. Press `r` and wait for the scan to finish; startup does not scan automatically.
2. Open Sources and inspect health and diagnostics.
3. Clear search by pressing `/`, then `Esc`.
4. Press `f` until the footer shows the All filter.
5. Open Settings → Companies and confirm that at least one relevant company is followed.
6. Open Settings and check Job types and Hide jobs.
7. Clear included title patterns if you want every role type.

Jobs without a resolved allowed country are not accepted. Jobs without a publication date can still be active but are not considered new.

Following or unfollowing a company does not start a scan. A newly followed company has no jobs until you press `r` and its scan completes.

## A source is Failed or Incomplete

Open Sources for the latest diagnostic and Scans for the outcome history.

- **Failed** means the request, trusted endpoint, parser, configuration, or storage operation failed.
- **Incomplete** means the adapter received data but could not prove the whole result was safe to apply.

Both states preserve the company's last trusted jobs. Do not delete jobs or weaken completeness checks to hide a source-contract change. Re-run the matching ignored live test and update the adapter or fixtures only after verifying the official source.

```bash
cargo test --all-targets -- --ignored --nocapture
```

This contacts external sources and may take time or hit rate limits.

## A job will not open

Try copying the URL with `c` and opening it manually. On narrow terminals, `Enter` toggles details; use `o` to open the URL directly.

If the source removed or redirected the posting, the stored official URL can be stale until the next complete scan updates lifecycle state.

## Copy does not work

The app uses platform clipboard tools. On Linux, install a clipboard utility suitable for your session, such as `wl-copy`, `xclip`, or `xsel`. In WSL, the app can use Windows clipboard behavior.

The error reports the clipboard fallbacks it attempted. You can still press `o` to open the official URL.

## Analytics is empty, refreshing, or low confidence

- The skill-matching job list depends on Analytics and its extracted posting facts.
- Analytics requires stored eligible job descriptions.
- A fresh database needs at least one complete scan.
- `minimum_skill_occurrence` and `minimum_cooccurrence` can hide small samples.
- Comparable momentum needs complete scan history in current and previous windows.
- Low confidence is an explicit warning, not a calculation failure.

Use `x` to clear shared filters, `t` to change the window, and check Sources for healthy scan history.

## Optional AI discovery does not work

The local analytics still works. For `claude` or `codex`, confirm that the selected CLI is installed, authenticated, and callable from the same terminal.

Increase `analytics.ai_timeout_seconds` only when the provider regularly needs longer than the current limit. Invalid JSON, unknown terms, unsupported evidence, failure, or timeout is rejected and safely falls back to the local bank.

## Reset the database

Close TechJobsNL and back up the database first. Its path is relative to `config.toml` unless configured as absolute.

Deleting it is permanent: all jobs, history, applied markers, snapshots, diagnostics, analytics state, and library choices are lost. The next complete scan treats eligible jobs as new.

See [Data and privacy](DATA_AND_PRIVACY.md) before resetting.
