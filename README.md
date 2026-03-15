# ue-trim

Утилита для сокращения размера билда Unreal Engine. Удаляет ненужные платформы, кэши компиляции, шаблоны и git-историю.

Доступны два варианта: **Rust-бинарник** и **PowerShell-скрипт** с идентичной функциональностью.

## Быстрый старт

```powershell
# PowerShell — предпросмотр (ничего не удаляет):
.\ue-trim.ps1 C:\UE5 -Platforms "Windows,Linux,Android,IOS"

# PowerShell — реальное удаление:
.\ue-trim.ps1 C:\UE5 -Platforms "Windows,Linux,Android,IOS" -Execute
```

```bash
# Rust — предпросмотр:
ue-trim C:\UE5 --platforms Windows,Linux,Android,IOS

# Rust — реальное удаление:
ue-trim C:\UE5 --platforms Windows,Linux,Android,IOS --execute
```

## Параметры

| Параметр | Описание |
|---|---|
| `UERoot` (позиционный) | Путь к корню Unreal Engine |
| `--platforms` / `-Platforms` | Платформы для **сохранения**, через запятую |
| `--execute` / `-Execute` | Реально удалить файлы. Без этого флага — только предпросмотр (dry-run) |
| `--keep-git` / `-KeepGit` | НЕ удалять `.git/` (по умолчанию удаляется) |
| `--keep-intermediate` / `-KeepIntermediate` | НЕ удалять `Engine/Intermediate/` (по умолчанию удаляется) |
| `--remove-tests` / `-RemoveTests` | Удалить тестовые бинарники из `Engine/Binaries/Win64/` (~5 ГБ) |
| `--strip-pdb` / `-StripPdb` | Удалить `.pdb` debug-символы из `Engine/Binaries/` (~5.5 ГБ) |

## Доступные платформы

| Платформа | Папки в движке | Apple-зависимость |
|---|---|---|
| `Windows` | Win64, Win32, Windows | Нет |
| `Linux` | Linux, LinuxArm64 | Нет |
| `Android` | Android | Нет |
| `IOS` | IOS | Да |
| `Mac` | Mac, macos, osx, Darwin | Да |
| `TVOS` | TVOS | Да |
| `VisionOS` | VisionOS | Да |
| `HoloLens` | HoloLens | Нет |
| `SteamDeck` | SteamDeck | Нет |

Если хотя бы одна Apple-платформа (IOS, Mac, TVOS, VisionOS) в списке сохраняемых, Apple shared-компоненты (Xcode build scripts, MetalCPP) автоматически сохраняются.

## Примеры

```powershell
# Типичный набор для мобильной игры:
.\ue-trim.ps1 C:\UE5 -Platforms "Windows,Android,IOS"

# Только десктоп:
.\ue-trim.ps1 C:\UE5 -Platforms "Windows,Linux"

# Только Windows (максимальная очистка):
.\ue-trim.ps1 C:\UE5 -Platforms "Windows"

# Оставить всё кроме VisionOS и HoloLens:
.\ue-trim.ps1 C:\UE5 -Platforms "Windows,Linux,Android,IOS,Mac,TVOS,SteamDeck"

# Сохранить .git (для продолжения работы с репозиторием):
.\ue-trim.ps1 C:\UE5 -Platforms "Windows,Linux,Android,IOS" -KeepGit

# Сохранить .git и Intermediate (почистить только платформы):
.\ue-trim.ps1 C:\UE5 -Platforms "Windows,Linux,Android,IOS" -KeepGit -KeepIntermediate

# Максимальная очистка (всё + тесты + PDB):
.\ue-trim.ps1 C:\UE5 -Platforms "Windows" -RemoveTests -StripPdb
```

## Что удаляется

### По умолчанию (можно отключить флагами)

| Компонент | Описание | Типичный размер | Флаг для сохранения |
|---|---|---|---|
| `.git/` | Git-история | 20–60 ГБ | `--keep-git` / `-KeepGit` |
| `Engine/Intermediate/` | Кэш компиляции (регенерируется при сборке) | 10–50 ГБ | `--keep-intermediate` / `-KeepIntermediate` |

### Опционально (по флагу)

| Компонент | Описание | Типичный размер | Флаг |
|---|---|---|---|
| Тестовые бинарники | `*Tests/`, `HeadlessChaos/` в `Engine/Binaries/Win64/` | ~5 ГБ | `--remove-tests` / `-RemoveTests` |
| PDB символы | `.pdb` файлы во всём `Engine/Binaries/` | ~5.5 ГБ | `--strip-pdb` / `-StripPdb` |

### Всегда (независимо от платформ)

