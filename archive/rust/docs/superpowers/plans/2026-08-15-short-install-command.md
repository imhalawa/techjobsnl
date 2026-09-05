# Short Install Command Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide short GitHub Pages install commands for Unix and Windows, with visible terminal progress and idempotent shell discovery.

**Architecture:** Two tiny GitHub Pages bootstrap scripts forward to the existing canonical installers. The canonical installers retain download, verification, installation, and PATH ownership; the TUI retains ownership of platform-native config creation on first launch.

**Tech Stack:** POSIX shell, PowerShell, GitHub Pages, existing shell release tests and GitHub Actions.

## Global Constraints

- Keep `scripts/install.sh` and `scripts/install.ps1` as the only installer implementations.
- Print progress only to the terminal; do not create log files or dump environment values.
- Preserve Linux `${XDG_CONFIG_HOME:-~/.config}/techjobsnl`, macOS `~/Library/Application Support/techjobsnl`, and Windows `%APPDATA%\techjobsnl` application data locations.
- Do not create application state or launch the TUI during installation.
- Do not add dependencies, package managers, or a custom domain.
- Do not commit anything under `docs/superpowers/`.

---

### Task 1: Unix progress and shell discovery

**Files:**
- Modify: `scripts/install.sh`
- Modify: `tests/release_test.sh`

**Interfaces:**
- Consumes: `TECHJOBSNL_INSTALL_DIR`, `HOME`, `PATH`, `SHELL`, `XDG_CONFIG_HOME`, `uname`, release archives, and `SHA256SUMS`.
- Produces: the installed `techjobsnl` binary, an idempotent Bash/Zsh PATH entry for the default install directory, and numbered terminal progress.

- [ ] **Step 1: Extend the isolated release test**

Run the installer twice with a temporary `HOME`, `SHELL=/bin/zsh`, and a `PATH` that omits the temporary default install directory. Assert that `~/.zshrc` contains exactly one literal line:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Also assert the captured output contains platform detection, download, checksum, installation, PATH, completion, and the platform-native first-run config location. Add a custom-install-directory case asserting no shell file is edited.

- [ ] **Step 2: Run the focused test and verify failure**

Run: `sh tests/release_test.sh`

Expected: FAIL because the current installer neither edits `.zshrc` nor prints the new progress messages.

- [ ] **Step 3: Add minimal Unix progress output**

In `scripts/install.sh`, print numbered messages immediately before/after the existing operations:

```text
[1/6] Detected <OS> <architecture>
[2/6] Downloading <asset>
[3/6] Verifying SHA-256 checksum
[4/6] Installing to <path>
[5/6] Configuring command discovery
[6/6] Installation complete
Config on first launch: <platform-native directory>
```

Keep existing failure exits and never enable `set -x`.

- [ ] **Step 4: Add idempotent Bash/Zsh discovery**

Only when the default `$HOME/.local/bin` is absent from `PATH`, select `~/.bashrc` for `*/bash` or `~/.zshrc` for `*/zsh`. Append this exact literal line only when `grep -Fqx` does not find it:

```sh
export PATH="$HOME/.local/bin:$PATH"
```

Log whether the entry was added or already available. For custom install directories or unknown shells, print the exact `export PATH="<install-dir>:$PATH"` instruction without modifying a shell file.

- [ ] **Step 5: Run the focused test**

Run: `sh tests/release_test.sh`

Expected: PASS, including install/update behavior, checksum verification, one PATH entry after two runs, and no edits outside the temporary home.

- [ ] **Step 6: Commit the Unix installer change**

```bash
git add scripts/install.sh tests/release_test.sh
git commit -m "feat: improve unix installer output"
```

---

### Task 2: PowerShell progress and PATH reporting

**Files:**
- Modify: `scripts/install.ps1`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `TECHJOBSNL_INSTALL_DIR`, `LOCALAPPDATA`, `APPDATA`, the Windows user `Path`, release ZIP archives, and `SHA256SUMS`.
- Produces: the installed `techjobsnl.exe`, an idempotent user PATH entry, an updated current-process PATH, and numbered terminal progress.

- [ ] **Step 1: Strengthen the existing CI PowerShell check**

