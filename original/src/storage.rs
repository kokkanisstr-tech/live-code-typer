use std::{
    collections::HashMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;
use tempfile::NamedTempFile;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::{
    core::{Hotkeys, Library, Snippet},
    room::RoomSettings,
};

pub fn library_path() -> Result<PathBuf> {
    if let Some(folder) = std::env::var_os("LIVECODETYPER_DATA_FOLDER") {
        return Ok(PathBuf::from(folder).join("library.json"));
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(folder) = portable_data_folder(&executable) {
            return Ok(folder.join("library.json"));
        }
    }
    let dirs = ProjectDirs::from("org", "LiveCodeTyper", "LiveCodeTyper")
        .context("Не удалось определить каталог данных приложения")?;
    Ok(dirs.config_dir().join("library.json"))
}

fn portable_data_folder(executable: &Path) -> Option<PathBuf> {
    let parent = executable.parent()?;
    let adjacent = parent.join("portable-data");
    if adjacent.is_dir() {
        return Some(adjacent);
    }

    #[cfg(target_os = "macos")]
    {
        let bundle = parent.parent()?.parent()?;
        let is_app_bundle = bundle
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("app"));
        if is_app_bundle {
            let sibling = bundle.parent()?.join("portable-data");
            if sibling.is_dir() {
                return Some(sibling);
            }
        }
    }

    None
}

pub fn load() -> Result<Library> {
    let path = library_path()?;
    let mut migrated = false;
    let mut library = if path.exists() {
        let bytes =
            fs::read(&path).with_context(|| format!("Не удалось прочитать {}", path.display()))?;
        match serde_json::from_slice::<Library>(&bytes) {
            Ok(library) => library,
            Err(current_error) => {
                let legacy = deserialize_legacy(&bytes).with_context(|| {
                    format!(
                        "Файл {} не распознан как библиотека LiveCode Typer: {current_error}",
                        path.display()
                    )
                })?;
                migrated = true;
                legacy
            }
        }
    } else if let Some(legacy_path) = legacy_library_path().filter(|path| path.is_file()) {
        let bytes = fs::read(&legacy_path).with_context(|| {
            format!(
                "Не удалось прочитать старую библиотеку {}",
                legacy_path.display()
            )
        })?;
        migrated = true;
        deserialize_legacy(&bytes).with_context(|| {
            format!(
                "Не удалось перенести старую библиотеку {}",
                legacy_path.display()
            )
        })?
    } else {
        return Ok(Library::default());
    };

    if library.snippets.is_empty() {
        library = Library::default();
    } else if !library
        .snippets
        .iter()
        .any(|item| item.id == library.selected_id)
    {
        library.selected_id = library.snippets[0].id;
    }
    for snippet in &mut library.snippets {
        snippet.code = crate::core::normalize_newlines(&snippet.code);
        snippet.cursor = snippet.cursor.min(snippet.grapheme_count());
    }
    if migrated {
        save(&library).context("Не удалось сохранить перенесённую библиотеку")?;
    }
    Ok(library)
}

pub fn save(library: &Library) -> Result<()> {
    let path = library_path()?;
    let folder = path.parent().context("Нет каталога данных")?;
    fs::create_dir_all(folder)
        .with_context(|| format!("Не удалось создать {}", folder.display()))?;
    let bytes =
        serde_json::to_vec_pretty(library).context("Не удалось сериализовать библиотеку")?;
    let mut temporary = NamedTempFile::new_in(folder)
        .with_context(|| format!("Не удалось создать временный файл в {}", folder.display()))?;
    temporary
        .write_all(&bytes)
        .context("Не удалось записать библиотеку")?;
    temporary
        .as_file()
        .sync_all()
        .context("Не удалось синхронизировать библиотеку на диск")?;
    temporary
        .persist(&path)
        .map_err(|error| error.error)
        .with_context(|| format!("Не удалось атомарно заменить {}", path.display()))?;
    Ok(())
}

fn legacy_library_path() -> Option<PathBuf> {
    if let Some(folder) = std::env::var_os("LIVECODETYPER_LEGACY_DATA_FOLDER") {
        return Some(PathBuf::from(folder).join("library.json"));
    }

    #[cfg(target_os = "windows")]
    return std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|folder| folder.join("LiveCodeTyper").join("library.json"));

    #[cfg(not(target_os = "windows"))]
    None
}