| Компонент | Описание | Типичный размер |
|---|---|---|
| `Engine/DerivedDataCache/` | Кэш ассетов | 0–5 ГБ |
| `Templates/` | Шаблоны проектов | ~1 ГБ |
| `Samples/` | Примеры (Lyra, StarterContent) | ~1 ГБ |
| `FeaturePacks/` | Пакеты шаблонов | <100 МБ |
| `Engine/Documentation/` | Оффлайн-документация | ~250 МБ |
| `Engine/Extras/Horde/` | CI/CD система Epic | ~500 МБ |
| `Engine/Extras/P4VUtils/` | Утилиты Perforce | ~350 МБ |
| `Engine/Extras/Maya_*` | Плагины Maya | ~350 МБ |
| `Engine/Extras/UnrealEngineLauncher/` | Лаунчер | ~200 МБ |
| `Engine/Extras/ThirdPartyNotUE/` | Сторонние утилиты | ~160 МБ |
| `Engine/Extras/VirtualProduction/` | VP тулы | ~130 МБ |
| и другие мелкие Extras | RoboMerge, Flutter, 3dsMax... | <50 МБ |
| `.idea/`, `UE5.sln`, `.tgitconfig` | IDE/проектные файлы | <10 МБ |

### В зависимости от платформ

Для каждой **неуказанной** платформы удаляются:

- `Engine/Platforms/<Platform>/` — платформенные расширения
- `Engine/Binaries/<Platform>/` — скомпилированные бинарники
- `Engine/Build/<Platform>/` — build-скрипты
- `Engine/Config/<Platform>/` — конфигурация
- `Engine/Extras/<Platform-specific>/` — платформенные утилиты
- Все подпапки с именем платформы внутри `Engine/Binaries/ThirdParty/` (рекурсивно)
- Все подпапки с именем платформы внутри `Engine/Source/ThirdParty/` (рекурсивно)
- `Engine/Source/Developer/<Platform>/` — исходники платформенных модулей редактора
- `Engine/Source/Programs/AutomationTool/<Platform>/` — билд-скрипты

## Ожидаемые результаты

Пример для `--platforms Windows,Linux,Android,IOS` на типичном репозитории UE 5.5:

| Источник экономии | Размер |
|---|---|
| `.git/` | ~59 ГБ |
| `Engine/Intermediate/` | ~42 ГБ |
| Mac ThirdParty (Binaries + Source) | ~8 ГБ |
| Extras (ненужные тулы) | ~1.5 ГБ |
| Templates + Samples | ~1.5 ГБ |
| VisionOS + TVOS + SteamDeck | ~1 ГБ |
| Documentation + Binaries/Mac | ~0.7 ГБ |
| **Базовый итого** | **~113 ГБ (~30%)** |
| + `--remove-tests` | **+5 ГБ** |
| + `--strip-pdb` | **+5.5 ГБ** |
| **Максимальный итого** | **~124 ГБ (~33%)** |

## Режимы работы

### Dry-run (по умолчанию)

Показывает список всего, что будет удалено, с размерами. Ничего не трогает.

```
╔══════════════════════════════════════════════════════════════╗
║  ue-trim: РЕЖИМ ПРЕДПРОСМОТРА (dry-run)                    ║
╚══════════════════════════════════════════════════════════════╝
  Сохраняем:  Windows, Linux, Android, IOS
  Удаляем:    Mac, TVOS, VisionOS, HoloLens, SteamDeck
  .git:            УДАЛЯЕМ
  Intermediate:    УДАЛЯЕМ
  Тесты (Win64):   сохраняем (--remove-tests для удаления)
  PDB символы:     сохраняем (--strip-pdb для удаления)
  Apple shared: СОХРАНЯЕМ (нужны для IOS)

── Шаг 1: Общая очистка ─────────────────────────────────────
  [УДАЛИТЬ] .git (59.12 ГБ)
  [УДАЛИТЬ] Engine\Intermediate (42.30 ГБ)
  ...

══════════════════════════════════════════════════════════════
ИТОГО (dry-run, ничего не удалено):
  Папок к удалению:  87
  Файлов к удалению: 3
  Освободится:       113.42 ГБ
```

### Execute

Реально удаляет. Перед удалением запрашивает подтверждение `(y/N)`.

## Сборка из исходников (Rust)

```bash
cargo build --release
# Бинарник: target/release/ue-trim.exe
```

Требования: Rust 1.56+ (edition 2021). Зависимостей нет — только стандартная библиотека.

## После очистки

1. Запустите `GenerateProjectFiles.bat` для регенерации файлов проекта
2. Выполните полную пересборку движка

## Предосторожности

- **Всегда сначала dry-run.** Убедитесь, что в списке нет ничего лишнего
- **Бэкап.** Удаление `.git/` необратимо — убедитесь, что есть копия репозитория
- **Apple-зависимости.** Если вы сохраняете iOS — папки с "Apple" в имени автоматически сохраняются (MetalCPP, Apple SDK, Xcode build scripts). Удаляются только папки с именем конкретной ненужной платформы (Mac, TVOS и т.д.)
- **Intermediate.** После удаления потребуется полная перекомпиляция (не инкрементальная), это может занять значительное время

## Лицензия

MIT
