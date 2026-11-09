param(
    [Parameter(Mandatory = $true)]
    [string]$ServerPath,
    [string]$RoomTypingProbePath
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Net.Http

$resolvedServer = (Resolve-Path -LiteralPath $ServerPath).Path
$resolvedTypingProbe = if ([string]::IsNullOrWhiteSpace($RoomTypingProbePath)) {
    $null
} else {
    (Resolve-Path -LiteralPath $RoomTypingProbePath).Path
}
$verificationRoot = Join-Path ([System.IO.Path]::GetTempPath()) "LiveCodeTyperRustE2E_$PID"
$dataFolder = Join-Path $verificationRoot 'server-data'
$legacyRoomFolder = Join-Path $verificationRoot 'legacy-server-data'
$browserFolder = Join-Path $verificationRoot 'browser-profile'
$script:http = [System.Net.Http.HttpClient]::new()
$script:serverProcess = $null
$script:serverStdout = $null
$script:serverStderr = $null
$script:port = 0
$roomCode = '731204'

function Assert-Equal {
    param([object]$Actual, [object]$Expected, [string]$Message)
    if ($Actual -cne $Expected) {
        throw "$Message Ожидалось [$Expected], получено [$Actual]."
    }
}

function Get-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $freePort = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    $listener.Stop()
    return $freePort
}

function Start-TestServer {
    $script:port = Get-FreePort
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $resolvedServer
    $start.WorkingDirectory = [System.IO.Path]::GetDirectoryName($resolvedServer)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.EnvironmentVariables['LIVECODE_SERVER_HOST'] = '127.0.0.1'
    $start.EnvironmentVariables['LIVECODE_SERVER_PORT'] = [string]$script:port
    $start.EnvironmentVariables['LIVECODE_ROOM_CODE'] = $roomCode
    $start.EnvironmentVariables['LIVECODE_ROOM_DATA_FOLDER'] = $dataFolder
    $start.EnvironmentVariables['LIVECODETYPER_LEGACY_ROOM_DATA_FOLDER'] = $legacyRoomFolder
    $script:serverProcess = [System.Diagnostics.Process]::Start($start)
    if (-not $script:serverProcess) {
        throw 'Не удалось запустить сервер комнаты.'
    }
    $script:serverStdout = $script:serverProcess.StandardOutput.ReadToEndAsync()
    $script:serverStderr = $script:serverProcess.StandardError.ReadToEndAsync()
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    $script:baseUrl = "http://127.0.0.1:$script:port"
    while ([DateTime]::UtcNow -lt $deadline) {
        $script:serverProcess.Refresh()
        if ($script:serverProcess.HasExited) {
            throw "Сервер завершился при запуске: $($script:serverStderr.GetAwaiter().GetResult())"
        }
        try {
            $health = Invoke-RoomRequest -Method 'GET' -Path '/health'
            if ($health.Status -eq 200) { return }
        } catch { }
        Start-Sleep -Milliseconds 150
    }
    throw "Сервер не ответил вовремя. $($script:serverStderr.GetAwaiter().GetResult())"
}

function Invoke-RoomTypingProbe {
    param([string]$ParticipantId)

    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $resolvedTypingProbe
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    foreach ($argument in @($script:baseUrl, $roomCode, 'Саят', $ParticipantId)) {
        $start.ArgumentList.Add($argument)
    }
    $probe = [System.Diagnostics.Process]::Start($start)
    if (-not $probe) { throw 'Не удалось запустить проверку автоматической печати.' }
    try {
        $stdout = $probe.StandardOutput.ReadToEndAsync()
        $stderr = $probe.StandardError.ReadToEndAsync()
        if (-not $probe.WaitForExit(30000)) {
            $probe.Kill($true)
            $null = $probe.WaitForExit(5000)
            throw 'Проверка автоматической печати не завершилась за 30 секунд.'
        }
        $output = $stdout.GetAwaiter().GetResult().Trim()
        $errorOutput = $stderr.GetAwaiter().GetResult().Trim()
        if ($probe.ExitCode -ne 0) {
            throw "Проверка автоматической печати завершилась с ошибкой: $errorOutput $output"
        }
        if ($output -notmatch '^PASS: room assignment auto-started;') {
            throw "Проверка не вернула ожидаемое подтверждение: $output"
        }
        Write-Output "PASS: $output"
    } finally {
        $probe.Dispose()
    }
}

