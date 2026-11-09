param(
    [string]$TlaToolsJar = (Join-Path $PSScriptRoot 'tla2tools.jar'),
    [string]$JavaExe = 'java'
)

$ErrorActionPreference = 'Stop'
$jar = (Resolve-Path -LiteralPath $TlaToolsJar).Path
$specDirectory = $PSScriptRoot
$spec = Join-Path $specDirectory 'TypingCore.tla'
$config = Join-Path $specDirectory 'TypingCore.cfg'
$expectedSha1 = 'bee4a54f3ee3d4afc347c3240ec2d9e93b075104'
$actualSha1 = (Get-FileHash -LiteralPath $jar -Algorithm SHA1).Hash.ToLowerInvariant()
if ($actualSha1 -ne $expectedSha1) {
    throw "Не совпала контрольная сумма tla2tools.jar: $actualSha1"
}

if (-not (Test-Path -LiteralPath $JavaExe -PathType Leaf)) {
    $javaCommand = Get-Command $JavaExe -ErrorAction SilentlyContinue
    if (-not $javaCommand) {
        throw 'Не найден Java 11 или новее. Установите JDK и укажите путь через -JavaExe.'
    }
    $JavaExe = $javaCommand.Source
}

& $JavaExe -XX:+UseParallelGC -cp $jar tlc2.TLC -config $config -workers 1 $spec
if ($LASTEXITCODE -ne 0) {
    throw "TLC завершился с кодом $LASTEXITCODE."
}
