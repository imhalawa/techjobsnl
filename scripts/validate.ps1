[CmdletBinding()]
param(
    [switch]$SkipRestore
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot
try {
    if (-not $SkipRestore) {
        dotnet restore --locked-mode
    }

    dotnet test --no-restore
    dotnet format --verify-no-changes --no-restore
    git diff --check
}
finally {
    Pop-Location
}