Keep parsing `scripts/install.ps1` with `[scriptblock]::Create`. Add static assertions that the installer contains all six progress labels and the first-launch config message; retain the existing temporary user-PATH/uninstaller coverage.

- [ ] **Step 2: Add minimal PowerShell progress output**

Wrap the existing operations with the same six visible stages used by Unix:

```text
[1/6] Detected Windows <architecture>
[2/6] Downloading <asset>
[3/6] Verifying SHA-256 checksum
[4/6] Installing to <path>
[5/6] Configuring command discovery
[6/6] Installation complete
Config on first launch: <APPDATA>\techjobsnl
```

Keep `$ErrorActionPreference = "Stop"`. Report whether the install directory was added to the user PATH or already present, and preserve the update to `$env:Path` for the running PowerShell process.

- [ ] **Step 3: Run local checks**

Run: `sh tests/release_test.sh`

Expected: PASS. If `pwsh` is installed, also run:

```powershell
[scriptblock]::Create((Get-Content scripts/install.ps1 -Raw)) | Out-Null
```

Expected: no parse error. The Windows CI job remains the executable verification for Windows-only behavior.

- [ ] **Step 4: Commit the PowerShell installer change**

```bash
git add scripts/install.ps1 .github/workflows/ci.yml
git commit -m "feat: improve windows installer output"
```

---

### Task 3: GitHub Pages bootstrap endpoints and documentation

**Files:**
- Create: `docs/install`
- Create: `docs/install.ps1`
- Modify: `tests/release_test.sh`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `docs/index.html`

**Interfaces:**
- Consumes: the canonical raw GitHub URLs for `scripts/install.sh` and `scripts/install.ps1` on `main`.
- Produces: `https://imhalawa.github.io/techjobsnl/install` and `https://imhalawa.github.io/techjobsnl/install.ps1`.

- [ ] **Step 1: Add failing endpoint checks**

In `tests/release_test.sh`, syntax-check `docs/install` and assert it references the canonical Unix installer. In the Windows CI step, parse `docs/install.ps1` and assert it references the canonical PowerShell installer. Assert `README.md` and `docs/index.html` contain both short GitHub Pages URLs.

- [ ] **Step 2: Run the focused test and verify failure**

Run: `sh tests/release_test.sh`

Expected: FAIL because the bootstrap files and short documentation URLs do not exist.

- [ ] **Step 3: Add the POSIX bootstrap**

Create `docs/install` with `set -eu`, a single bootstrap status message, and an HTTPS-restricted download that must succeed before invoking `sh`:

```sh
#!/bin/sh
set -eu
printf '%s\n' '[bootstrap] Loading the TechJobsNL installer'
installer=$(curl --proto '=https' --tlsv1.2 -fsSL \
  https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.sh)
printf '%s\n' "$installer" | sh
```

- [ ] **Step 4: Add the PowerShell bootstrap**

Create `docs/install.ps1` with terminating errors, one bootstrap message, and a forward to the canonical script:

```powershell
$ErrorActionPreference = "Stop"
Write-Host "[bootstrap] Loading the TechJobsNL installer"
Invoke-RestMethod https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.ps1 | Invoke-Expression
```

- [ ] **Step 5: Replace installation examples**

Use these exact commands in `README.md` and `docs/index.html`:

```bash
curl -fsSL https://imhalawa.github.io/techjobsnl/install | sh
```

```powershell
irm https://imhalawa.github.io/techjobsnl/install.ps1 | iex
```

Remove the direct post-install launch from the old examples. Keep the raw installer URLs available through repository history and direct access; do not duplicate installer logic under `docs/`.

- [ ] **Step 6: Run verification**

Run:

```bash
sh tests/release_test.sh
git diff --check
```

Expected: both commands PASS. Confirm `rg -n 'imhalawa.github.io/techjobsnl/install' README.md docs/index.html` reports both platform commands in both documentation surfaces.

- [ ] **Step 7: Commit the user-facing installation change**

```bash
git add docs/install docs/install.ps1 tests/release_test.sh .github/workflows/ci.yml README.md docs/index.html
git commit -m "feat: add short install commands"
```

Do not add or commit `docs/superpowers/`.
