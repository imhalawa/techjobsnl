param(
    [string]$Version = $(if ($env:TECHJOBSNL_VERSION) { $env:TECHJOBSNL_VERSION } else { $env:JOB_WATCH_VERSION }),
    [string]$InstallDir = $(if ($env:TECHJOBSNL_INSTALL_DIR) { $env:TECHJOBSNL_INSTALL_DIR } else { $env:JOB_WATCH_INSTALL_DIR })
)

$ErrorActionPreference = "Stop"
$Repository = if ($env:TECHJOBSNL_REPOSITORY) { $env:TECHJOBSNL_REPOSITORY } elseif ($env:JOB_WATCH_REPOSITORY) { $env:JOB_WATCH_REPOSITORY } else { "imhalawa/techjobsnl" }
if (-not $Version) { $Version = "latest" }
if (-not $InstallDir) { $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\techjobsnl\bin" }

$Architecture = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
$Target = switch ($Architecture) {
    "X64" { "x86_64-pc-windows-msvc" }
    "Arm64" { "aarch64-pc-windows-msvc" }
    default { throw "Unsupported CPU architecture: $Architecture" }
}

$Asset = "techjobsnl-$Target.zip"
$DownloadBase = if ($env:TECHJOBSNL_DOWNLOAD_BASE) {
    $env:TECHJOBSNL_DOWNLOAD_BASE.TrimEnd("/")
} elseif ($env:JOB_WATCH_DOWNLOAD_BASE) {
    $env:JOB_WATCH_DOWNLOAD_BASE.TrimEnd("/")
} elseif ($Version -eq "latest") {
    "https://github.com/$Repository/releases/latest/download"
} else {
    "https://github.com/$Repository/releases/download/$Version"
}

$TemporaryDir = Join-Path ([System.IO.Path]::GetTempPath()) ("techjobsnl-install-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $TemporaryDir | Out-Null
try {
    $Archive = Join-Path $TemporaryDir $Asset
    $Checksums = Join-Path $TemporaryDir "SHA256SUMS"
    Invoke-WebRequest -UseBasicParsing "$DownloadBase/$Asset" -OutFile $Archive
    Invoke-WebRequest -UseBasicParsing "$DownloadBase/SHA256SUMS" -OutFile $Checksums

    $Pattern = "^(?<hash>[a-fA-F0-9]{64})\s+\*?" + [regex]::Escape($Asset) + "$"
    $ChecksumLine = Get-Content $Checksums | Where-Object { $_ -match $Pattern } | Select-Object -First 1
    if (-not $ChecksumLine) { throw "SHA256SUMS has no entry for $Asset" }
    $Expected = ([regex]::Match($ChecksumLine, $Pattern).Groups["hash"].Value).ToLowerInvariant()
    $Actual = (Get-FileHash -Algorithm SHA256 $Archive).Hash.ToLowerInvariant()
    if ($Actual -ne $Expected) { throw "Download checksum verification failed" }

    $Expanded = Join-Path $TemporaryDir "expanded"
    Expand-Archive -Path $Archive -DestinationPath $Expanded
    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Force (Join-Path $Expanded "techjobsnl.exe") (Join-Path $InstallDir "techjobsnl.exe")

    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $PathEntries = if ($UserPath) { $UserPath -split ";" } else { @() }
    if (-not ($PathEntries | Where-Object { $_.TrimEnd([char]'\') -ieq $InstallDir.TrimEnd([char]'\') })) {
        $UpdatedPath = if ($UserPath) { "$UserPath;$InstallDir" } else { $InstallDir }
        [Environment]::SetEnvironmentVariable("Path", $UpdatedPath, "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Host "Added $InstallDir to your user PATH. Open a new terminal to use it."
    }
    Write-Host "Installed techjobsnl to $InstallDir\techjobsnl.exe"
} finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $TemporaryDir
}
