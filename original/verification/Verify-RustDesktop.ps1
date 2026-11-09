param(
    [Parameter(Mandatory = $true)]
    [string]$AppPath,
    [Parameter(Mandatory = $true)]
    [string]$ServerPath,
    [switch]$DemoHotkey
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class RustDesktopE2E
{
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr window);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr window, int command);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr window, StringBuilder text, int maxCount);

    public static string GetForegroundTitle()
    {
        var title = new StringBuilder(512);
        GetWindowText(GetForegroundWindow(), title, title.Capacity);
        return title.ToString();
    }

    [DllImport("user32.dll")]
    public static extern void keybd_event(byte virtualKey, byte scanCode, uint flags, UIntPtr extraInfo);

    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extraInfo);

}
'@

$resolvedApp = (Resolve-Path -LiteralPath $AppPath).Path
$resolvedServer = (Resolve-Path -LiteralPath $ServerPath).Path
$rootFolder = Join-Path ([System.IO.Path]::GetTempPath()) "LiveCodeTyperRustDesktopE2E_$PID"
$dataFolder = Join-Path $rootFolder 'portable-data'
$serverDataFolder = Join-Path $rootFolder 'server-data'
$legacyRoomFolder = Join-Path $rootFolder 'legacy-server-data'
$firstOutput = Join-Path $rootFolder 'first-run.txt'
$secondOutput = Join-Path $rootFolder 'custom-hotkey.txt'
$roomOutput = Join-Path $rootFolder 'room-assignment.txt'
$targetScript = Join-Path $PSScriptRoot 'VerificationTarget.ps1'
$targetPath = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$appProcess = $null
$targetProcess = $null
$roomServerProcess = $null
$roomServerStdout = $null
$roomServerStderr = $null
$roomPort = 0
$roomCode = '731204'
$roomBaseUrl = ''
$snippetId = [guid]::NewGuid().ToString()

function Wait-Until {
    param(
        [Parameter(Mandatory = $true)][scriptblock]$Condition,
        [Parameter(Mandatory = $true)][string]$FailureMessage,
        [int]$TimeoutSeconds = 10
    )

    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    while ($timer.Elapsed.TotalSeconds -lt $TimeoutSeconds) {
        $value = & $Condition
        if ($value) { return $value }
        Start-Sleep -Milliseconds 100
    }
    throw $FailureMessage
}

function Get-Data {
    $path = Join-Path $dataFolder 'library.json'
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    return Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Write-Library {
    param([string]$Text, [int]$Cursor = 0, [switch]$DemoHotkey)
    New-Item -ItemType Directory -Path $dataFolder -Force | Out-Null
    $path = [System.IO.Path]::Combine($dataFolder, 'library.json')
    $secondSnippetId = if ($DemoHotkey) { [guid]::NewGuid().ToString() } else { $null }
    $snippets = @(
        [ordered]@{
            id = $snippetId
            name = 'Задание 1'
            language = 'Текст'
            code = $Text
            pinned = [bool]$DemoHotkey
            cursor = $Cursor
            revision = 0
            remote_key = $null
            is_remote_assignment = $false
        }
    )
    if ($DemoHotkey) {
        $snippets += [ordered]@{
            id = $secondSnippetId
            name = 'Задание 2 — выбрать неправильно'
            language = 'Текст'
            code = 'НЕ ДОЛЖНО НАПЕЧАТАТЬСЯ'
            pinned = $true
            cursor = 0
            revision = 0
            remote_key = $null
            is_remote_assignment = $false
        }
    }
    $library = [ordered]@{
        snippets = $snippets
        selected_id = if ($DemoHotkey) { $secondSnippetId } else { $snippetId }
        hotkeys = [ordered]@{
            toggle_typing = 'Ctrl+Alt+Space'
            next_snippet = 'Ctrl+Alt+ArrowRight'
            previous_snippet = 'Ctrl+Alt+ArrowLeft'
            restart = 'Ctrl+Alt+R'
            toggle_pin = 'Ctrl+Alt+L'
            emergency_stop = 'Ctrl+Alt+Escape'
        }
        room = @{
            serverUrl = 'http://localhost:5080'
            roomCode = ''
            participantName = ''
            participantId = $null
        }
    }
    $json = ConvertTo-Json -InputObject $library -Depth 20
    [System.IO.File]::WriteAllText($path, $json, [System.Text.UTF8Encoding]::new($false))
}

function Start-App {
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $resolvedApp
    $start.WorkingDirectory = [System.IO.Path]::GetDirectoryName($resolvedApp)
    $start.UseShellExecute = $false
    $start.EnvironmentVariables['LIVECODETYPER_DATA_FOLDER'] = $dataFolder
    $script:appProcess = [System.Diagnostics.Process]::Start($start)
    if (-not $script:appProcess) { throw 'Не удалось запустить настольное приложение.' }
    Wait-Until -TimeoutSeconds 15 -FailureMessage 'Главное окно приложения не открылось.' -Condition {
        $script:appProcess.Refresh()
        $script:appProcess.HasExited -or
            ($script:appProcess.MainWindowHandle -ne [IntPtr]::Zero -and
                $script:appProcess.MainWindowTitle.StartsWith('LiveCode Typer', [System.StringComparison]::Ordinal))
    } | Out-Null
    if ($script:appProcess.HasExited) {
        throw "Приложение завершилось с кодом $($script:appProcess.ExitCode). Проверьте недостающие DLL среды выполнения."
    }
}

function Get-FreePort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    $listener.Stop()
    return $port
}

