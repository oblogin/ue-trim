#Requires -Version 5.1
<#
.SYNOPSIS
    Утилита для сокращения размера билда Unreal Engine.

.PARAMETER UERoot
    Путь к корню Unreal Engine (например C:\UEACO)

.PARAMETER Platforms
    Платформы для СОХРАНЕНИЯ через запятую.
    Доступные: Windows, Linux, Android, IOS, Mac, TVOS, VisionOS, HoloLens, SteamDeck

.PARAMETER Execute
    Реально удалить файлы. Без этого флага — только предпросмотр (dry-run).

.PARAMETER KeepGit
    НЕ удалять .git/ (по умолчанию удаляется).

.PARAMETER KeepIntermediate
    НЕ удалять Engine/Intermediate/ (по умолчанию удаляется).

.PARAMETER RemoveTests
    Удалить тестовые бинарники из Engine/Binaries/Win64/ (~5 ГБ).

.PARAMETER StripPdb
    Удалить .pdb debug-символы из Engine/Binaries/ (~5.5 ГБ).
    Редактор работает без них, но отладка крашей станет сложнее.

.EXAMPLE
    .\ue-trim.ps1 C:\UEACO -Platforms "Windows,Linux,Android,IOS"

.EXAMPLE
    .\ue-trim.ps1 C:\UEACO -Platforms "Windows,Linux" -Execute -RemoveTests -StripPdb

.EXAMPLE
    .\ue-trim.ps1 C:\UEACO -Platforms "Windows,IOS" -KeepGit -RemoveTests
#>

param(
    [Parameter(Mandatory=$true, Position=0)]
    [string]$UERoot,

    [Parameter(Mandatory=$true)]
    [string]$Platforms,

    [switch]$Execute,
    [switch]$KeepGit,
    [switch]$KeepIntermediate,
    [switch]$RemoveTests,
    [switch]$StripPdb
)

$ErrorActionPreference = "Continue"

# ══════════════════════════════════════════════════════════════════════════
# База знаний о платформах UE
# ══════════════════════════════════════════════════════════════════════════

$AllPlatforms = @{
    "Windows" = @{
        FolderNames   = @("Win64", "Win32", "Windows")
        BinariesDir   = "Win64"
        PlatformsDir  = "Windows"
        BuildDir      = "Windows"
        ConfigDir     = $null
        ExtrasDirs    = @("Windows")
        SourceDirs    = @("Engine\Source\Developer\Windows")
        RequiresApple = $false
    }
    "Linux" = @{
        FolderNames   = @("Linux", "LinuxArm64")
        BinariesDir   = "Linux"
        PlatformsDir  = $null
        BuildDir      = "Linux"
        ConfigDir     = $null
        ExtrasDirs    = @("GDBPrinters")
        SourceDirs    = @("Engine\Source\Developer\Linux", "Engine\Source\Programs\AutomationTool\Linux")
        RequiresApple = $false
    }
    "Android" = @{
        FolderNames   = @("Android")
        BinariesDir   = $null
        PlatformsDir  = "Android"
        BuildDir      = "Android"
        ConfigDir     = "Android"
        ExtrasDirs    = @("Android")
        SourceDirs    = @("Engine\Source\Developer\Android", "Engine\Source\Programs\AutomationTool\Android")
        RequiresApple = $false
    }
    "IOS" = @{
        FolderNames   = @("IOS")
        BinariesDir   = $null
        PlatformsDir  = "IOS"
        BuildDir      = "IOS"
        ConfigDir     = $null
        ExtrasDirs    = @("iTunes")
        SourceDirs    = @("Engine\Source\Developer\IOS", "Engine\Source\Programs\AutomationTool\IOS")
        RequiresApple = $true
    }
    "Mac" = @{
        FolderNames   = @("Mac", "macos", "osx", "Darwin")
        BinariesDir   = "Mac"
        PlatformsDir  = $null
        BuildDir      = "Mac"
        ConfigDir     = $null
        ExtrasDirs    = @("LLDBDataFormatters", "Instruments")
        SourceDirs    = @("Engine\Source\Developer\Mac", "Engine\Source\Programs\AutomationTool\Mac")
        RequiresApple = $true
    }
    "TVOS" = @{
        FolderNames   = @("TVOS")
        BinariesDir   = $null
        PlatformsDir  = $null
        BuildDir      = "TVOS"
        ConfigDir     = "TVOS"
        ExtrasDirs    = @()
        SourceDirs    = @("Engine\Source\Programs\AutomationTool\TVOS")
        RequiresApple = $true
    }
    "VisionOS" = @{
        FolderNames   = @("VisionOS")
        BinariesDir   = $null
        PlatformsDir  = "VisionOS"
        BuildDir      = $null
        ConfigDir     = $null
        ExtrasDirs    = @()
        SourceDirs    = @()
        RequiresApple = $true
    }
    "HoloLens" = @{
        FolderNames   = @("HoloLens")
        BinariesDir   = $null
        PlatformsDir  = $null
        BuildDir      = $null
        ConfigDir     = $null
        ExtrasDirs    = @()
        SourceDirs    = @()
        RequiresApple = $false
    }
    "SteamDeck" = @{
        FolderNames   = @("SteamDeck")
        BinariesDir   = $null
        PlatformsDir  = $null
        BuildDir      = "SteamDeck"
        ConfigDir     = $null
        ExtrasDirs    = @()
        SourceDirs    = @()
        RequiresApple = $false
    }
}

