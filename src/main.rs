use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// База знаний о платформах UE
// ---------------------------------------------------------------------------

struct PlatformInfo {
    name: &'static str,
    folder_names: &'static [&'static str],
    binaries_dir: Option<&'static str>,
    platforms_dir: Option<&'static str>,
    build_dir: Option<&'static str>,
    config_dir: Option<&'static str>,
    extras_dirs: &'static [&'static str],
    /// Дополнительные папки для удаления (Source/Developer, Programs/AutomationTool и т.д.)
    source_dirs: &'static [&'static str],
    requires_apple: bool,
}

const ALL_PLATFORMS: &[PlatformInfo] = &[
    PlatformInfo {
        name: "Windows",
        folder_names: &["Win64", "Win32", "Windows"],
        binaries_dir: Some("Win64"),
        platforms_dir: Some("Windows"),
        build_dir: Some("Windows"),
        config_dir: None,
        extras_dirs: &["Windows"],
        source_dirs: &["Engine/Source/Developer/Windows"],
        requires_apple: false,
    },
    PlatformInfo {
        name: "Linux",
        folder_names: &["Linux", "LinuxArm64"],
        binaries_dir: Some("Linux"),
        platforms_dir: None,
        build_dir: Some("Linux"),
        config_dir: None,
        extras_dirs: &["GDBPrinters"],
        source_dirs: &[
            "Engine/Source/Developer/Linux",
            "Engine/Source/Programs/AutomationTool/Linux",
        ],
        requires_apple: false,
    },
    PlatformInfo {
        name: "Android",
        folder_names: &["Android"],
        binaries_dir: None,
        platforms_dir: Some("Android"),
        build_dir: Some("Android"),
        config_dir: Some("Android"),
        extras_dirs: &["Android"],
        source_dirs: &[
            "Engine/Source/Developer/Android",
            "Engine/Source/Programs/AutomationTool/Android",
        ],
        requires_apple: false,
    },
    PlatformInfo {
        name: "IOS",
        folder_names: &["IOS"],
        binaries_dir: None,
        platforms_dir: Some("IOS"),
        build_dir: Some("IOS"),
        config_dir: None,
        extras_dirs: &["iTunes"],
        source_dirs: &[
            "Engine/Source/Developer/IOS",
            "Engine/Source/Programs/AutomationTool/IOS",
        ],
        requires_apple: true,
    },
    PlatformInfo {
        name: "Mac",
        folder_names: &["Mac", "macos", "osx", "Darwin"],
        binaries_dir: Some("Mac"),
        platforms_dir: None,
        build_dir: Some("Mac"),
        config_dir: None,
        extras_dirs: &["LLDBDataFormatters", "Instruments"],
        source_dirs: &[
            "Engine/Source/Developer/Mac",
            "Engine/Source/Programs/AutomationTool/Mac",
        ],
        requires_apple: true,
    },
    PlatformInfo {
        name: "TVOS",
        folder_names: &["TVOS"],
        binaries_dir: None,
        platforms_dir: None,
        build_dir: Some("TVOS"),
        config_dir: Some("TVOS"),
        extras_dirs: &[],
        source_dirs: &["Engine/Source/Programs/AutomationTool/TVOS"],
        requires_apple: true,
    },
    PlatformInfo {
        name: "VisionOS",
        folder_names: &["VisionOS"],
        binaries_dir: None,
        platforms_dir: Some("VisionOS"),
        build_dir: None,
        config_dir: None,
        extras_dirs: &[],
        source_dirs: &[],
        requires_apple: true,
    },
    PlatformInfo {
        name: "HoloLens",
        folder_names: &["HoloLens"],
        binaries_dir: None,
        platforms_dir: None,
        build_dir: None,
        config_dir: None,
        extras_dirs: &[],
        source_dirs: &[],
        requires_apple: false,
    },
    PlatformInfo {
        name: "SteamDeck",
        folder_names: &["SteamDeck"],
        binaries_dir: None,
        platforms_dir: None,
        build_dir: Some("SteamDeck"),
        config_dir: None,
        extras_dirs: &[],
        source_dirs: &[],
        requires_apple: false,
    },
];

