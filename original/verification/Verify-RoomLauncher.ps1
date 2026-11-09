param(
    [Parameter(Mandatory = $true)]
    [string]$LauncherPath,
    [switch]$UseDefaultPort
)

$ErrorActionPreference = 'Stop'
$resolvedLauncher = (Resolve-Path -LiteralPath $LauncherPath).Path
$bundleFolder = [System.IO.Path]::GetDirectoryName($resolvedLauncher)
$verificationRoot = Join-Path ([System.IO.Path]::GetTempPath()) "LiveCodeTyperRoomLauncher_$([guid]::NewGuid().ToString('N'))"
$dataFolder = Join-Path $verificationRoot 'server-data'
$legacyFolder = Join-Path $verificationRoot 'legacy-server-data'
$stdoutPath = Join-Path $verificationRoot 'server.stdout.log'
$stderrPath = Join-Path $verificationRoot 'server.stderr.log'
$environmentNames = @(
    'LIVECODE_SERVER_HOST',
    'LIVECODE_SERVER_PORT',
    'LIVECODE_ROOM_CODE',
    'LIVECODE_ROOM_DATA_FOLDER',
    'LIVECODETYPER_LEGACY_ROOM_DATA_FOLDER'
)
$previousEnvironment = @{}
foreach ($name in $environmentNames) {
    $previousEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}

$blocker = $null
$serverProcess = $null
$port = 0
$testDefaultPortFallback = $true
$testNormalDefaultPort = $false
$started = $false

try {
    New-Item -ItemType Directory -Path $verificationRoot -Force | Out-Null
    if ($UseDefaultPort) {
        $portProbe = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Any, 5080)
        try {
            $portProbe.Start()
            $portProbe.Stop()
            $testNormalDefaultPort = $true
        }
        catch {
            $portProbe = $null
            $env:LIVECODE_SERVER_PORT = '0'
        }
        $testDefaultPortFallback = $false
    }
    else {
        $blocker = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Any, 5080)
        try {
            $blocker.Start()
        }
        catch {
            $blocker = $null
            $testDefaultPortFallback = $false
        }
    }

    $env:LIVECODE_SERVER_HOST = '0.0.0.0'
    $env:LIVECODE_ROOM_CODE = '731204'
    $env:LIVECODE_ROOM_DATA_FOLDER = $dataFolder
    $env:LIVECODETYPER_LEGACY_ROOM_DATA_FOLDER = $legacyFolder
    if ($UseDefaultPort -and $testNormalDefaultPort) {
        Remove-Item Env:LIVECODE_SERVER_PORT -ErrorAction SilentlyContinue
    }
    elseif ($UseDefaultPort -or -not $testDefaultPortFallback) {
        $env:LIVECODE_SERVER_PORT = '0'
    }
    elseif ($testDefaultPortFallback) {
        Remove-Item Env:LIVECODE_SERVER_PORT -ErrorAction SilentlyContinue
    }

    $commandLine = '/d /c call "' + $resolvedLauncher + '"'
    $serverProcess = Start-Process -FilePath 'cmd.exe' -ArgumentList $commandLine `
        -WorkingDirectory $bundleFolder -RedirectStandardOutput $stdoutPath `
        -RedirectStandardError $stderrPath -WindowStyle Hidden -PassThru

    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while ([DateTime]::UtcNow -lt $deadline) {
        $serverProcess.Refresh()
        if ($serverProcess.HasExited) {
            $errorText = if (Test-Path -LiteralPath $stderrPath) { Get-Content -LiteralPath $stderrPath -Raw } else { '' }
            $outputText = if (Test-Path -LiteralPath $stdoutPath) { Get-Content -LiteralPath $stdoutPath -Raw } else { '' }
            throw "Запускатель завершился до запуска сервера. $outputText $errorText"
        }

        if (Test-Path -LiteralPath $stdoutPath) {
            $outputText = Get-Content -LiteralPath $stdoutPath -Raw -ErrorAction SilentlyContinue
            if ($outputText -match '(?m)^Порт:\s*(\d+)\s*$') {
                $port = [int]$Matches[1]
                try {
                    $health = Invoke-RestMethod -Uri "http://127.0.0.1:$port/health" -TimeoutSec 1
                    if ($health.status -eq 'ok') { $started = $true; break }
                }
                catch { }
            }
        }
        Start-Sleep -Milliseconds 150
    }

    if (-not $started) {
        $outputText = if (Test-Path -LiteralPath $stdoutPath) { Get-Content -LiteralPath $stdoutPath -Raw } else { '' }
        $errorText = if (Test-Path -LiteralPath $stderrPath) { Get-Content -LiteralPath $stderrPath -Raw } else { '' }
        throw "Сервер не ответил после запуска из .cmd. $outputText $errorText"
    }
    if ($testDefaultPortFallback -and $port -eq 5080) {
        throw 'Проверка занятости порта не сработала: сервер занял 5080, пока он удерживался тестом.'
    }
    if ($testNormalDefaultPort -and $port -ne 5080) {
        throw "Свободный стандартный порт 5080 не был использован. Фактически: $port."
    }

    $info = Invoke-RestMethod -Uri "http://127.0.0.1:$port/api/info" -TimeoutSec 2
    if ($null -eq $info.roomInviteBases) {
        throw 'Сервер не вернул ссылку локальной сети для приглашения Windows и Mac.'
    }
    $inviteBases = @($info.roomInviteBases)
    foreach ($base in $inviteBases) {
        $inviteUri = [System.Uri]::new([string]$base)
        if ($inviteUri.IsLoopback -or $inviteUri.Port -ne $port) {
            throw 'Ссылка для группы должна вести на сетевой адрес ведущего и правильный порт.'
        }
    }
    if (-not (Test-Path -LiteralPath (Join-Path $dataFolder 'room.json') -PathType Leaf)) {
        throw 'Сервер не создал данные в изолированной папке portable-data.'
    }

    if ($testNormalDefaultPort) {
        Write-Output 'PASS: установленный запускатель открыл сервер на стандартном порту 5080.'
    }
    elseif ($UseDefaultPort) {
        Write-Output "PASS: порт 5080 занят другой программой; сервер стартовал на свободном порту $port."
    }
    elseif ($testDefaultPortFallback) {
        Write-Output "PASS: установленный запускатель обошёл занятый порт 5080 и открыл сервер на порту $port."
    }
    else {
        Write-Output "PASS: установленный запускатель открыл сервер на порту $port; порт 5080 занят другой программой, проверен безопасный тестовый порт."
    }
    Write-Output ('PASS: серверная ссылка для локальной сети и отдельная папка данных доступны (' + $inviteBases.Count + ' сетевых ссылок).')
}
finally {
    if ($serverProcess -and -not $serverProcess.HasExited) {
        $serverProcess.Kill($true)
        $null = $serverProcess.WaitForExit(5000)
    }
    if ($blocker) {
        $blocker.Stop()
    }
    foreach ($name in $environmentNames) {
        $value = $previousEnvironment[$name]
        if ($null -eq $value) {
            Remove-Item "Env:$name" -ErrorAction SilentlyContinue
        }
        else {
            Set-Item "Env:$name" -Value $value
        }
    }
    if (Test-Path -LiteralPath $verificationRoot -PathType Container) {
        $tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
        $targetPath = [System.IO.Path]::GetFullPath($verificationRoot)
        if ($targetPath.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $targetPath -Recurse -Force
        }
    }
}