fn deserialize_legacy(bytes: &[u8]) -> Result<Library> {
    let legacy: LegacyLibrary = serde_json::from_slice(bytes)
        .context("Старый формат библиотеки повреждён или не поддерживается")?;
    if legacy.snippets.is_empty() {
        anyhow::bail!("В старой библиотеке не найдены фрагменты")
    }

    let defaults = Hotkeys::default();
    let hotkeys = Hotkeys {
        toggle_typing: legacy_hotkey(&legacy.hotkeys, "ToggleTyping", &defaults.toggle_typing),
        next_snippet: legacy_hotkey(&legacy.hotkeys, "NextSnippet", &defaults.next_snippet),
        previous_snippet: legacy_hotkey(
            &legacy.hotkeys,
            "PreviousSnippet",
            &defaults.previous_snippet,
        ),
        restart: legacy_hotkey(&legacy.hotkeys, "Restart", &defaults.restart),
        toggle_pin: legacy_hotkey(&legacy.hotkeys, "TogglePin", &defaults.toggle_pin),
        emergency_stop: legacy_hotkey(&legacy.hotkeys, "EmergencyStop", &defaults.emergency_stop),
    };

    let snippets = legacy
        .snippets
        .into_iter()
        .map(|old| {
            let mut snippet = Snippet::new(
                if old.name.trim().is_empty() {
                    "Без названия"
                } else {
                    old.name.trim()
                },
                if old.language.trim().is_empty() {
                    "Текст"
                } else {
                    old.language.trim()
                },
                old.code,
            );
            if old.id != Uuid::nil() {
                snippet.id = old.id;
            }
            snippet.cursor =
                legacy_position_to_graphemes(&snippet.code, old.position.max(0) as usize);
            snippet.pinned = old.is_pinned;
            snippet.remote_key = old.remote_key;
            snippet.is_remote_assignment = old.is_remote_assignment;
            snippet
        })
        .collect::<Vec<_>>();

    let selected_id = legacy
        .selected_snippet_id
        .filter(|id| snippets.iter().any(|snippet| snippet.id == *id))
        .unwrap_or(snippets[0].id);
    Ok(Library {
        snippets,
        selected_id,
        hotkeys,
        room: RoomSettings {
            server_url: legacy
                .room_server_url
                .filter(|url| !url.trim().is_empty())
                .unwrap_or_else(|| RoomSettings::default().server_url),
            room_code: legacy.room_code.unwrap_or_default(),
            participant_name: legacy.participant_name.unwrap_or_default(),
            participant_id: legacy.participant_id,
        },
        onboarding_complete: false,
        typing_state: Default::default(),
    })
}

fn legacy_hotkey(bindings: &HashMap<String, LegacyHotkey>, action: &str, fallback: &str) -> String {
    bindings
        .get(action)
        .and_then(format_legacy_hotkey)
        .unwrap_or_else(|| fallback.to_owned())
}

fn format_legacy_hotkey(binding: &LegacyHotkey) -> Option<String> {
    const ALT: u32 = 0x0001;
    const CONTROL: u32 = 0x0002;
    const SHIFT: u32 = 0x0004;
    const SUPER: u32 = 0x0008;
    const MODIFIER_MASK: u32 = ALT | CONTROL | SHIFT | SUPER;

    if binding.modifiers & !MODIFIER_MASK != 0 {
        return None;
    }
    let key = virtual_key_name(binding.virtual_key)?;
    let mut parts = Vec::with_capacity(5);
    if binding.modifiers & CONTROL != 0 {
        parts.push("Ctrl".to_owned());
    }
    if binding.modifiers & ALT != 0 {
        parts.push("Alt".to_owned());
    }
    if binding.modifiers & SHIFT != 0 {
        parts.push("Shift".to_owned());
    }
    if binding.modifiers & SUPER != 0 {
        parts.push("Super".to_owned());
    }
    if parts.is_empty() {
        return None;
    }
    parts.push(key);
    Some(parts.join("+"))
}

fn virtual_key_name(virtual_key: u32) -> Option<String> {
    Some(match virtual_key {
        0x08 => "Backspace".to_owned(),
        0x09 => "Tab".to_owned(),
        0x0D => "Enter".to_owned(),
        0x1B => "Escape".to_owned(),
        0x20 => "Space".to_owned(),
        0x21 => "PageUp".to_owned(),
        0x22 => "PageDown".to_owned(),
        0x23 => "End".to_owned(),
        0x24 => "Home".to_owned(),
        0x25 => "ArrowLeft".to_owned(),
        0x26 => "ArrowUp".to_owned(),
        0x27 => "ArrowRight".to_owned(),
        0x28 => "ArrowDown".to_owned(),
        0x2D => "Insert".to_owned(),
        0x2E => "Delete".to_owned(),
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(virtual_key)?.to_string(),
        0x70..=0x87 => format!("F{}", virtual_key - 0x6F),
        _ => return None,
    })
}

fn legacy_position_to_graphemes(text: &str, utf16_position: usize) -> usize {
    let mut consumed = 0;
    for (index, grapheme) in text.graphemes(true).enumerate() {
        if consumed >= utf16_position {
            return index;
        }
        consumed += grapheme.encode_utf16().count();
        if consumed >= utf16_position {
            return index + 1;
        }
    }
    text.graphemes(true).count()
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct LegacyLibrary {
    snippets: Vec<LegacySnippet>,
    hotkeys: HashMap<String, LegacyHotkey>,
    selected_snippet_id: Option<Uuid>,
    room_server_url: Option<String>,
    room_code: Option<String>,
    participant_name: Option<String>,
    participant_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct LegacySnippet {
    id: Uuid,
    name: String,
    language: String,
    code: String,
    is_pinned: bool,
    position: i64,
    remote_key: Option<String>,
    is_remote_assignment: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct LegacyHotkey {
    modifiers: u32,
    virtual_key: u32,
}
