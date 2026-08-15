param(
    [string]$InstallDir = $(if ($env:TECHJOBSNL_INSTALL_DIR) { $env:TECHJOBSNL_INSTALL_DIR } else { $env:JOB_WATCH_INSTALL_DIR })
)

$ErrorActionPreference = "Stop"
if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\techjobsnl\bin" }

$Binary = Join-Path $InstallDir "techjobsnl.exe"
if (Test-Path -LiteralPath $Binary) {
    Remove-Item -LiteralPath $Binary -Force
    Write-Host "Removed $Binary"
} else {
    Write-Host "techjobsnl is not installed at $Binary"
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath) {
    $UpdatedEntries = @($UserPath -split ";" | Where-Object {
        $_ -and $_.TrimEnd([char]'\') -ine $InstallDir.TrimEnd([char]'\')
    })
    $UpdatedPath = $UpdatedEntries -join ";"
    if ($UpdatedPath -ne $UserPath) {
        [Environment]::SetEnvironmentVariable("Path", $UpdatedPath, "User")
        Write-Host "Removed $InstallDir from your user PATH. Open a new terminal to apply it."
    }
}

Write-Host "Configuration and job history were not removed."
Write-Host "Feedback welcome: https://github.com/imhalawa/techjobsnl/issues/new?labels=feedback"
