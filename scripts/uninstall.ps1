param(
    [string]$InstallDir = $env:TECHJOBSNL_INSTALL_DIR,
    [string]$RemoveData = $env:TECHJOBSNL_REMOVE_DATA
)

$ErrorActionPreference = "Stop"
if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\techjobsnl\bin" }
$DataRoot = if ($env:APPDATA) { $env:APPDATA } elseif ($env:USERPROFILE) { Join-Path $env:USERPROFILE "AppData\Roaming" } else { throw "User configuration directory is unavailable" }

$Binary = Join-Path $InstallDir "techjobsnl.exe"
$DataDir = Join-Path $DataRoot "techjobsnl"
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

if (Test-Path -LiteralPath $DataDir) {
    if (-not $RemoveData) {
        $RemoveData = Read-Host "Remove configuration and job history at $DataDir? [y/N]"
    }
    if ($RemoveData -match "^(y|yes|1|true)$") {
        Remove-Item -LiteralPath $DataDir -Recurse -Force
        Write-Host "Removed configuration and job history at $DataDir"
    } else {
        Write-Host "Kept configuration and job history at $DataDir"
    }
} else {
    Write-Host "No configuration or job history found at $DataDir"
}
Write-Host "Feedback welcome: https://github.com/imhalawa/techjobsnl/issues/new?labels=feedback"