/// Папки, удаляемые ВСЕГДА (не зависят от платформ).
const ALWAYS_REMOVE_DIRS: &[&str] = &[
    "Engine/DerivedDataCache",
    "Templates",
    "Samples",
    "FeaturePacks",
    "Engine/Documentation",
    "Engine/Extras/Horde",
    "Engine/Extras/P4VUtils",
    "Engine/Extras/Maya_AnimationRiggingTools",
    "Engine/Extras/MayaVelocityGridExporter",
    "Engine/Extras/UnrealEngineLauncher",
    "Engine/Extras/ThirdPartyNotUE",
    "Engine/Extras/VirtualProduction",
    "Engine/Extras/RoboMerge",
    "Engine/Extras/Flutter",
    "Engine/Extras/3dsMaxScripts",
    ".idea",
];

const ALWAYS_REMOVE_FILES: &[&str] = &[
    ".tgitconfig",
    "PULL_REQUEST_TEMPLATE.md",
    "UE5.sln",
];

const APPLE_SHARED_DIRS: &[&str] = &[
    "Engine/Build/Xcode",
    "Engine/Extras/Xcode",
];

const THIRDPARTY_ROOTS: &[&str] = &[
    "Engine/Binaries/ThirdParty",
    "Engine/Source/ThirdParty",
];

// ---------------------------------------------------------------------------
// Статистика и утилиты
// ---------------------------------------------------------------------------

struct TrimStats {
    dirs_found: u64,
    files_found: u64,
    dirs_removed: u64,
    files_removed: u64,
    bytes_freed: u64,
    errors: Vec<String>,
}

impl TrimStats {
    fn new() -> Self {
        Self { dirs_found: 0, files_found: 0, dirs_removed: 0, files_removed: 0, bytes_freed: 0, errors: Vec::new() }
    }
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} ГБ", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} МБ", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} КБ", bytes as f64 / KB as f64)
    } else {
        format!("{} Б", bytes)
    }
}

fn find_platform_dirs(root: &Path, names: &[String], max_depth: u32) -> Vec<PathBuf> {
    let mut result = Vec::new();
    find_platform_dirs_inner(root, names, max_depth, 0, &mut result);
    result
}

fn find_platform_dirs_inner(dir: &Path, names: &[String], max_depth: u32, depth: u32, result: &mut Vec<PathBuf>) {
    if depth > max_depth { return; }
    let entries = match fs::read_dir(dir) { Ok(e) => e, Err(_) => return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let name = match path.file_name().and_then(|n| n.to_str()) { Some(n) => n, None => continue };
        if names.iter().any(|pn| pn.eq_ignore_ascii_case(name)) {
            result.push(path);
        } else {
            find_platform_dirs_inner(&path, names, max_depth, depth + 1, result);
        }
    }
}

fn remove_dir(path: &Path, stats: &mut TrimStats, dry_run: bool) {
    let size = dir_size(path);
    stats.dirs_found += 1;
    stats.bytes_freed += size;
    if dry_run {
        println!("  [УДАЛИТЬ] {} ({})", path.display(), format_size(size));
    } else {
        print!("  Удаляю {} ({})... ", path.display(), format_size(size));
        io::stdout().flush().ok();
        match fs::remove_dir_all(path) {
            Ok(_) => { println!("OK"); stats.dirs_removed += 1; }
            Err(e) => { println!("ОШИБКА: {}", e); stats.errors.push(format!("{}: {}", path.display(), e)); }
        }
    }
}

fn remove_file(path: &Path, stats: &mut TrimStats, dry_run: bool) {
    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
    stats.files_found += 1;
    stats.bytes_freed += size;
    if dry_run {
        println!("  [УДАЛИТЬ] {} ({})", path.display(), format_size(size));
    } else {
        print!("  Удаляю {}... ", path.display());
        io::stdout().flush().ok();
        match fs::remove_file(path) {
            Ok(_) => { println!("OK"); stats.files_removed += 1; }
            Err(e) => { println!("ОШИБКА: {}", e); stats.errors.push(format!("{}: {}", path.display(), e)); }
        }
    }
}

