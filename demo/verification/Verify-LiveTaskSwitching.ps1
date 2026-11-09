param(
    [Parameter(Mandatory = $true)]
    [string]$AppPath
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class LiveSwitchNative
{
    [StructLayout(LayoutKind.Sequential)]
    private struct Input
    {
        public uint Type;
        public InputUnion Data;
    }

    [StructLayout(LayoutKind.Explicit, Size = 32)]
    private struct InputUnion
    {
        [FieldOffset(0)] public KeyboardInput Keyboard;
        [FieldOffset(0)] public MouseInput Mouse;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct KeyboardInput
    {
        public ushort VirtualKey;
        public ushort ScanCode;
        public uint Flags;
        public uint Time;
        public UIntPtr ExtraInfo;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MouseInput
    {
        public int X;
        public int Y;
        public uint MouseData;
        public uint Flags;
        public uint Time;
        public UIntPtr ExtraInfo;
    }

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr window);

    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr window, int command);

    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll")]
    public static extern IntPtr SetFocus(IntPtr window);

    [DllImport("user32.dll")]
    public static extern IntPtr GetFocus();

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint SendInput(uint inputCount, Input[] inputs, int inputSize);

    private static void Send(params Input[] inputs)
    {
        uint sent = SendInput((uint)inputs.Length, inputs, Marshal.SizeOf<Input>());
        if (sent != inputs.Length)
            throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
    }

    private static Input Key(byte virtualKey, uint flags)
    {
        return new Input
        {
            Type = 1,
            Data = new InputUnion
            {
                Keyboard = new KeyboardInput
                {
                    VirtualKey = virtualKey,
                    Flags = flags,
                    ExtraInfo = UIntPtr.Zero
                }
            }
        };
    }

    public static void Press(byte virtualKey)
    {
        Send(Key(virtualKey, 0), Key(virtualKey, 2));
    }

    public static void AltNumber(byte virtualKey)
    {
        Send(Key(0x12, 0), Key(virtualKey, 0), Key(virtualKey, 2), Key(0x12, 2));
    }

    public static void MouseXButton(uint button)
    {
        Input down = new Input
        {
            Type = 0,
            Data = new InputUnion
            {
                Mouse = new MouseInput { MouseData = button, Flags = 0x0080 }
            }
        };
        Input up = new Input
        {
            Type = 0,
            Data = new InputUnion
            {
                Mouse = new MouseInput { MouseData = button, Flags = 0x0100 }
            }
        };
        Send(down, up);
    }

    public static void ForceForeground(IntPtr window)
    {
        Send(Key(0x12, 0));
        SetForegroundWindow(window);
        Send(Key(0x12, 2));
    }
}
'@

$resolvedApp = (Resolve-Path -LiteralPath $AppPath).Path
$verificationRoot = Join-Path ([System.IO.Path]::GetTempPath()) "LiveCodeTyperLiveSwitch_$([guid]::NewGuid().ToString('N'))"
$dataFolder = Join-Path $verificationRoot 'portable-data'
$app = $null
$form = $null
$timer = $null

try {
    New-Item -ItemType Directory -Path $dataFolder -Force | Out-Null
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $resolvedApp
    $start.WorkingDirectory = [System.IO.Path]::GetDirectoryName($resolvedApp)
    $start.UseShellExecute = $false
    $start.EnvironmentVariables['LIVECODETYPER_DATA_FOLDER'] = $dataFolder
    $app = [System.Diagnostics.Process]::Start($start)
    if (-not $app) { throw 'The demo application did not start.' }

    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    while ([DateTime]::UtcNow -lt $deadline -and $app.MainWindowHandle -eq [IntPtr]::Zero) {
        Start-Sleep -Milliseconds 100
        $app.Refresh()
    }
    if ($app.MainWindowHandle -eq [IntPtr]::Zero) {
        throw 'The demo application did not create its window in time.'
    }

    [LiveSwitchNative]::ShowWindow($app.MainWindowHandle, 9) | Out-Null
    [LiveSwitchNative]::SetForegroundWindow($app.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 2000

    $form = [System.Windows.Forms.Form]::new()
    $form.Text = 'LiveCode Typer controlled target'
    $form.ShowInTaskbar = $false
    $form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedToolWindow
    $form.StartPosition = [System.Windows.Forms.FormStartPosition]::Manual
    $form.Location = [System.Drawing.Point]::new(20, 20)
    $form.Size = [System.Drawing.Size]::new(500, 240)
    $form.Opacity = 1.0
    $form.TopMost = $true

    $editor = [System.Windows.Forms.TextBox]::new()
    $editor.Multiline = $true
    $editor.Dock = [System.Windows.Forms.DockStyle]::Fill
    $form.Controls.Add($editor)

    $state = [pscustomobject]@{ Stage = 0; FocusAttempts = 0; Error = $null }
    $timer = [System.Windows.Forms.Timer]::new()
    $timer.Interval = 450
    $timer.Add_Tick({
        try {
            if ($state.Stage -ge 1 -and $state.Stage -le 9 -and
                [LiveSwitchNative]::GetForegroundWindow() -ne $form.Handle) {
                [LiveSwitchNative]::ForceForeground($form.Handle)
                $editor.Focus() | Out-Null
                [LiveSwitchNative]::SetFocus($editor.Handle) | Out-Null
                $state.FocusAttempts++
                if ($state.FocusAttempts -gt 12) {
                    $state.Error = 'Windows did not give focus to the controlled editor after repeated attempts.'
                    $timer.Stop()
                    $form.Close()
                }
                return
            }
            $state.FocusAttempts = 0
            switch ($state.Stage) {
                0 {
                    $form.Activate()
                    [LiveSwitchNative]::ForceForeground($form.Handle)
                    $editor.Focus() | Out-Null
                    [LiveSwitchNative]::SetFocus($editor.Handle) | Out-Null
                }
                1 {
                    [LiveSwitchNative]::Press(0x5A)
                }
                2 {
                    if ([string]::IsNullOrEmpty($editor.Text)) {
                        $state.Error = "The controlled editor did not receive its preflight input. Foreground window was $([LiveSwitchNative]::GetForegroundWindow()), form handle was $($form.Handle), focus handle was $([LiveSwitchNative]::GetFocus()), editor handle was $($editor.Handle)."
                        $timer.Stop()
                        $form.Close()
                        return
                    }
                    $editor.Clear()
                    [LiveSwitchNative]::AltNumber(0x33)
                }
                3 {
                    1..22 | ForEach-Object { [LiveSwitchNative]::Press(0x58) }
                }
                4 {
                    [LiveSwitchNative]::AltNumber(0x35)
                }
                5 {
                    1..22 | ForEach-Object { [LiveSwitchNative]::Press(0x59) }
                }
                6 {
                    [LiveSwitchNative]::MouseXButton(1)
                }
                7 {
                    1..22 | ForEach-Object { [LiveSwitchNative]::Press(0x5A) }
                }
                8 {
                    [LiveSwitchNative]::MouseXButton(2)
                    [LiveSwitchNative]::MouseXButton(2)
                }
                9 {
                    1..22 | ForEach-Object { [LiveSwitchNative]::Press(0x56) }
                }
                10 {
                    $timer.Stop()
                    $form.Close()
                }
            }
            $state.Stage++
        }
        catch {
            $state.Error = "Live switching E2E stage $($state.Stage) failed: $($_.Exception.Message)"
            if ($timer) {
                $timer.Stop()
            }
            $form.Close()
        }
    })
    $form.Add_Shown({
        $form.Activate()
        $editor.Focus() | Out-Null
        $timer.Start()
    })
    $form.ShowDialog() | Out-Null

    if ($state.Error) { throw $state.Error }

    $actual = $editor.Text.Replace("`r`n", "`n").Replace("`r", "`n")
    $expected = "using System;`nclass Seusing System;`nclass Fausing System;`nclass Deusing System;`nclass Pa"
    if ($actual -cne $expected) {
        throw "Background switching failed. Expected [$expected], got [$actual]."
    }

    Write-Output 'PASS: Alt+3, Alt+5, mouse button 4, and mouse button 5 switched and typed into a separate foreground editor while the LCT window stayed untouched.'
}
finally {
    if ($timer) { $timer.Stop(); $timer.Dispose() }
    if ($form) { $form.Dispose() }
    if ($app -and -not $app.HasExited) {
        $app.Kill($true)
        $null = $app.WaitForExit(5000)
    }
    if ($app) { $app.Dispose() }
    if (Test-Path -LiteralPath $verificationRoot) {
        $resolvedRoot = [System.IO.Path]::GetFullPath($verificationRoot)
        $resolvedTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath())
        if ($resolvedRoot.StartsWith($resolvedTemp, [System.StringComparison]::OrdinalIgnoreCase)) {
            Remove-Item -LiteralPath $resolvedRoot -Recurse -Force
        }
    }
}
