param(
    [Parameter(Mandatory = $true)]
    [string]$AppPath
)

$ErrorActionPreference = 'Stop'
$resolvedApp = (Resolve-Path -LiteralPath $AppPath).Path
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\') + '\'
$rootFolder = [System.IO.Path]::GetFullPath(
    (Join-Path ([System.IO.Path]::GetTempPath()) "LiveCodeTyperMigrationE2E_$PID")
)
if (-not $rootFolder.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'The migration check path is outside the temporary directory.'
}
if (Test-Path -LiteralPath $rootFolder) {
    throw 'The unique migration check directory already exists; refusing to overwrite it.'
}

$legacyFolder = Join-Path $rootFolder 'legacy'
$portableFolder = Join-Path $rootFolder 'portable-data'
$legacyFile = Join-Path $legacyFolder 'library.json'
$portableFile = Join-Path $portableFolder 'library.json'
$null = New-Item -ItemType Directory -Path $legacyFolder -Force
$null = New-Item -ItemType Directory -Path $portableFolder -Force

$firstId = [guid]::NewGuid()
$selectedId = [guid]::NewGuid()
$participantId = [guid]::NewGuid()
$legacy = [ordered]@{
    SchemaVersion = 3
    OnlySupportedEditors = $false
    SelectedSnippetId = $selectedId
    Snippets = @(
        [ordered]@{
            Id = $firstId
            Name = 'Unicode и строки'
            Language = 'Текст'
            Code = "a👋`n"
            IsPinned = $true
            Position = 3
            UpdatedAt = '2026-11-03T00:00:00'
            IsRemoteAssignment = $false
        },
        [ordered]@{
            Id = $selectedId
            Name = 'Код для комнаты'
            Language = 'C#'
            Code = "первая строка`r`nвторая строка"
            IsPinned = $false
            Position = 0
            UpdatedAt = '2026-11-03T00:00:00'
            IsRemoteAssignment = $true
        }
    )
    Hotkeys = [ordered]@{
        ToggleTyping = @{ Modifiers = 3; VirtualKey = 0x20 }
        NextSnippet = @{ Modifiers = 3; VirtualKey = 0x5A }
        PreviousSnippet = @{ Modifiers = 3; VirtualKey = 0x58 }
        Restart = @{ Modifiers = 3; VirtualKey = 0x52 }
        TogglePin = @{ Modifiers = 3; VirtualKey = 0x4C }
        EmergencyStop = @{ Modifiers = 3; VirtualKey = 0x1B }
    }
    RoomServerUrl = 'http://127.0.0.1:5080'
    RoomCode = '862417'
    ParticipantName = 'Сохранённое имя'
    ParticipantId = $participantId.ToString()
}
$json = $legacy | ConvertTo-Json -Depth 12
[System.IO.File]::WriteAllText($legacyFile, $json, [System.Text.UTF8Encoding]::new($false))
[System.IO.File]::WriteAllText($portableFile, $json, [System.Text.UTF8Encoding]::new($false))
$sourceHash = (Get-FileHash -LiteralPath $legacyFile -Algorithm SHA256).Hash

function Invoke-PortableMigration {
    param(
        [string]$Executable,
        [string]$TargetFolder,
        [string]$SourceFolder
    )

    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $Executable
    $start.WorkingDirectory = [System.IO.Path]::GetDirectoryName($Executable)
    $start.ArgumentList.Add('--migrate-data')
    $start.EnvironmentVariables['LIVECODETYPER_DATA_FOLDER'] = $TargetFolder
    $start.EnvironmentVariables['LIVECODETYPER_LEGACY_DATA_FOLDER'] = $SourceFolder
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $process = [System.Diagnostics.Process]::Start($start)
    if (-not $process.WaitForExit(15000)) {
        $process.Kill($true)
        $null = $process.WaitForExit(5000)
        throw 'The background migration command did not finish within 15 seconds.'
    }
    if ($process.ExitCode -ne 0) {
        throw "The background migration command exited with code $($process.ExitCode)."
    }
    $process.Dispose()
}