/// Удаляет все .pdb файлы рекурсивно внутри directory.
fn remove_pdb_files(directory: &Path, stats: &mut TrimStats, dry_run: bool) {
    let entries = match fs::read_dir(directory) { Ok(e) => e, Err(_) => return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            remove_pdb_files(&path, stats, dry_run);
        } else if let Some(ext) = path.extension() {
            if ext.eq_ignore_ascii_case("pdb") {
                remove_file(&path, stats, dry_run);
            }
        }
    }
}

/// Удаляет все папки, заканчивающиеся на "Tests" или "LowLevelTests" внутри directory (1 уровень).
fn remove_test_dirs(directory: &Path, stats: &mut TrimStats, dry_run: bool) {
    let entries = match fs::read_dir(directory) { Ok(e) => e, Err(_) => return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() { continue; }
        let name = match path.file_name().and_then(|n| n.to_str()) { Some(n) => n, None => continue };
        if name.ends_with("Tests") {
            remove_dir(&path, stats, dry_run);
        }
    }
}

fn validate_ue_root(root: &Path) -> bool {
    ["Engine/Source", "Engine/Binaries", "Engine/Build", "GenerateProjectFiles.bat"]
        .iter().all(|m| root.join(m).exists())
}

fn parse_platforms(arg: &str) -> Vec<String> {
    arg.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn normalize_platform_name(input: &str) -> Option<&'static str> {
    ALL_PLATFORMS.iter().find(|p| p.name.eq_ignore_ascii_case(input)).map(|p| p.name)
}

