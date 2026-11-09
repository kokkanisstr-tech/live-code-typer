param(
    [Parameter(Mandatory = $true)]
    [string]$ServerPath
)

$ErrorActionPreference = 'Stop'
$resolvedServer = (Resolve-Path -LiteralPath $ServerPath).Path
$tempPrefix = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
$rootFolder = [System.IO.Path]::GetFullPath(
    (Join-Path ([System.IO.Path]::GetTempPath()) "LiveCodeTyperRoomMigrationE2E_$PID")
)
if (-not $rootFolder.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'The room migration check path is outside the temporary directory.'
}
if (Test-Path -LiteralPath $rootFolder) {
    throw 'The unique room migration check directory already exists; refusing to overwrite it.'
}

$legacyFolder = Join-Path $rootFolder 'legacy-server'
$portableFolder = Join-Path $rootFolder 'portable-data'
$legacyFile = Join-Path $legacyFolder 'room.json'
$portableFile = Join-Path $portableFolder 'room.json'
$taskId = [guid]::NewGuid().ToString('N')
$participantId = [guid]::NewGuid().ToString('N')
$timestamp = '2026-11-03T06:00:00'
$taskCode = 'Console.WriteLine(12);' + "`n" + 'Console.WriteLine("Сәлем");'
$legacy = [ordered]@{
    RoomCode = '739146'
    Version = 12
    Tasks = @(
        [ordered]@{
            Id = $taskId
            Number = '12'
            Title = 'Сумма двух чисел'
            Language = 'C#'
            Code = $taskCode
            UpdatedAtUtc = $timestamp
            IsQuickSend = $false
            QuickSendForParticipantId = $null
        }
    )
    Participants = @(
        [ordered]@{
            Id = $participantId
            Name = 'Саят'
            LastSeenUtc = $timestamp
            AssignedTaskId = $taskId
            AssignmentRevision = 4
            CommandVersion = 3
            LastCommand = 'pause'
        }
    )
    Actions = @(
        [ordered]@{
            ActorName = 'Айжан'
            Action = 'Назначил задание'
            TargetName = 'Саят'
            Details = '№12 · Сумма двух чисел'
            CreatedAtUtc = $timestamp
        }
    )
}
$json = $legacy | ConvertTo-Json -Depth 12

function Invoke-PortableRoomMigration {
    param(
        [string]$Executable,
        [string]$TargetFolder,
        [string]$SourceFolder
    )

    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $Executable
    $start.WorkingDirectory = [System.IO.Path]::GetDirectoryName($Executable)
    $start.ArgumentList.Add('--migrate-data')
    $start.EnvironmentVariables['LIVECODE_ROOM_DATA_FOLDER'] = $TargetFolder
    $start.EnvironmentVariables['LIVECODETYPER_LEGACY_ROOM_DATA_FOLDER'] = $SourceFolder
    $null = $start.EnvironmentVariables.Remove('LIVECODE_ROOM_CODE')
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $process = [System.Diagnostics.Process]::Start($start)
    if (-not $process.WaitForExit(15000)) {
        $process.Kill($true)
        $null = $process.WaitForExit(5000)
        throw 'The background room migration command did not finish within 15 seconds.'
    }
    if ($process.ExitCode -ne 0) {
        throw "The background room migration command exited with code $($process.ExitCode)."
    }
    $process.Dispose()
}

try {
    $null = New-Item -ItemType Directory -Path $legacyFolder -Force
    $null = New-Item -ItemType Directory -Path $portableFolder -Force
    [System.IO.File]::WriteAllText($legacyFile, $json, [System.Text.UTF8Encoding]::new($false))
    $sourceHash = (Get-FileHash -LiteralPath $legacyFile -Algorithm SHA256).Hash

    Invoke-PortableRoomMigration -Executable $resolvedServer -TargetFolder $portableFolder -SourceFolder $legacyFolder
    if (-not (Test-Path -LiteralPath $portableFile -PathType Leaf)) {
        throw 'Portable room.json was not created.'
    }
    $converted = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($converted.roomCode -ne '739146' -or $converted.version -ne 12) {
        throw 'The room code or version was not preserved.'
    }
    if ($converted.tasks.Count -ne 1 -or $converted.tasks[0].id -ne $taskId) {
        throw 'The saved task was not preserved.'
    }
    if ($converted.tasks[0].code -cne $taskCode -or $converted.tasks[0].title -ne 'Сумма двух чисел') {
        throw 'The task source text or title changed during migration.'
    }
    if ($converted.participants.Count -ne 1 -or $converted.participants[0].id -ne $participantId) {
        throw 'The participant was not preserved.'
    }
    if ($converted.participants[0].name -ne 'Саят' -or
        $converted.participants[0].assignedTaskId -ne $taskId -or
        $converted.participants[0].assignmentRevision -ne 4 -or
        $converted.participants[0].commandVersion -ne 3 -or
        $converted.participants[0].lastCommand -ne 'pause') {
        throw 'The participant name or current assignment state changed during migration.'
    }
    if ($converted.actions.Count -ne 1 -or
        $converted.actions[0].actorName -ne 'Айжан' -or
        $converted.actions[0].targetName -ne 'Саят' -or
        $converted.actions[0].details -ne '№12 · Сумма двух чисел') {
        throw 'The room action history was not preserved.'
    }
    if ((Get-FileHash -LiteralPath $legacyFile -Algorithm SHA256).Hash -ne $sourceHash) {
        throw 'The original room file changed during migration.'
    }

    Invoke-PortableRoomMigration -Executable $resolvedServer -TargetFolder $portableFolder -SourceFolder $legacyFolder
    $reloaded = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($reloaded.tasks.Count -ne 1 -or $reloaded.participants.Count -ne 1 -or $reloaded.actions.Count -ne 1) {
        throw 'The portable room did not reload idempotently.'
    }
    if ((Get-FileHash -LiteralPath $legacyFile -Algorithm SHA256).Hash -ne $sourceHash) {
        throw 'The original room file changed during the second load.'
    }

    Write-Output 'PASS: old room code, saved task, Unicode source, participant assignment, and action history migrate.'
    Write-Output 'PASS: room migration leaves the old file untouched and reloads the portable room idempotently.'
}
finally {
    Get-ChildItem -LiteralPath $portableFolder -Force -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $legacyFolder -Force -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $portableFolder,$legacyFolder -Force -ErrorAction SilentlyContinue
    $resolvedRoot = [System.IO.Path]::GetFullPath($rootFolder)
    if ($resolvedRoot.StartsWith($tempPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $resolvedRoot -Force -ErrorAction SilentlyContinue
    }
}