$AlwaysRemoveDirs = @(
    "Engine\DerivedDataCache"
    "Templates"
    "Samples"
    "FeaturePacks"
    "Engine\Documentation"
    "Engine\Extras\Horde"
    "Engine\Extras\P4VUtils"
    "Engine\Extras\Maya_AnimationRiggingTools"
    "Engine\Extras\MayaVelocityGridExporter"
    "Engine\Extras\UnrealEngineLauncher"
    "Engine\Extras\ThirdPartyNotUE"
    "Engine\Extras\VirtualProduction"
    "Engine\Extras\RoboMerge"
    "Engine\Extras\Flutter"
    "Engine\Extras\3dsMaxScripts"
    ".idea"
)

$AlwaysRemoveFiles = @(
    ".tgitconfig"
    "PULL_REQUEST_TEMPLATE.md"
    "UE5.sln"
)

$AppleSharedDirs = @(
    "Engine\Build\Xcode"
    "Engine\Extras\Xcode"
)

$ThirdPartyRoots = @(
    "Engine\Binaries\ThirdParty"
    "Engine\Source\ThirdParty"
)

# ══════════════════════════════════════════════════════════════════════════
# Функции
# ══════════════════════════════════════════════════════════════════════════

function Format-Size([long]$Bytes) {
    if ($Bytes -ge 1GB) { return "{0:N2} ГБ" -f ($Bytes / 1GB) }
    if ($Bytes -ge 1MB) { return "{0:N1} МБ" -f ($Bytes / 1MB) }
    if ($Bytes -ge 1KB) { return "{0:N0} КБ" -f ($Bytes / 1KB) }
    return "$Bytes Б"
}

function Get-DirSize([string]$Path) {
    try {
        $size = (Get-ChildItem -Path $Path -Recurse -Force -File -ErrorAction SilentlyContinue |
                 Measure-Object -Property Length -Sum -ErrorAction SilentlyContinue).Sum
        if ($null -eq $size) { return 0 }
        return [long]$size
    } catch {
        return 0
    }
}

function Find-PlatformDirs([string]$Root, [string[]]$Names, [int]$MaxDepth = 5) {
    $results = @()
    if (-not (Test-Path $Root)) { return $results }
    Get-ChildItem -Path $Root -Directory -Recurse -Depth $MaxDepth -Force -ErrorAction SilentlyContinue |
        Where-Object { $Names -contains $_.Name } |
        ForEach-Object { $results += $_.FullName }
    return $results
}

function Remove-Target([string]$FullPath, [string]$RelPath, [bool]$IsFile = $false) {
    if ($IsFile) {
        $size = (Get-Item $FullPath -Force).Length
        $script:totalFilesFound++
    } else {
        $size = Get-DirSize $FullPath
        $script:totalDirsFound++
    }
    $script:totalBytesFreed += $size

    if ($DryRun) {
        Write-Host "  [УДАЛИТЬ] $RelPath ($(Format-Size $size))" -ForegroundColor DarkYellow
    } else {
        Write-Host "  Удаляю $RelPath ($(Format-Size $size))... " -NoNewline
        try {
            Remove-Item -Path $FullPath -Recurse:$(-not $IsFile) -Force -ErrorAction Stop
            Write-Host "OK" -ForegroundColor Green
            if ($IsFile) { $script:totalFilesRemoved++ } else { $script:totalDirsRemoved++ }
        } catch {
            Write-Host "ОШИБКА: $_" -ForegroundColor Red
            $script:errors += "${RelPath}: $_"
        }
    }
}