fn print_usage() {
    let all_names: Vec<&str> = ALL_PLATFORMS.iter().map(|p| p.name).collect();
    eprintln!(r#"
ue-trim — утилита для сокращения размера билда Unreal Engine

ИСПОЛЬЗОВАНИЕ:
    ue-trim <путь_к_UE> --platforms <список> [опции]

ПАРАМЕТРЫ:
    --platforms          Платформы для СОХРАНЕНИЯ (через запятую)
    --execute            Реально удалить (по умолчанию — dry-run)
    --keep-git           НЕ удалять .git/
    --keep-intermediate  НЕ удалять Engine/Intermediate/
    --remove-tests       Удалить тестовые бинарники из Engine/Binaries/Win64/ (~5 ГБ)
    --strip-pdb          Удалить .pdb debug-символы из Engine/Binaries/ (~5.5 ГБ)

ДОСТУПНЫЕ ПЛАТФОРМЫ:
    {platforms}

ПРИМЕРЫ:
    ue-trim C:\UE5 --platforms Windows,Linux,Android,IOS
    ue-trim C:\UE5 --platforms Windows,Linux --execute
    ue-trim C:\UE5 --platforms Windows,IOS --keep-git --remove-tests --strip-pdb
    ue-trim C:\UE5 --platforms Windows --keep-git --keep-intermediate
"#, platforms = all_names.join(", "));
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { print_usage(); std::process::exit(1); }

    let ue_root = PathBuf::from(&args[1]);
    let execute = args.iter().any(|a| a == "--execute");
    let dry_run = !execute;
    let keep_git = args.iter().any(|a| a == "--keep-git");
    let keep_intermediate = args.iter().any(|a| a == "--keep-intermediate");
    let remove_tests = args.iter().any(|a| a == "--remove-tests");
    let strip_pdb = args.iter().any(|a| a == "--strip-pdb");

    let platforms_arg = args.windows(2).find(|w| w[0] == "--platforms").map(|w| w[1].clone());
    let keep_platforms: Vec<String> = match platforms_arg {
        Some(ref arg) => parse_platforms(arg),
        None => { eprintln!("ОШИБКА: не указан параметр --platforms\n"); print_usage(); std::process::exit(1); }
    };

    let mut keep_normalized: Vec<&str> = Vec::new();
    for name in &keep_platforms {
        match normalize_platform_name(name) {
            Some(n) => keep_normalized.push(n),
            None => {
                let all_names: Vec<&str> = ALL_PLATFORMS.iter().map(|p| p.name).collect();
                eprintln!("ОШИБКА: неизвестная платформа '{}'\nДоступные: {}", name, all_names.join(", "));
                std::process::exit(1);
            }
        }
    }

    if !validate_ue_root(&ue_root) {
        eprintln!("ОШИБКА: '{}' не похоже на корень Unreal Engine.", ue_root.display());
        std::process::exit(1);
    }

    let platforms_to_remove: Vec<&PlatformInfo> = ALL_PLATFORMS.iter()
        .filter(|p| !keep_normalized.iter().any(|&k| k.eq_ignore_ascii_case(p.name)))
        .collect();

    let any_apple_kept = keep_normalized.iter()
        .any(|&k| ALL_PLATFORMS.iter().any(|p| p.name == k && p.requires_apple));

    let mut folder_names_to_remove: Vec<String> = Vec::new();
    for p in &platforms_to_remove {
        for &name in p.folder_names {
            let s = name.to_string();
            if !folder_names_to_remove.iter().any(|existing| existing.eq_ignore_ascii_case(&s)) {
                folder_names_to_remove.push(s);
            }
        }
    }

    // Заголовок
    println!("");
    if dry_run {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ue-trim: РЕЖИМ ПРЕДПРОСМОТРА (dry-run)                    ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    } else {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║  ue-trim: РЕЖИМ УДАЛЕНИЯ                                   ║");
        println!("║  ВНИМАНИЕ: Файлы будут удалены безвозвратно!                ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
    }

    let remove_names: Vec<&str> = platforms_to_remove.iter().map(|p| p.name).collect();
    println!("  Сохраняем:       {}", keep_normalized.join(", "));
    println!("  Удаляем:         {}", remove_names.join(", "));
    println!("  .git:            {}", if keep_git { "СОХРАНЯЕМ" } else { "УДАЛЯЕМ" });
    println!("  Intermediate:    {}", if keep_intermediate { "СОХРАНЯЕМ" } else { "УДАЛЯЕМ" });
    println!("  Тесты (Win64):   {}", if remove_tests { "УДАЛЯЕМ (~5 ГБ)" } else { "сохраняем (--remove-tests для удаления)" });
    println!("  PDB символы:     {}", if strip_pdb { "УДАЛЯЕМ (~5.5 ГБ)" } else { "сохраняем (--strip-pdb для удаления)" });
    if any_apple_kept {
        let apple_kept: Vec<&str> = keep_normalized.iter()
            .filter(|&&k| ALL_PLATFORMS.iter().any(|p| p.name == k && p.requires_apple))
            .cloned().collect();
        println!("  Apple shared:    СОХРАНЯЕМ (нужны для {})", apple_kept.join(", "));
    } else {
        println!("  Apple shared:    УДАЛЯЕМ");
    }

    if !dry_run {
        print!("\nПродолжить? (y/N): ");
        io::stdout().flush().ok();
        let mut input = String::new();
        io::stdin().read_line(&mut input).ok();
        if !input.trim().eq_ignore_ascii_case("y") { println!("Отменено."); std::process::exit(0); }
    }

    let mut stats = TrimStats::new();

    // ── Шаг 1: Общая очистка ──
    println!("\n── Шаг 1: Общая очистка ─────────────────────────────────");
    if !keep_git {
        let p = ue_root.join(".git");
        if p.exists() && p.is_dir() { remove_dir(&p, &mut stats, dry_run); }
    }
    if !keep_intermediate {
        let p = ue_root.join("Engine/Intermediate");
        if p.exists() && p.is_dir() { remove_dir(&p, &mut stats, dry_run); }
    }
    for rel in ALWAYS_REMOVE_DIRS {
        let p = ue_root.join(rel);
        if p.exists() && p.is_dir() { remove_dir(&p, &mut stats, dry_run); }
    }
    for rel in ALWAYS_REMOVE_FILES {
        let p = ue_root.join(rel);
        if p.exists() && p.is_file() { remove_file(&p, &mut stats, dry_run); }
    }

    // ── Шаг 2: Платформенные папки ──
    println!("\n── Шаг 2: Платформенные папки ────────────────────────────");
    for plat in &platforms_to_remove {
        if let Some(dir) = plat.platforms_dir {
            let p = ue_root.join("Engine/Platforms").join(dir);
            if p.exists() { remove_dir(&p, &mut stats, dry_run); }
        }
        if let Some(dir) = plat.binaries_dir {
            let p = ue_root.join("Engine/Binaries").join(dir);
            if p.exists() { remove_dir(&p, &mut stats, dry_run); }
        }
        if let Some(dir) = plat.build_dir {
            let p = ue_root.join("Engine/Build").join(dir);
            if p.exists() { remove_dir(&p, &mut stats, dry_run); }
        }
        if let Some(dir) = plat.config_dir {
            let p = ue_root.join("Engine/Config").join(dir);
            if p.exists() { remove_dir(&p, &mut stats, dry_run); }
        }
        for &dir in plat.extras_dirs {
            let p = ue_root.join("Engine/Extras").join(dir);
            if p.exists() { remove_dir(&p, &mut stats, dry_run); }
        }
        for &dir in plat.source_dirs {
            let p = ue_root.join(dir);
            if p.exists() { remove_dir(&p, &mut stats, dry_run); }
        }
    }

    if !any_apple_kept {
        println!("\n  Apple shared:");
        for rel in APPLE_SHARED_DIRS {
            let p = ue_root.join(rel);
            if p.exists() { remove_dir(&p, &mut stats, dry_run); }
        }
    }

    // ── Шаг 3: ThirdParty ──
    if !folder_names_to_remove.is_empty() {
        println!("\n── Шаг 3: ThirdParty ({}) ───────────────────────────", folder_names_to_remove.join(", "));
        for root_rel in THIRDPARTY_ROOTS {
            let root_path = ue_root.join(root_rel);
            if !root_path.exists() { continue; }
            println!("\n  Сканирую {}...", root_rel);
            let dirs = find_platform_dirs(&root_path, &folder_names_to_remove, 5);
            for pd in &dirs { remove_dir(pd, &mut stats, dry_run); }
            if dirs.is_empty() { println!("  (не найдено)"); }
        }
    }

    // ── Шаг 4: Тестовые бинарники ──
    if remove_tests {
        println!("\n── Шаг 4: Тестовые бинарники (Win64/*Tests) ─────────────");
        let win64 = ue_root.join("Engine/Binaries/Win64");
        if win64.exists() {
            remove_test_dirs(&win64, &mut stats, dry_run);
        }
        // HeadlessChaos — тестовый harness для Chaos физики
        let headless = ue_root.join("Engine/Binaries/Win64/HeadlessChaos");
        if headless.exists() { remove_dir(&headless, &mut stats, dry_run); }
    }

    // ── Шаг 5: PDB файлы ──
    if strip_pdb {
        println!("\n── Шаг 5: PDB debug-символы ─────────────────────────────");
        let binaries = ue_root.join("Engine/Binaries");
        if binaries.exists() {
            remove_pdb_files(&binaries, &mut stats, dry_run);
        }
    }

    // ── Итоги ──
    println!("\n══════════════════════════════════════════════════════════");
    if dry_run {
        println!("ИТОГО (dry-run, ничего не удалено):");
        println!("  Папок к удалению:  {}", stats.dirs_found);
        println!("  Файлов к удалению: {}", stats.files_found);
        println!("  Освободится:       {}", format_size(stats.bytes_freed));
        println!("\nДля реального удаления добавьте --execute");
    } else {
        println!("ИТОГО:");
        println!("  Папок удалено:     {} / {}", stats.dirs_removed, stats.dirs_found);
        println!("  Файлов удалено:    {} / {}", stats.files_removed, stats.files_found);
        println!("  Освобождено:       {}", format_size(stats.bytes_freed));
        if !stats.errors.is_empty() {
            println!("\nОШИБКИ ({}):", stats.errors.len());
            for e in &stats.errors { println!("  - {}", e); }
        }
        println!("\nРекомендации:");
        println!("  1. Запустите GenerateProjectFiles.bat");
        println!("  2. Выполните полную пересборку движка");
    }
}