function Start-RoomServer {
    New-Item -ItemType Directory -Path $serverDataFolder -Force | Out-Null
    $script:roomPort = Get-FreePort
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $resolvedServer
    $start.WorkingDirectory = [System.IO.Path]::GetDirectoryName($resolvedServer)
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.EnvironmentVariables['LIVECODE_SERVER_HOST'] = '127.0.0.1'
    $start.EnvironmentVariables['LIVECODE_SERVER_PORT'] = [string]$script:roomPort
    $start.EnvironmentVariables['LIVECODE_ROOM_CODE'] = $roomCode
    $start.EnvironmentVariables['LIVECODE_ROOM_DATA_FOLDER'] = $serverDataFolder
    $start.EnvironmentVariables['LIVECODETYPER_LEGACY_ROOM_DATA_FOLDER'] = $legacyRoomFolder
    $script:roomServerProcess = [System.Diagnostics.Process]::Start($start)
    if (-not $script:roomServerProcess) { throw 'Не удалось запустить временный сервер комнаты.' }
    $script:roomServerStdout = $script:roomServerProcess.StandardOutput.ReadToEndAsync()
    $script:roomServerStderr = $script:roomServerProcess.StandardError.ReadToEndAsync()
    $script:roomBaseUrl = "http://127.0.0.1:$script:roomPort"
    $deadline = [DateTime]::UtcNow.AddSeconds(12)
    while ([DateTime]::UtcNow -lt $deadline) {
        $script:roomServerProcess.Refresh()
        if ($script:roomServerProcess.HasExited) {
            $stderr = $script:roomServerStderr.GetAwaiter().GetResult()
            throw "Сервер комнаты завершился при запуске: $stderr"
        }
        try {
            $health = Invoke-RestMethod -Uri "$script:roomBaseUrl/health" -TimeoutSec 2
            if ($health.status -eq 'ok') { return }
        } catch { }
        Start-Sleep -Milliseconds 150
    }
    $stderr = $script:roomServerStderr.GetAwaiter().GetResult()
    throw "Сервер комнаты не ответил вовремя: $stderr"
}

function Stop-RoomServer {
    if ($script:roomServerProcess -and -not $script:roomServerProcess.HasExited) {
        try { $script:roomServerProcess.Kill($true) } catch { $script:roomServerProcess.Kill() }
        $null = $script:roomServerProcess.WaitForExit(5000)
    }
    if ($script:roomServerProcess) {
        $script:roomServerProcess.Dispose()
        $script:roomServerProcess = $null
    }
}

