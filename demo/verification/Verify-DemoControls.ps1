param(
    [Parameter(Mandatory = $true)]
    [string]$ProbePath
)

$ErrorActionPreference = 'Stop'
$probe = (Resolve-Path -LiteralPath $ProbePath).Path
$outputLines = & $probe
$exitCode = $LASTEXITCODE
$output = $outputLines -join "`n"
if ($exitCode -ne 0) {
    throw "Demo controls E2E failed with exit code $exitCode."
}
if ($output -notmatch 'PASS: 25 pinned C# tasks') {
    throw "Demo controls E2E returned an unexpected result: $output"
}
Write-Output $output
