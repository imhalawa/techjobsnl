$ErrorActionPreference = "Stop"
Write-Host "[bootstrap] Loading the TechJobsNL installer"
Invoke-RestMethod https://raw.githubusercontent.com/imhalawa/techjobsnl/main/scripts/install.ps1 | Invoke-Expression