# ══════════════════════════════════════════════════════════════════════════
# Валидация
# ══════════════════════════════════════════════════════════════════════════

$UERoot = (Resolve-Path $UERoot -ErrorAction SilentlyContinue).Path
if (-not $UERoot) { Write-Error "Путь не найден"; exit 1 }

$markers = @("Engine\Source", "Engine\Binaries", "Engine\Build", "GenerateProjectFiles.bat")
foreach ($m in $markers) {
    if (-not (Test-Path (Join-Path $UERoot $m))) {
        Write-Error "'$UERoot' не похоже на корень Unreal Engine."
        exit 1
    }
}

$KeepPlatforms = $Platforms -split ',' | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' }
$KeepNormalized = @()
foreach ($name in $KeepPlatforms) {
    $found = $AllPlatforms.Keys | Where-Object { $_ -ieq $name }
    if ($found) { $KeepNormalized += $found }
    else { Write-Error "Неизвестная платформа '$name'. Доступные: $($AllPlatforms.Keys -join ', ')"; exit 1 }
}

$RemovePlatforms = $AllPlatforms.Keys | Where-Object { $_ -notin $KeepNormalized }
$AnyAppleKept = $false
foreach ($k in $KeepNormalized) { if ($AllPlatforms[$k].RequiresApple) { $AnyAppleKept = $true; break } }

$FolderNamesToRemove = @()
foreach ($pName in $RemovePlatforms) {
    foreach ($fn in $AllPlatforms[$pName].FolderNames) {
        if ($FolderNamesToRemove -notcontains $fn) { $FolderNamesToRemove += $fn }
    }
}

# ══════════════════════════════════════════════════════════════════════════
# Запуск
# ══════════════════════════════════════════════════════════════════════════

$DryRun = -not $Execute

Write-Host ""
if ($DryRun) {
    Write-Host "  ╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
    Write-Host "  ║  ue-trim: РЕЖИМ ПРЕДПРОСМОТРА (dry-run)                    ║" -ForegroundColor Cyan
    Write-Host "  ╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
} else {
    Write-Host "  ╔══════════════════════════════════════════════════════════════╗" -ForegroundColor Red
    Write-Host "  ║  ue-trim: РЕЖИМ УДАЛЕНИЯ                                   ║" -ForegroundColor Red
    Write-Host "  ║  ВНИМАНИЕ: Файлы будут удалены безвозвратно!                ║" -ForegroundColor Red
    Write-Host "  ╚══════════════════════════════════════════════════════════════╝" -ForegroundColor Red
}

Write-Host "  Сохраняем:       $($KeepNormalized -join ', ')" -ForegroundColor Green
Write-Host "  Удаляем:         $($RemovePlatforms -join ', ')" -ForegroundColor Yellow

if ($KeepGit) { Write-Host "  .git:            СОХРАНЯЕМ" -ForegroundColor Green }
else          { Write-Host "  .git:            УДАЛЯЕМ" -ForegroundColor Yellow }

if ($KeepIntermediate) { Write-Host "  Intermediate:    СОХРАНЯЕМ" -ForegroundColor Green }
else                   { Write-Host "  Intermediate:    УДАЛЯЕМ" -ForegroundColor Yellow }

if ($RemoveTests) { Write-Host "  Тесты (Win64):   УДАЛЯЕМ (~5 ГБ)" -ForegroundColor Yellow }
else              { Write-Host "  Тесты (Win64):   сохраняем (-RemoveTests для удаления)" -ForegroundColor Gray }

if ($StripPdb) { Write-Host "  PDB символы:     УДАЛЯЕМ (~5.5 ГБ)" -ForegroundColor Yellow }
else           { Write-Host "  PDB символы:     сохраняем (-StripPdb для удаления)" -ForegroundColor Gray }