function Send-RoomQuickText {
    param([string]$ParticipantId, [string]$Text)
    $body = @{
        roomCode = $roomCode
        actorName = 'Проверка E2E'
        participantId = $ParticipantId
        text = $Text
    } | ConvertTo-Json -Depth 10 -Compress
    return Invoke-RestMethod `
        -Uri "$roomBaseUrl/api/quick-send" `
        -Method Post `
        -ContentType 'application/json' `
        -Body $body `
        -TimeoutSec 5
}

function Send-RoomCommand {
    param([string]$ParticipantId, [string]$Command)
    $body = @{
        roomCode = $roomCode
        actorName = 'Проверка E2E'
        participantId = $ParticipantId
        command = $Command
    } | ConvertTo-Json -Depth 10 -Compress
    return Invoke-RestMethod `
        -Uri "$roomBaseUrl/api/command" `
        -Method Post `
        -ContentType 'application/json' `
        -Body $body `
        -TimeoutSec 5
}

function Stop-ProcessTree {
    param([System.Diagnostics.Process]$Process)
    if ($Process -and -not $Process.HasExited) {
        try { $Process.Kill($true) } catch { $Process.Kill() }
        $null = $Process.WaitForExit(5000)
        $Process.Dispose()
    }
}

function Send-Combination {
    param([byte[]]$Modifiers, [byte]$Key)
    foreach ($modifier in $Modifiers) {
        [RustDesktopE2E]::keybd_event($modifier, 0, 0, [UIntPtr]::Zero)
    }
    [RustDesktopE2E]::keybd_event($Key, 0, 0, [UIntPtr]::Zero)
    [RustDesktopE2E]::keybd_event($Key, 0, 2, [UIntPtr]::Zero)
    for ($index = $Modifiers.Length - 1; $index -ge 0; $index--) {
        [RustDesktopE2E]::keybd_event($Modifiers[$index], 0, 2, [UIntPtr]::Zero)
    }
}

function Send-RandomKey {
    [RustDesktopE2E]::keybd_event(0x58, 0, 0, [UIntPtr]::Zero)
    [RustDesktopE2E]::keybd_event(0x58, 0, 2, [UIntPtr]::Zero)
}

function Start-TextTarget {
    param([string]$OutputPath)
    $quote = [char]34
    $arguments = @(
        '-NoProfile', '-STA', '-ExecutionPolicy', 'Bypass',
        '-File', "$quote$targetScript$quote",
        '-OutputPath', "$quote$OutputPath$quote"
    ) -join ' '
    $script:targetProcess = Start-Process -FilePath $targetPath -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if (-not $script:targetProcess) { throw 'Не удалось открыть целевое текстовое окно.' }
    Wait-Until -TimeoutSeconds 10 -FailureMessage 'Тестовое окно WPF не открылось.' -Condition {
        $script:targetProcess.Refresh()
        if ($script:targetProcess.HasExited) { return $false }
        $script:targetProcess.MainWindowHandle -ne [IntPtr]::Zero
    } | Out-Null
    $targetProcess.Refresh()
    $targetHandle = $targetProcess.MainWindowHandle
    [RustDesktopE2E]::ShowWindow($targetHandle, 9) | Out-Null
    [RustDesktopE2E]::SetForegroundWindow($targetHandle) | Out-Null
    $targetRoot = [System.Windows.Automation.AutomationElement]::FromHandle($targetHandle)
    $editCondition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Edit)
    $edit = $targetRoot.FindFirst(
        [System.Windows.Automation.TreeScope]::Descendants,
        $editCondition)
    if (-not $edit) { throw 'В тестовом окне не найден настоящий текстовый ввод.' }
    $edit.SetFocus()
    [RustDesktopE2E]::SetForegroundWindow($targetHandle) | Out-Null
    $script:targetEdit = $edit
    Wait-Until -TimeoutSeconds 5 -FailureMessage 'Тестовое окно не вышло на передний план.' -Condition {
        Start-Sleep -Milliseconds 80
        [RustDesktopE2E]::GetForegroundTitle() -eq 'LiveCodeTyper Verification Target'
    } | Out-Null
    Start-Sleep -Milliseconds 250
}

function Find-UiElement {
    param(
        [System.Windows.Automation.AutomationElement]$Root,
        [string]$NameFragment,
        [System.Windows.Automation.ControlType]$ControlType = $null
    )
    $all = $Root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    foreach ($element in $all) {
        $name = $element.Current.Name
        if ($name -and $name.IndexOf($NameFragment, [System.StringComparison]::OrdinalIgnoreCase) -ge 0 -and
            ($null -eq $ControlType -or $element.Current.ControlType -eq $ControlType)) {
            return $element
        }
    }
    return $null
}

