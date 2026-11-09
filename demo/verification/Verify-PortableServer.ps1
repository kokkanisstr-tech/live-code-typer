param(
    [Parameter(Mandatory = $true)]
    [string]$ServerPath
)

$ErrorActionPreference = 'Stop'
$resolvedServer = (Resolve-Path -LiteralPath $ServerPath).Path
$bundleFolder = [System.IO.Path]::GetDirectoryName($resolvedServer)
$portableFolder = Join-Path $bundleFolder 'portable-data'
$roomFile = Join-Path $portableFolder 'room.json'
$legacyRoomFolder = Join-Path ([System.IO.Path]::GetTempPath()) "LiveCodeTyperPortableLegacy_$([guid]::NewGuid().ToString('N'))"
if (-not (Test-Path -LiteralPath $portableFolder -PathType Container)) {
    throw 'Рядом с сервером нет portable-data.'
}
if (Test-Path -LiteralPath $roomFile) {
    throw 'Тестовый room.json уже существует; проверка его не перезапишет.'
}

$listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
$listener.Start()
$port = $listener.LocalEndpoint.Port
$listener.Stop()

$start = [System.Diagnostics.ProcessStartInfo]::new()
$start.FileName = $resolvedServer
$start.WorkingDirectory = $bundleFolder
$start.UseShellExecute = $false
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$start.CreateNoWindow = $true
$start.EnvironmentVariables['LIVECODE_SERVER_HOST'] = '127.0.0.1'
$start.EnvironmentVariables['LIVECODE_SERVER_PORT'] = [string]$port
$start.EnvironmentVariables['LIVECODE_ROOM_CODE'] = '862417'
$start.EnvironmentVariables['LIVECODETYPER_LEGACY_ROOM_DATA_FOLDER'] = $legacyRoomFolder
$null = $start.EnvironmentVariables.Remove('LIVECODE_ROOM_DATA_FOLDER')
$process = [System.Diagnostics.Process]::Start($start)
$http = [System.Net.Http.HttpClient]::new()

try {
    $ready = $false
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        if ($process.HasExited) {
            throw "Сервер досрочно завершился: $($process.StandardError.ReadToEnd())"
        }
        try {
            $response = $http.GetAsync("http://127.0.0.1:$port/health").GetAwaiter().GetResult()
            if ($response.IsSuccessStatusCode) {
                $ready = $true
                $response.Dispose()
                break
            }
            $response.Dispose()
        }
        catch { }
        Start-Sleep -Milliseconds 100
    }
    if (-not $ready) {
        throw 'Сервер не запустился в сквозной проверке portable-data.'
    }
    if (-not (Test-Path -LiteralPath $roomFile -PathType Leaf)) {
        throw 'Сервер не создал room.json рядом с исполняемым файлом.'
    }
    Write-Output 'PASS: сервер использует portable-data рядом с исполняемым файлом без отдельной настройки.'
}
finally {
    if ($process -and -not $process.HasExited) {
        $process.Kill($true)
        $null = $process.WaitForExit(5000)
    }
    $http.Dispose()
    if (Test-Path -LiteralPath $roomFile -PathType Leaf) {
        Remove-Item -LiteralPath $roomFile -Force
    }
}