function Stop-TestServer {
    if ($script:serverProcess -and -not $script:serverProcess.HasExited) {
        $script:serverProcess.Kill()
        $null = $script:serverProcess.WaitForExit(5000)
    }
    if ($script:serverProcess) {
        $script:serverProcess.Dispose()
        $script:serverProcess = $null
    }
}

function Invoke-RoomRequest {
    param(
        [Parameter(Mandatory = $true)][string]$Method,
        [Parameter(Mandatory = $true)][string]$Path,
        [AllowNull()][object]$Body = $null
    )

    $request = [System.Net.Http.HttpRequestMessage]::new(
        [System.Net.Http.HttpMethod]::new($Method),
        "$script:baseUrl$Path")
    if ($null -ne $Body) {
        $json = ConvertTo-Json -InputObject $Body -Depth 100 -Compress
        $request.Content = [System.Net.Http.StringContent]::new(
            $json,
            [System.Text.Encoding]::UTF8,
            'application/json')
    }
    $response = $script:http.SendAsync($request).GetAwaiter().GetResult()
    try {
        $text = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult()
        $parsed = $null
        if (-not [string]::IsNullOrWhiteSpace($text)) {
            try { $parsed = ConvertFrom-Json -InputObject $text -Depth 100 } catch { }
        }
        return [pscustomobject]@{
            Status = [int]$response.StatusCode
            Text = $text
            Json = $parsed
        }
    } finally {
        $response.Dispose()
        $request.Dispose()
    }
}

