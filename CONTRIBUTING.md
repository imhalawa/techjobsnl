# Contributing

Prefer the smallest change that preserves source completeness, local data, evidence, and existing behavior.

## Before changing code

- Read the relevant user or architecture document.
- Search for an existing adapter, helper, type, or test pattern before adding one.
- Keep unrelated cleanup out of the same change.
- Do not add speculative abstractions or dependencies.

## Development setup

You need Git, Make, and a Rust toolchain with Rust 2024 edition support.

```bash
git clone https://github.com/imhalawa/techjobsnl.git
cd techjobsnl
make check
```

Run the app from an interactive terminal:

```bash
make run
```

The app loads stored jobs on startup and scans official sources only when you press `r`.

## Working on a change

1. Create a focused branch.
2. Add or update the smallest deterministic test that proves the behavior.
3. Implement the change in the module that owns it.
4. Update user, configuration, architecture, privacy, or source evidence docs when behavior or claims change.
5. Run the full offline checks before submitting.

```bash
make check
```

`make check` verifies formatting, runs Clippy with warnings denied, checks release and Makefile behavior, and runs all deterministic offline tests.

Pull requests and pushes to `main` repeat the Rust checks and release build on Linux, macOS, and Windows. A change should not merge until all three CI jobs pass.

During development, run a focused test when it is faster:

```bash
cargo test test_name
```

Format Rust sources with:

```bash
make fmt
```

## Source adapter changes

Career sites are external contracts and can return partial results without an obvious error. A new or changed adapter must:

- use a first-party company or ATS source;
- reuse an existing generic adapter when it fits;
- establish stable job identity and trusted official URLs;
- validate pagination, declared totals, duplicates, required fields, and Netherlands location evidence;
- fetch and compare detail data when the listing alone cannot prove the contract;
- return **Incomplete** when data arrives but completeness cannot be proved;
- return an error when transport, configuration, schema, or another explicit failure prevents a safe result;
- include a deterministic offline test with representative stored input;
- include or update an ignored live test for contract drift;
- record the official endpoint, evidence, date, and caveats in `SOURCES.md`;
- update `SUPPORTED_COMPANIES.md` when support status changes.

Do not weaken validation to make a changing live source pass. An incomplete or failed scan must preserve the last trusted jobs instead of falsely closing them.

Run live tests only when the change needs current external verification. They contact third-party services, can be slow, and can fail because an external contract changed.

```bash
make test-live
```

## UI and analytics changes

- Keep direct search semantics clear: `/` matches title or company.
- Keep skill-based discovery evidence-backed: a selected skill shows jobs whose extracted facts contain that skill.
- Preserve keyboard use, mouse use, narrow terminal layouts, and basic readable contrast.
- Put blocking database, network, and analytics work outside the render loop.
- Update `tests/ui_test.rs` when controls, labels, selection, or layout behavior changes.
- Do not present observed postings as the complete Netherlands labour market.

## Pull request checklist

- The change has one clear purpose.
- `make check` passes.
- New behavior has a deterministic test.
- Live-source evidence is included when a source contract changed.
- Relevant documentation is updated.
- User-visible claims match the implemented behavior and dataset limits.
- No unrelated generated files, local databases, or credentials are included.

Contributions are accepted under the repository's [AGPL-3.0-only license](LICENSE).
