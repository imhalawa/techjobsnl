[CmdletBinding()]
param(
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repositoryRoot
try {
    $revision = git rev-parse HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Could not determine the source revision.' }
    $dirty = [bool](git status --porcelain)
    if (-not $OutputDirectory) {
        $OutputDirectory = Join-Path $repositoryRoot ('artifacts/windows/' + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss'))
    }
    $destination = [IO.Path]::GetFullPath($OutputDirectory)
    if ((Test-Path -LiteralPath $destination) -and (Get-ChildItem -LiteralPath $destination -Force | Select-Object -First 1)) {
        throw 'Publish requires an empty output directory to prevent shipping stale files.'
    }

    dotnet publish src/TechJobsNL.App/TechJobsNL.App.csproj -c Release -r win-x64 --self-contained true -p:RestoreLockedMode=true `
        -p:DebugType=None -p:DebugSymbols=false -o $destination
    if ($LASTEXITCODE -ne 0) { throw 'Windows publish failed.' }
    foreach ($symbol in Get-ChildItem -LiteralPath $destination -Filter '*.pdb' -File -Recurse) {
        Remove-Item -LiteralPath $symbol.FullName
    }
    Copy-Item -LiteralPath 'docs/WINDOWS_CANDIDATE.md' -Destination (Join-Path $destination 'README.md')

    $files = @(Get-ChildItem -LiteralPath $destination -File -Recurse | Sort-Object FullName | ForEach-Object {
        [ordered]@{
            path = [IO.Path]::GetRelativePath($destination, $_.FullName).Replace('\', '/')
            bytes = $_.Length
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    })
    [ordered]@{
        sourceRevision = $revision
        workingTreeModified = $dirty
        builtAtUtc = [DateTime]::UtcNow.ToString('o')
        runtime = 'win-x64'
        selfContained = $true
        files = $files
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destination 'manifest.json') -Encoding utf8
    Write-Output $destination
}
finally {
    Pop-Location
}
