param(
    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName PresentationFramework

$outputFile = [System.IO.Path]::GetFullPath($OutputPath)
$window = [System.Windows.Window]::new()
$window.Title = "LiveCodeTyper Verification Target"
$window.Width = 620
$window.Height = 320
$window.WindowStartupLocation = "CenterScreen"
$window.Topmost = $true

$textBox = [System.Windows.Controls.TextBox]::new()
$textBox.AcceptsReturn = $true
$textBox.AcceptsTab = $true
$textBox.FontFamily = "Consolas"
$textBox.FontSize = 18
$textBox.VerticalScrollBarVisibility = "Auto"
$textBox.HorizontalScrollBarVisibility = "Auto"
$textBox.Padding = [System.Windows.Thickness]::new(16)

$textBox.Add_TextChanged({
    [System.IO.File]::WriteAllText(
        $outputFile,
        $textBox.Text,
        [System.Text.UTF8Encoding]::new($false))
})

$window.Content = $textBox
$window.Add_Loaded({
    $textBox.Focus() | Out-Null
    [System.Windows.Input.Keyboard]::Focus($textBox) | Out-Null
})

$window.ShowDialog() | Out-Null