try {
    New-Item -ItemType Directory -Path $dataFolder -Force | Out-Null
    Start-TestServer
    $base = $script:baseUrl

    $health = Invoke-RoomRequest -Method 'GET' -Path '/health'
    Assert-Equal $health.Status 200 'Health endpoint.'
    Assert-Equal $health.Json.status 'ok' 'Health response.'
    $info = Invoke-RoomRequest -Method 'GET' -Path '/api/info'
    Assert-Equal $info.Status 200 'Room connection info endpoint.'
    if ($null -eq $info.Json.roomInviteBases) { throw 'Информация комнаты не содержит список ссылок локальной сети.' }
    $index = Invoke-RoomRequest -Method 'GET' -Path '/'
    Assert-Equal $index.Status 200 'Web panel document.'
    if ($index.Text -notmatch 'id="joinForm"') { throw 'В HTML-панели нет формы входа.' }
    $scriptAsset = Invoke-RoomRequest -Method 'GET' -Path '/app.js'
    Assert-Equal $scriptAsset.Status 200 'Web panel JavaScript.'
    if ($scriptAsset.Text -notmatch 'quick-send') { throw 'JavaScript-панель не содержит быструю отправку.' }
    $styleAsset = Invoke-RoomRequest -Method 'GET' -Path '/styles.css'
    if ($styleAsset.Status -ne 200 -or
        $styleAsset.Text -notmatch '@media\s*\(max-width:\s*680px\)' -or
        $styleAsset.Text -notmatch 'mobile-detail-open') {
        throw 'Не найдены мобильный брейкпоинт или режим навигации узкого экрана.'
    }
    Write-Output 'PASS: сервер, HTML, JavaScript и адаптивные стили доступны по HTTP.'

    $wrongCode = Invoke-RoomRequest -Method 'GET' -Path '/api/room?roomCode=000000'
    Assert-Equal $wrongCode.Status 401 'Invalid room code.'
    $room = Invoke-RoomRequest -Method 'GET' -Path "/api/room?roomCode=$roomCode"
    Assert-Equal $room.Status 200 'Valid room code.'
    Assert-Equal $room.Json.version 0 'New room version.'

    $firstJoin = Invoke-RoomRequest -Method 'POST' -Path '/api/join' -Body @{
        roomCode = $roomCode; name = 'Аружан'; participantId = $null
    }
    $secondJoin = Invoke-RoomRequest -Method 'POST' -Path '/api/join' -Body @{
        roomCode = $roomCode; name = 'Саят'; participantId = $null
    }
    Assert-Equal $firstJoin.Status 200 'First participant join.'
    Assert-Equal $secondJoin.Status 200 'Second participant join.'
    if ([string]::IsNullOrWhiteSpace($secondJoin.Json.participantId)) {
        throw 'Сервер не создал ID участника.'
    }
    Write-Output 'PASS: несколько участников входят в одну комнату и имеют одинаковый доступ.'

    $taskCode = 'using System;' + "`n" + 'Console.WriteLine("Сәлем, 🌟");'
    $taskResponse = Invoke-RoomRequest -Method 'POST' -Path '/api/tasks' -Body @{
        roomCode = $roomCode
        actorName = 'Аружан'
        id = $null
        number = '12'
        title = 'Сумма двух чисел'
        language = 'C#'
        code = $taskCode
    }
    Assert-Equal $taskResponse.Status 200 'Create task.'
    Assert-Equal $taskResponse.Json.code $taskCode 'Saved task code.'
    $assignment = Invoke-RoomRequest -Method 'POST' -Path '/api/assign' -Body @{
        roomCode = $roomCode
        actorName = 'Аружан'
        participantId = $secondJoin.Json.participantId
        taskId = $taskResponse.Json.id
    }
    Assert-Equal $assignment.Status 200 'Assign task.'
    $pollRequest = @{
        roomCode = $roomCode
        participantId = $secondJoin.Json.participantId
    }
    $assignedPoll = Invoke-RoomRequest -Method 'POST' -Path '/api/poll' -Body $pollRequest
    Assert-Equal $assignedPoll.Json.assignedTask.code $taskCode 'Code delivered to assigned participant.'
    if ($resolvedTypingProbe) {
        Invoke-RoomTypingProbe -ParticipantId $secondJoin.Json.participantId
    }
    $revisionBeforeDelete = [long]$assignedPoll.Json.assignmentRevision

    $encodedActor = [Uri]::EscapeDataString('Саят')
    $deleted = Invoke-RoomRequest -Method 'DELETE' -Path "/api/tasks/$($taskResponse.Json.id)?roomCode=$roomCode&actorName=$encodedActor"
    Assert-Equal $deleted.Status 200 'Delete assigned task.'
    $clearedPoll = Invoke-RoomRequest -Method 'POST' -Path '/api/poll' -Body $pollRequest
    Assert-Equal $clearedPoll.Json.assignedTask $null 'Deleting an assignment clears it.'
    Assert-Equal ([long]$clearedPoll.Json.assignmentRevision) ($revisionBeforeDelete + 1) 'Assignment revision after deletion.'
    Write-Output 'PASS: заданный код приходит участнику, а удаление очищает назначение и меняет ревизию.'

    $quickText = "Сәлем, Саят!`r`nConsole.WriteLine(12); 🌟"
    $quick = Invoke-RoomRequest -Method 'POST' -Path '/api/quick-send' -Body @{
        roomCode = $roomCode
        actorName = 'Аружан'
        participantId = $secondJoin.Json.participantId
        text = $quickText
    }
    Assert-Equal $quick.Status 200 'Quick send arbitrary Unicode text.'
    Assert-Equal $quick.Json.code $quickText 'Quick-send text.'
    Assert-Equal $quick.Json.isQuickSend $true 'Quick-send flag.'
    $quickPoll = Invoke-RoomRequest -Method 'POST' -Path '/api/poll' -Body $pollRequest
    Assert-Equal $quickPoll.Json.assignedTask.code $quickText 'Quick text delivered to participant.'

    foreach ($command in @('pause', 'restart')) {
        $commandResponse = Invoke-RoomRequest -Method 'POST' -Path '/api/command' -Body @{
            roomCode = $roomCode
            actorName = 'Аружан'
            participantId = $secondJoin.Json.participantId
            command = $command
        }
        Assert-Equal $commandResponse.Status 200 "Room command $command."
        $commandPoll = Invoke-RoomRequest -Method 'POST' -Path '/api/poll' -Body $pollRequest
        Assert-Equal $commandPoll.Json.command $command "Delivered room command $command."
    }
    $invalidCommand = Invoke-RoomRequest -Method 'POST' -Path '/api/command' -Body @{
        roomCode = $roomCode; actorName = 'Аружан'; participantId = $secondJoin.Json.participantId; command = 'delete'
    }
    Assert-Equal $invalidCommand.Status 400 'Invalid room command is rejected.'
    Assert-Equal $invalidCommand.Json.error 'Поддерживаются команды паузы и перезапуска.' 'Invalid command message.'
    Write-Output 'PASS: произвольный текст с Unicode и переносом строк и команды комнаты доставляются участнику.'

    Stop-TestServer
    Start-TestServer
    $persistedRoom = Invoke-RoomRequest -Method 'GET' -Path "/api/room?roomCode=$roomCode"
    Assert-Equal $persistedRoom.Status 200 'Room survives server restart.'
    Assert-Equal $persistedRoom.Json.tasks.Count 0 'Deleted tasks stay deleted after restart.'
    if (-not ($persistedRoom.Json.actions | Where-Object { $_.action -eq 'Отправил быстрый текст' })) {
        throw 'История действий комнаты не сохранилась после перезапуска.'
    }
    $persistedPoll = Invoke-RoomRequest -Method 'POST' -Path '/api/poll' -Body $pollRequest
    Assert-Equal $persistedPoll.Json.assignedTask.code $quickText 'Quick send survives server restart.'
    Write-Output 'PASS: код, история, участники и быстрый текст переживают перезапуск сервера.'

    $node = Get-Command node -ErrorAction Stop
    $edgeCandidates = @(
        'C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe',
        'C:\Program Files\Microsoft\Edge\Application\msedge.exe'
    )
    $edge = $edgeCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
    if (-not $edge) { throw 'Для E2E панели нужен установленный Microsoft Edge.' }
    $browserPort = Get-FreePort
    New-Item -ItemType Directory -Path $browserFolder -Force | Out-Null
    $edgeArguments = @(
        '--headless=new', '--disable-gpu', '--no-first-run', '--no-default-browser-check',
        '--remote-debugging-address=127.0.0.1', "--remote-debugging-port=$browserPort",
        "--user-data-dir=`"$browserFolder`"", 'about:blank'
    ) -join ' '
    $browserProcess = Start-Process -FilePath $edge -ArgumentList $edgeArguments -WindowStyle Hidden -PassThru
    try {
        & $node.Source (Join-Path $PSScriptRoot 'Verify-RoomPanel.mjs') $browserPort $script:baseUrl $roomCode
        if ($LASTEXITCODE -ne 0) { throw 'E2E браузерной панели завершился с ошибкой.' }
    } finally {
        if ($browserProcess -and -not $browserProcess.HasExited) {
            $browserProcess.Kill($true)
            $null = $browserProcess.WaitForExit(5000)
        }
    }
    Write-Output 'PASS: веб-панель проверена через настоящий headless Edge на узком и широком экране.'

    $leave = Invoke-RoomRequest -Method 'POST' -Path '/api/leave' -Body @{
        roomCode = $roomCode; participantId = $secondJoin.Json.participantId
    }
    Assert-Equal $leave.Status 200 'Participant leave.'
    $offlineRoom = Invoke-RoomRequest -Method 'GET' -Path "/api/room?roomCode=$roomCode"
    $offline = $offlineRoom.Json.participants | Where-Object { $_.id -eq $secondJoin.Json.participantId }
    Assert-Equal $offline.isOnline $false 'Left participant status.'
    Write-Output 'PASS: выход участника сразу меняет онлайн-статус.'
    Stop-TestServer
}
finally {
    Stop-TestServer
    if ($script:http) { $script:http.Dispose() }
    $resolvedRoot = [System.IO.Path]::GetFullPath($verificationRoot)
    $resolvedTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if ($resolvedRoot.StartsWith($resolvedTemp, [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $resolvedRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