try {
    Invoke-PortableMigration -Executable $resolvedApp -TargetFolder $portableFolder -SourceFolder $legacyFolder
    if (-not (Test-Path -LiteralPath $portableFile -PathType Leaf)) {
        throw 'Portable library.json was not created.'
    }
    $converted = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($converted.snippets.Count -ne 2) { throw 'The migrated snippet count is incorrect.' }
    if ($converted.selected_id -ne $selectedId.ToString()) { throw 'The selected snippet was not preserved.' }
    if ($converted.snippets[0].code -cne "a👋`n") { throw 'Unicode or line endings changed during migration.' }
    if ($converted.snippets[0].cursor -ne 2) { throw 'A UTF-16 cursor position was not converted to graphemes correctly.' }
    if (-not $converted.snippets[0].pinned) { throw 'The code lock was not preserved.' }
    if (-not $converted.snippets[1].is_remote_assignment) { throw 'The room assignment marker was not preserved.' }
    if ($converted.snippets[1].code -cne "первая строка`nвторая строка") { throw 'CRLF text was not normalized correctly.' }
    if ($converted.hotkeys.toggle_typing -ne 'Ctrl+Alt+Space') { throw 'The print shortcut was not preserved.' }
    if ($converted.hotkeys.next_snippet -ne 'Ctrl+Alt+Z') { throw 'The next-snippet shortcut was not preserved.' }
    if ($converted.hotkeys.previous_snippet -ne 'Ctrl+Alt+X') { throw 'The previous-snippet shortcut was not preserved.' }
    if ($converted.hotkeys.emergency_stop -ne 'Ctrl+Alt+Escape') { throw 'The emergency-stop shortcut was not preserved.' }
    if ($converted.room.serverUrl -ne 'http://127.0.0.1:5080') { throw 'The room server address was not preserved.' }
    if ($converted.room.roomCode -ne '862417') { throw 'The room code was not preserved.' }
    if ($converted.room.participantName -ne 'Сохранённое имя') { throw 'The participant name was not preserved.' }
    if ($converted.room.participantId -ne $participantId.ToString()) { throw 'The room participant identifier was not preserved.' }
    if ($converted.onboarding_complete) { throw 'The first-launch instructions should be shown after an upgrade.' }
    if ((Get-FileHash -LiteralPath $legacyFile -Algorithm SHA256).Hash -ne $sourceHash) {
        throw 'The original library file changed during migration.'
    }

    Invoke-PortableMigration -Executable $resolvedApp -TargetFolder $portableFolder -SourceFolder $legacyFolder
    $reloaded = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($reloaded.snippets.Count -ne 2 -or $reloaded.selected_id -ne $selectedId.ToString()) {
        throw 'The migrated file could not be loaded and saved a second time.'
    }
    if ((Get-FileHash -LiteralPath $legacyFile -Algorithm SHA256).Hash -ne $sourceHash) {
        throw 'The original library file changed during the second load.'
    }

    Remove-Item -LiteralPath $portableFile -Force
    Invoke-PortableMigration -Executable $resolvedApp -TargetFolder $portableFolder -SourceFolder $legacyFolder
    $isolatedDemo = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($isolatedDemo.snippets.Count -ne 25) {
        throw 'The demo imported data from the original app instead of loading its own 25 tasks.'
    }
    for ($index = 0; $index -lt 25; $index++) {
        if (-not $isolatedDemo.snippets[$index].name.StartsWith("Задание $($index + 1) · ")) {
            throw 'The demo default task list changed while checking app-data isolation.'
        }
    }
    $oldDemo = $isolatedDemo | ConvertTo-Json -Depth 20 | ConvertFrom-Json
    $oldDemo.snippets = @($oldDemo.snippets | Select-Object -First 7) + @($converted.snippets[0])
    $oldDemo.selected_id = $converted.snippets[0].id
    $oldClasses = @('Kompyuter','Bagdarlama','Server','Paydalanushy','Fail','Derekqor','JelilikQurylgy')
    $oldIds = @($oldDemo.snippets | Select-Object -First 7 | ForEach-Object { $_.id })
    for ($index = 0; $index -lt 7; $index++) {
        $oldDemo.snippets[$index].name = "Задание $($index + 1)"
        $oldDemo.snippets[$index].code = "using System;`nclass $($oldClasses[$index]) {}"
    }
    [System.IO.File]::WriteAllText($portableFile, ($oldDemo | ConvertTo-Json -Depth 20), [System.Text.UTF8Encoding]::new($false))
    Invoke-PortableMigration -Executable $resolvedApp -TargetFolder $portableFolder -SourceFolder $legacyFolder
    $upgraded = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($upgraded.snippets.Count -ne 26 -or $upgraded.selected_id -ne $oldDemo.selected_id) {
        throw 'Upgrading the seven-task demo lost the custom snippet or failed to add all 25 tasks.'
    }
    for ($index = 0; $index -lt 7; $index++) {
        if ($upgraded.snippets[$index].id -ne $oldIds[$index] -or
            -not $upgraded.snippets[$index].name.StartsWith("Задание $($index + 1) · ")) {
            throw 'Upgrading an old prepared task did not preserve its identity.'
        }
    }
    if ($upgraded.snippets[0].code -notmatch 'abstract class Lesson') {
        throw 'The first task was not replaced with the new DOCX code.'
    }
    Invoke-PortableMigration -Executable $resolvedApp -TargetFolder $portableFolder -SourceFolder $legacyFolder
    $upgradedAgain = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($upgradedAgain.snippets.Count -ne 26) { throw 'The task upgrade duplicated prepared snippets on second launch.' }
    $customOnly = $converted | ConvertTo-Json -Depth 20 | ConvertFrom-Json
    $customOnly.onboarding_complete = $true
    [System.IO.File]::WriteAllText($portableFile, ($customOnly | ConvertTo-Json -Depth 20), [System.Text.UTF8Encoding]::new($false))
    Invoke-PortableMigration -Executable $resolvedApp -TargetFolder $portableFolder -SourceFolder $legacyFolder
    $restored = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($restored.snippets.Count -ne 27 -or $restored.selected_id -ne $selectedId.ToString()) {
        throw 'Adding prepared tasks to a previously used custom-only demo lost user data.'
    }
    if (@($restored.snippets | Where-Object { $_.id -eq $selectedId.ToString() }).Count -ne 1) {
        throw 'The selected custom snippet was duplicated or removed.'
    }
    Invoke-PortableMigration -Executable $resolvedApp -TargetFolder $portableFolder -SourceFolder $legacyFolder
    $restoredAgain = Get-Content -Raw -LiteralPath $portableFile | ConvertFrom-Json
    if ($restoredAgain.snippets.Count -ne 27) { throw 'Refreshing custom-only data duplicated prepared tasks.' }
    if ((Get-FileHash -LiteralPath $legacyFile -Algorithm SHA256).Hash -ne $sourceHash) {
        throw 'The demo changed the original app data during the isolation check.'
    }

    Write-Output 'PASS: older demo data retains Unicode, newlines, cursor position, pin lock, shortcuts and room settings.'
    Write-Output 'PASS: migration is repeatable and the demo stays separate from the original app library.'
    Write-Output 'PASS: previously used custom-only demo data receives 25 tasks while preserving custom snippets.'
}
finally {
    Get-ChildItem -LiteralPath $portableFolder -Force -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $legacyFolder -Force -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $portableFolder,$legacyFolder -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $rootFolder -Force -ErrorAction SilentlyContinue
}