if ($AnyAppleKept) {
    $appleKept = $KeepNormalized | Where-Object { $AllPlatforms[$_].RequiresApple }
    Write-Host "  Apple shared:    СОХРАНЯЕМ (нужны для $($appleKept -join ', '))" -ForegroundColor Green
} else {
    Write-Host "  Apple shared:    УДАЛЯЕМ" -ForegroundColor Yellow
}

if (-not $DryRun) {
    $confirm = Read-Host "`nПродолжить? (y/N)"
    if ($confirm -ne "y") { Write-Host "Отменено."; exit 0 }
}

$totalDirsFound = 0
$totalFilesFound = 0
$totalDirsRemoved = 0
$totalFilesRemoved = 0
$totalBytesFreed = [long]0
$errors = @()

# ── Шаг 1: Общая очистка ─────────────────────────────────────────────────

Write-Host "`n── Шаг 1: Общая очистка ─────────────────────────────────" -ForegroundColor Yellow

if (-not $KeepGit) {
    $p = Join-Path $UERoot ".git"
    if (Test-Path $p -PathType Container) { Remove-Target $p ".git" }
}
if (-not $KeepIntermediate) {
    $p = Join-Path $UERoot "Engine\Intermediate"
    if (Test-Path $p -PathType Container) { Remove-Target $p "Engine\Intermediate" }
}
foreach ($rel in $AlwaysRemoveDirs) {
    $p = Join-Path $UERoot $rel
    if (Test-Path $p -PathType Container) { Remove-Target $p $rel }
}
foreach ($rel in $AlwaysRemoveFiles) {
    $p = Join-Path $UERoot $rel
    if (Test-Path $p -PathType Leaf) { Remove-Target $p $rel -IsFile $true }
}

# ── Шаг 2: Платформенные папки ────────────────────────────────────────────

Write-Host "`n── Шаг 2: Платформенные папки ────────────────────────────" -ForegroundColor Yellow

foreach ($pName in $RemovePlatforms) {
    $plat = $AllPlatforms[$pName]

    if ($plat.PlatformsDir) {
        $p = Join-Path $UERoot "Engine\Platforms\$($plat.PlatformsDir)"
        if (Test-Path $p) { Remove-Target $p "Engine\Platforms\$($plat.PlatformsDir)" }
    }
    if ($plat.BinariesDir) {
        $p = Join-Path $UERoot "Engine\Binaries\$($plat.BinariesDir)"
        if (Test-Path $p) { Remove-Target $p "Engine\Binaries\$($plat.BinariesDir)" }
    }
    if ($plat.BuildDir) {
        $p = Join-Path $UERoot "Engine\Build\$($plat.BuildDir)"
        if (Test-Path $p) { Remove-Target $p "Engine\Build\$($plat.BuildDir)" }
    }
    if ($plat.ConfigDir) {
        $p = Join-Path $UERoot "Engine\Config\$($plat.ConfigDir)"
        if (Test-Path $p) { Remove-Target $p "Engine\Config\$($plat.ConfigDir)" }
    }
    foreach ($dir in $plat.ExtrasDirs) {
        $p = Join-Path $UERoot "Engine\Extras\$dir"
        if (Test-Path $p) { Remove-Target $p "Engine\Extras\$dir" }
    }
    foreach ($dir in $plat.SourceDirs) {
        $p = Join-Path $UERoot $dir
        if (Test-Path $p) { Remove-Target $p $dir }
    }
}

if (-not $AnyAppleKept) {
    Write-Host "`n  Apple shared:" -ForegroundColor Gray
    foreach ($rel in $AppleSharedDirs) {
        $p = Join-Path $UERoot $rel
        if (Test-Path $p) { Remove-Target $p $rel }
    }
}

# ── Шаг 3: ThirdParty ────────────────────────────────────────────────────

if ($FolderNamesToRemove.Count -gt 0) {
    Write-Host "`n── Шаг 3: ThirdParty ($($FolderNamesToRemove -join ', ')) ──" -ForegroundColor Yellow

    foreach ($rootRel in $ThirdPartyRoots) {
        $rootPath = Join-Path $UERoot $rootRel
        if (-not (Test-Path $rootPath)) { continue }

        Write-Host "`n  Сканирую $rootRel..." -ForegroundColor Gray
        $platformDirs = Find-PlatformDirs $rootPath $FolderNamesToRemove 5

        if ($platformDirs.Count -eq 0) {
            Write-Host "  (не найдено)" -ForegroundColor Gray
            continue
        }
        foreach ($pd in $platformDirs) {
            $relPath = $pd.Substring($UERoot.Length + 1)
            Remove-Target $pd $relPath
        }
    }
}