function Wait-UiElement {
    param(
        [System.Windows.Automation.AutomationElement]$Root,
        [string]$NameFragment,
        [System.Windows.Automation.ControlType]$ControlType
    )
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    while ($timer.Elapsed.TotalSeconds -lt 10) {
        $element = Find-UiElement -Root $Root -NameFragment $NameFragment -ControlType $ControlType
        if ($null -ne $element) { return $element }
        Start-Sleep -Milliseconds 120
    }
    $all = $Root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.Condition]::TrueCondition)
    $available = @($all | ForEach-Object { $_.Current.Name } | Where-Object { $_ }) -join ' | '
    throw "Не найден элемент '$NameFragment' в '$($Root.Current.Name)' (дочерних элементов: $($all.Count)). Доступные элементы: $available"
}

function Wait-UiText {
    param([System.Windows.Automation.AutomationElement]$Root, [string]$Text)
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    while ($timer.Elapsed.TotalSeconds -lt 5) {
        $all = $Root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
        foreach ($element in $all) {
            if ($element.Current.Name -and $element.Current.Name.Contains($Text)) { return $true }
        }
        Start-Sleep -Milliseconds 100
    }
    $available = @($all | ForEach-Object { $_.Current.Name } | Where-Object { $_ }) -join ' | '
    throw "В UI не появилось '$Text'. Текущие элементы: $available"
}

function Click-UiElement {
    param([System.Windows.Automation.AutomationElement]$Element)
    try {
        $pattern = $Element.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        ([System.Windows.Automation.InvokePattern]$pattern).Invoke()
        return
    } catch { }
    $rectangle = $Element.Current.BoundingRectangle
    if ($rectangle.IsEmpty) { throw "UIA-кнопка '$($Element.Current.Name)' не имеет доступной области." }
    [RustDesktopE2E]::SetCursorPos(
        [int]($rectangle.Left + $rectangle.Width / 2),
        [int]($rectangle.Top + $rectangle.Height / 2)) | Out-Null
    [RustDesktopE2E]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    [RustDesktopE2E]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
}

function Set-RoomConnectionFields {
    param(
        [System.Windows.Automation.AutomationElement]$Root,
        [string]$ServerUrl,
        [string]$RoomCode,
        [string]$ParticipantName
    )

    $editCondition = [System.Windows.Automation.PropertyCondition]::new(
        [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
        [System.Windows.Automation.ControlType]::Edit)
    $edits = $Root.FindAll(
        [System.Windows.Automation.TreeScope]::Descendants,
        $editCondition)
    $roomFields = @($edits | Where-Object {
        $rectangle = $_.Current.BoundingRectangle
        -not $rectangle.IsEmpty -and
        -not $_.Current.IsOffscreen -and
        $rectangle.Width -ge 180 -and
        $rectangle.Height -ge 15 -and
        $rectangle.Height -le 60
    } | Sort-Object { $_.Current.BoundingRectangle.Top } | Select-Object -Last 3)

    if ($roomFields.Count -ne 3) {
        $details = @($edits | ForEach-Object {
            $rectangle = $_.Current.BoundingRectangle
            "name='$($_.Current.Name)' bounds=$($rectangle.X),$($rectangle.Y),$($rectangle.Width),$($rectangle.Height) offscreen=$($_.Current.IsOffscreen)"
        }) -join '; '
        throw "Не удалось найти три поля подключения комнаты (найдено $($roomFields.Count)). Поля ввода: $details"
    }

    $values = @($ServerUrl, $RoomCode, $ParticipantName)
    for ($index = 0; $index -lt 3; $index++) {
        try {
            $pattern = $roomFields[$index].GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
            if (([System.Windows.Automation.ValuePattern]$pattern).Current.IsReadOnly) {
                throw 'Поле доступно только для чтения.'
            }
            ([System.Windows.Automation.ValuePattern]$pattern).SetValue($values[$index])
        } catch {
            throw "Не удалось заполнить поле подключения комнаты №$($index + 1): $($_.Exception.Message)"
        }
    }
}

function Get-LibraryText {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) { return $null }
    return [System.IO.File]::ReadAllText($Path).Replace(
        [Environment]::NewLine,
        ([char]10).ToString())
}

