# Short Install Command Design

## Goal

Let users install TechJobsNL with memorable GitHub Pages URLs.

macOS and Linux:

```bash
curl -fsSL https://imhalawa.github.io/techjobsnl/install | sh
```

Windows:

```powershell
irm https://imhalawa.github.io/techjobsnl/install.ps1 | iex
```

## Design

Add two small forwarding scripts:

- `docs/install` downloads and runs `scripts/install.sh` from the repository's `main` branch.
- `docs/install.ps1` downloads and runs `scripts/install.ps1` from the repository's `main` branch.

The existing installers remain the single source of truth for platform detection, checksum verification, installation, PATH handling, and updates.

## Terminal output

Both installers print concise numbered progress messages as work happens: platform detection, archive download, checksum verification, binary installation, PATH configuration, and completion. Errors stop at the visible failing step. Shell tracing and environment dumps are not used.

The final output identifies the platform-native configuration directory that TechJobsNL will create on first launch:

- Linux: `${XDG_CONFIG_HOME:-~/.config}/techjobsnl`
- macOS: `~/Library/Application Support/techjobsnl`
- Windows: `%APPDATA%\techjobsnl`

The installer does not create application state or launch the TUI; the application keeps ownership of first-run initialization.

## Command discovery

On macOS and Linux, keep installing to `~/.local/bin` by default. When that directory is absent from `PATH`, append one idempotent PATH entry to the active supported shell's user configuration (`~/.bashrc` for Bash or `~/.zshrc` for Zsh), then print the file changed and tell the user to open a new terminal. For an unrecognized shell or a custom install directory, print the exact PATH instruction without editing an unrelated shell file.

On Windows, retain the existing idempotent user `PATH` update and log whether it was added or already present. The current PowerShell process also receives the updated path.

Update both platform examples in `README.md` and `docs/index.html` to use the short commands.

## Errors and compatibility

The shell forwarding script uses the same HTTPS-only curl options as the current documented command. The PowerShell forwarding script retains terminating error behavior. Each announces that it is loading the canonical installer and returns a failure when downloading or running it fails. The old raw GitHub URLs continue to work.

## Verification

Extend the existing release checks to syntax-check both forwarding scripts and exercise idempotent PATH configuration without touching the developer's real shell files. Run the focused release test and confirm the documentation uses both new URLs.

## Scope

No custom domain, package-manager publication, duplicated installer logic, or automatic app launch is included.
