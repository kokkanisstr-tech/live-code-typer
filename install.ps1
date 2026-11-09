[CmdletBinding()]
param(
    [ValidateSet('original', 'demo')]
    [string]$Variant = 'original',
    [string]$InstallDirectory,
    [switch]$NoLaunch,
    [switch]$NoPathUpdate
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$repository = 'kokkanisstr-tech/live-code-typer'
$variantKey = $Variant.ToLowerInvariant()

if ($variantKey -eq 'demo') {
    $assetName = 'live-code-typer-demo-windows-x64.zip'
    $executableName = 'live-code-typer-demo.exe'
} else {
    $assetName = 'live-code-typer-windows-x64.zip'
    $executableName = 'live-code-typer.exe'
}

if ([string]::IsNullOrWhiteSpace($InstallDirectory)) {
    $InstallDirectory = Join-Path $env:LOCALAPPDATA "Programs\LiveCodeTyper\$variantKey"
}
$InstallDirectory = [System.IO.Path]::GetFullPath($InstallDirectory)

$headers = @{
    Accept = 'application/vnd.github+json'
    'User-Agent' = 'LiveCodeTyper-Installer'
}
$releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$repository/releases?per_page=100" -Headers $headers
$release = $releases |
    Where-Object { $_.tag_name -like "$variantKey-v*" -and -not $_.draft -and -not $_.prerelease -and $_.published_at } |
    Sort-Object { [DateTimeOffset]::Parse($_.published_at) } -Descending |
    Select-Object -First 1

if (-not $release) {
    throw "No published $variantKey release is available yet. Check https://github.com/$repository/releases."
}

$asset = $release.assets | Where-Object { $_.name -ceq $assetName } | Select-Object -First 1
if (-not $asset -or $asset.browser_download_url -notlike "https://github.com/$repository/releases/download/*") {
    throw "Release $($release.tag_name) does not contain the expected Windows installer archive."
}

$temporaryDirectory = Join-Path $env:TEMP ('LiveCodeTyperInstall-' + [guid]::NewGuid().ToString('N'))
$archivePath = Join-Path $temporaryDirectory $assetName
New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null

try {
    Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $archivePath -Headers $headers -UseBasicParsing
    New-Item -ItemType Directory -Path $InstallDirectory -Force | Out-Null
    Expand-Archive -LiteralPath $archivePath -DestinationPath $InstallDirectory -Force

    $executablePath = Join-Path $InstallDirectory $executableName
    if (-not (Test-Path -LiteralPath $executablePath -PathType Leaf)) {
        throw "The downloaded archive did not contain $executableName."
    }

    if (-not $NoPathUpdate) {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $pathEntries = @($userPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        $alreadyAdded = $pathEntries | Where-Object {
            [string]::Equals($_.TrimEnd('\'), $InstallDirectory.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
        }
        if (-not $alreadyAdded) {
            [Environment]::SetEnvironmentVariable('Path', (@($pathEntries) + $InstallDirectory) -join ';', 'User')
        }
        if (-not (($env:Path -split ';') | Where-Object {
            [string]::Equals($_.TrimEnd('\'), $InstallDirectory.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
        })) {
            $env:Path = "$InstallDirectory;$env:Path"
        }
    }

    Write-Host "Installed LiveCode Typer ($variantKey) $($release.tag_name) to: $InstallDirectory"
    Write-Host "Run it later with: $executableName"
    if (-not $NoLaunch) {
        Start-Process -FilePath $executablePath
    }
} finally {
    Remove-Item -LiteralPath $temporaryDirectory -Recurse -Force -ErrorAction SilentlyContinue
}