# ── Шаг 4: Тестовые бинарники ────────────────────────────────────────────

if ($RemoveTests) {
    Write-Host "`n── Шаг 4: Тестовые бинарники (Win64/*Tests) ─────────────" -ForegroundColor Yellow

    $win64 = Join-Path $UERoot "Engine\Binaries\Win64"
    if (Test-Path $win64) {
        Get-ChildItem -Path $win64 -Directory -Force -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match 'Tests$' } |
            ForEach-Object {
                $relPath = "Engine\Binaries\Win64\$($_.Name)"
                Remove-Target $_.FullName $relPath
            }

        # HeadlessChaos — тестовый harness для Chaos физики
        $hc = Join-Path $win64 "HeadlessChaos"
        if (Test-Path $hc) { Remove-Target $hc "Engine\Binaries\Win64\HeadlessChaos" }
    }
}

# ── Шаг 5: PDB файлы ─────────────────────────────────────────────────────

if ($StripPdb) {
    Write-Host "`n── Шаг 5: PDB debug-символы ─────────────────────────────" -ForegroundColor Yellow

    $binaries = Join-Path $UERoot "Engine\Binaries"
    if (Test-Path $binaries) {
        $pdbFiles = Get-ChildItem -Path $binaries -Filter "*.pdb" -Recurse -Force -File -ErrorAction SilentlyContinue
        $pdbCount = ($pdbFiles | Measure-Object).Count
        $pdbSize = ($pdbFiles | Measure-Object -Property Length -Sum).Sum

        if ($pdbCount -gt 0) {
            if ($DryRun) {
                Write-Host "  [УДАЛИТЬ] $pdbCount .pdb файлов ($(Format-Size $pdbSize))" -ForegroundColor DarkYellow
                $script:totalFilesFound += $pdbCount
                $script:totalBytesFreed += $pdbSize
            } else {
                Write-Host "  Удаляю $pdbCount .pdb файлов ($(Format-Size $pdbSize))... " -NoNewline
                $removed = 0
                foreach ($pdb in $pdbFiles) {
                    try {
                        Remove-Item -Path $pdb.FullName -Force -ErrorAction Stop
                        $removed++
                    } catch {
                        $script:errors += "$($pdb.Name): $_"
                    }
                }
                Write-Host "OK ($removed/$pdbCount)" -ForegroundColor Green
                $script:totalFilesRemoved += $removed
                $script:totalBytesFreed += $pdbSize
            }
        } else {
            Write-Host "  (PDB файлов не найдено)" -ForegroundColor Gray
        }
    }
}

# ── Итоги ─────────────────────────────────────────────────────────────────

Write-Host "`n══════════════════════════════════════════════════════════" -ForegroundColor White

if ($DryRun) {
    Write-Host "ИТОГО (dry-run, ничего не удалено):" -ForegroundColor Cyan
    Write-Host "  Папок к удалению:  $totalDirsFound"
    Write-Host "  Файлов к удалению: $totalFilesFound"
    Write-Host "  Освободится:       $(Format-Size $totalBytesFreed)" -ForegroundColor Green
    Write-Host "`nДля реального удаления добавьте -Execute" -ForegroundColor Cyan
} else {
    Write-Host "ИТОГО:" -ForegroundColor Green
    Write-Host "  Папок удалено:     $totalDirsRemoved / $totalDirsFound"
    Write-Host "  Файлов удалено:    $totalFilesRemoved / $totalFilesFound"
    Write-Host "  Освобождено:       $(Format-Size $totalBytesFreed)" -ForegroundColor Green

    if ($errors.Count -gt 0) {
        Write-Host "`nОШИБКИ ($($errors.Count)):" -ForegroundColor Red
        foreach ($e in $errors) { Write-Host "  - $e" -ForegroundColor Red }
    }

    Write-Host "`nРекомендации:" -ForegroundColor Yellow
    Write-Host "  1. Запустите GenerateProjectFiles.bat"
    Write-Host "  2. Выполните полную пересборку движка"
}