try {
    if (-not (Test-Path -LiteralPath $targetPath -PathType Leaf)) {
        throw 'Для настоящего текстового окна нужен Windows PowerShell с поддержкой WPF.'
    }
    Write-Library -Text ("A B" + [char]10 + 'C') -DemoHotkey:$DemoHotkey
    Start-App
    if ($DemoHotkey) {
        $mainRoot = [System.Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
        $toggleButton = Wait-UiElement -Root $mainRoot -NameFragment 'Включить печать' -ControlType ([System.Windows.Automation.ControlType]::Button)
        $toggleButton.SetFocus()
        Click-UiElement -Element $toggleButton
        Start-Sleep -Milliseconds 250
        Start-TextTarget -OutputPath $firstOutput
        Send-Combination -Modifiers ([byte[]](0x12)) -Key 0x31
        Wait-UiText -Root $mainRoot -Text 'Задание №1 включено. Печать продолжается.' | Out-Null
        foreach ($index in 1..5) {
            Send-RandomKey
            Start-Sleep -Milliseconds 120
        }
        try {
            Wait-Until -TimeoutSeconds 8 -FailureMessage 'Alt+1 не напечатал задание в активном внешнем поле.' -Condition {
                (Get-LibraryText -Path $firstOutput) -ceq ("A B" + [char]10 + 'C')
            } | Out-Null
        } catch {
            $actual = Get-LibraryText -Path $firstOutput
            $saved = Get-Data | ConvertTo-Json -Depth 10 -Compress
            throw "Демо E2E Alt+1 и фонового набора: $($_.Exception.Message) Получено [$actual]; состояние [$saved]."
        }
        Write-Output 'PASS: Alt+1 switched from task 2 to task 1 without stopping typing; spaces and Enter were typed into the active external window.'
        return
    }
    $mainRoot = [System.Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
    $toggleButton = Wait-UiElement -Root $mainRoot -NameFragment 'Включить печать' -ControlType ([System.Windows.Automation.ControlType]::Button)
    $toggleButton.SetFocus()
    Click-UiElement -Element $toggleButton
    Start-Sleep -Milliseconds 250
    Start-TextTarget -OutputPath $firstOutput
    Start-Sleep -Milliseconds 400
    foreach ($index in 1..5) {
        Send-RandomKey
        Start-Sleep -Milliseconds 120
    }
    try {
        Wait-Until -TimeoutSeconds 8 -FailureMessage 'Случайные клавиши не ввели ожидаемые пробел и Enter.' -Condition {
            (Get-LibraryText -Path $firstOutput) -ceq ("A B" + [char]10 + 'C')
        } | Out-Null
    } catch {
        $actual = Get-LibraryText -Path $firstOutput
        $saved = Get-Data | ConvertTo-Json -Depth 10 -Compress
        throw "Настольный E2E ввода: $($_.Exception.Message) Получено [$actual]; состояние [$saved]."
    }
    Write-Output 'PASS: пять случайных клавиш напечатали точный текст с пробелом и настоящим Enter.'

    Stop-ProcessTree -Process $targetProcess
    $script:targetProcess = $null
    $root = [System.Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
    [RustDesktopE2E]::SetForegroundWindow($appProcess.MainWindowHandle) | Out-Null
    $settingsButton = Wait-UiElement -Root $root -NameFragment 'Горячие клавиши' -ControlType ([System.Windows.Automation.ControlType]::Button)
    $settingsButton.SetFocus()
    Click-UiElement -Element $settingsButton
    $assignButton = Wait-UiElement -Root $root -NameFragment 'Изменить' -ControlType ([System.Windows.Automation.ControlType]::Button)
    $assignButton.SetFocus()
    Click-UiElement -Element $assignButton
    Start-Sleep -Milliseconds 300
    Send-Combination -Modifiers ([byte[]](0x11, 0x10)) -Key 0x72
    Wait-UiText -Root $root -Text 'Ctrl+Shift+F3' | Out-Null
    $saveButton = Wait-UiElement -Root $root -NameFragment 'Сохранить сочетания' -ControlType ([System.Windows.Automation.ControlType]::Button)
    $saveButton.SetFocus()
    Click-UiElement -Element $saveButton
    try {
        Wait-Until -TimeoutSeconds 8 -FailureMessage 'Новое сочетание не сохранилось в библиотеке.' -Condition {
            $saved = Get-Data
            $saved -and $saved.hotkeys.toggle_typing -ceq 'Ctrl+Shift+F3'
        } | Out-Null
    } catch {
        $saved = Get-Data | ConvertTo-Json -Depth 10 -Compress
        $all = $root.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
        $status = @($all | ForEach-Object { $_.Current.Name } | Where-Object { $_ }) -join ' | '
        throw "Настройка видна, но не сохранилась: $($_.Exception.Message) библиотека [$saved]; интерфейс [$status]."
    }
    Write-Output 'PASS: пользователь изменил глобальную горячую клавишу через окно настроек; новая привязка сохранилась.'

    Start-RoomServer
    Stop-ProcessTree -Process $appProcess
    $script:appProcess = $null
    $libraryPath = [System.IO.Path]::Combine($dataFolder, 'library.json')
    $resetLibrary = Get-Data
    $resetLibrary.snippets[0].cursor = 0
    $resetLibrary.room.serverUrl = $roomBaseUrl
    $resetLibrary.room.roomCode = $roomCode
    $resetLibrary.room.participantName = 'E2E участник'
    $resetLibrary.room.participantId = $null
    $resetJson = ConvertTo-Json -InputObject $resetLibrary -Depth 20
    [System.IO.File]::WriteAllText(
        $libraryPath,
        $resetJson,
        [System.Text.UTF8Encoding]::new($false))
    Start-App
    $restartRoot = [System.Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
    $null = Wait-UiElement -Root $restartRoot -NameFragment 'Горячие клавиши' -ControlType ([System.Windows.Automation.ControlType]::Button)
    Start-TextTarget -OutputPath $secondOutput
    Send-Combination -Modifiers ([byte[]](0x11, 0x10)) -Key 0x72
    Start-Sleep -Milliseconds 500
    Wait-UiText -Root $restartRoot -Text 'Печать включена.' | Out-Null
    $script:targetEdit.SetFocus()
    Start-Sleep -Milliseconds 150
    foreach ($index in 1..5) {
        Send-RandomKey
        Start-Sleep -Milliseconds 120
    }
    try {
        Wait-Until -TimeoutSeconds 8 -FailureMessage 'Сохранённая новая клавиша не сработала после перезапуска.' -Condition {
            (Get-LibraryText -Path $secondOutput) -ceq ("A B" + [char]10 + 'C')
        } | Out-Null
    } catch {
        $actual = Get-LibraryText -Path $secondOutput
        $saved = Get-Data | ConvertTo-Json -Depth 10 -Compress
        $mainRoot = [System.Windows.Automation.AutomationElement]::FromHandle($appProcess.MainWindowHandle)
        $all = $mainRoot.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition)
        $status = @($all | ForEach-Object { $_.Current.Name } | Where-Object { $_ }) -join ' | '
        throw "Перезапуск с новой комбинацией: $($_.Exception.Message) Получено [$actual]; библиотека [$saved]; интерфейс [$status]."
    }
    Write-Output 'PASS: Ctrl+Shift+F3 по-прежнему включает печать после полного перезапуска приложения.'

    Stop-ProcessTree -Process $targetProcess
    $script:targetProcess = $null
    $restartButton = Wait-UiElement -Root $restartRoot -NameFragment 'Сначала' -ControlType ([System.Windows.Automation.ControlType]::Button)
    Click-UiElement -Element $restartButton
    Wait-Until -TimeoutSeconds 5 -FailureMessage 'Локальный курсор не сбросился перед проверкой комнаты.' -Condition {
        $saved = Get-Data
        $saved -and $saved.snippets[0].cursor -eq 0
    } | Out-Null

    $roomButton = Wait-UiElement -Root $restartRoot -NameFragment 'Комната' -ControlType ([System.Windows.Automation.ControlType]::Button)
    $roomButton.SetFocus()
    Click-UiElement -Element $roomButton
    $connectButton = Wait-UiElement -Root $restartRoot -NameFragment 'Подключиться' -ControlType ([System.Windows.Automation.ControlType]::Button)
    Set-RoomConnectionFields -Root $restartRoot -ServerUrl $roomBaseUrl -RoomCode $roomCode -ParticipantName 'E2E участник'
    Click-UiElement -Element $connectButton
    Wait-UiText -Root $restartRoot -Text 'Подключено к комнате.' | Out-Null

    $participantId = Wait-Until -TimeoutSeconds 8 -FailureMessage 'Клиент не сохранил ID участника комнаты.' -Condition {
        $saved = Get-Data
        if ($saved -and $saved.room.participantId) { $saved.room.participantId }
    }
    $roomState = Invoke-RestMethod -Uri "$roomBaseUrl/api/room?roomCode=$roomCode" -TimeoutSec 5
    $serverParticipant = @($roomState.participants | Where-Object { $_.id -eq $participantId }) | Select-Object -First 1
    if (-not $serverParticipant -or -not $serverParticipant.isOnline) {
        throw 'Настольный клиент подключился, но сервер не видит его активным участником.'
    }
    Write-Output 'PASS: настольный клиент присоединился к реальному HTTP-серверу комнаты.'

    Start-TextTarget -OutputPath $roomOutput
    Send-Combination -Modifiers ([byte[]](0x11, 0x10)) -Key 0x72
    Wait-UiText -Root $restartRoot -Text 'Печать включена.' | Out-Null
    $roomText = "Сәлем, Саят!`n12-есеп: Console.WriteLine(12); 🌟"
    $quickTask = Send-RoomQuickText -ParticipantId $participantId -Text $roomText
    if ($quickTask.code -cne $roomText -or -not $quickTask.isQuickSend) {
        throw 'Сервер комнаты не сохранил точный быстрый многострочный текст.'
    }
    Wait-UiText -Root $restartRoot -Text 'Задание запущено. Через 3 секунды начнётся автоматическая печать.' | Out-Null

    $remoteSnippet = Wait-Until -TimeoutSeconds 8 -FailureMessage 'Клиент не сохранил полученное назначение.' -Condition {
        $saved = Get-Data
        $saved.snippets | Where-Object { $_.is_remote_assignment -and $_.code -ceq $roomText }
    }
    $saved = Get-Data
    if (-not $remoteSnippet.pinned -or $saved.selected_id -ne $remoteSnippet.id) {
        throw 'Удалённое задание не выбрано и не защищено закреплением.'
    }
    $roomTextElements = [System.Globalization.StringInfo]::new($roomText).LengthInTextElements
    for ($index = 1; $index -le $roomTextElements; $index++) {
        Send-RandomKey
        Start-Sleep -Milliseconds 80
    }
    Wait-Until -TimeoutSeconds 10 -FailureMessage 'Клиент комнаты не напечатал полученный текст.' -Condition {
        (Get-LibraryText -Path $roomOutput) -ceq $roomText
    } | Out-Null
    Write-Output 'PASS: быстрый текст дошёл до клиента, закрепился и напечатался с Enter и Unicode.'

    $null = Send-RoomCommand -ParticipantId $participantId -Command 'restart'
    Wait-UiText -Root $restartRoot -Text 'Команда комнаты: позиция сброшена.' | Out-Null
    Wait-Until -TimeoutSeconds 5 -FailureMessage 'Команда перезапуска не сбросила курсор назначенного текста.' -Condition {
        $saved = Get-Data
        $saved.snippets | Where-Object { $_.is_remote_assignment -and $_.cursor -eq 0 }
    } | Out-Null
    Write-Output 'PASS: команда комнаты перезапуска обработана настоящим настольным клиентом.'
}
finally {
    Stop-ProcessTree -Process $targetProcess
    Stop-ProcessTree -Process $appProcess
    Stop-RoomServer
    $resolvedRoot = [System.IO.Path]::GetFullPath($rootFolder)
    $resolvedTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
    if ($resolvedRoot.StartsWith($resolvedTemp, [System.StringComparison]::OrdinalIgnoreCase)) {
        Remove-Item -LiteralPath $resolvedRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
