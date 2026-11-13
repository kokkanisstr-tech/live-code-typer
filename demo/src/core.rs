use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::room::{RoomSettings, RoomTaskView};

pub const ROOM_ASSIGNMENT_START_DELAY: Duration = Duration::from_secs(3);
pub const DEMO_TASK_COUNT: usize = 25;
pub const DEMO_HOTKEY_COUNT: usize = 7;

include!("prepared_tasks.rs");

pub fn demo_task_hotkey(task_number: usize) -> Option<String> {
    (1..=DEMO_HOTKEY_COUNT)
        .contains(&task_number)
        .then(|| format!("Alt+{task_number}"))
}

pub fn demo_mouse_offset(button: u8) -> Option<isize> {
    match button {
        4 => Some(1),
        5 => Some(-1),
        _ => None,
    }
}

pub fn navigate_demo_mouse(library: &mut Library, button: u8) -> bool {
    if !library.typing_state.is_active() {
        return false;
    }
    let Some(offset) = demo_mouse_offset(button) else {
        return false;
    };
    library.select_demo_offset(offset);
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snippet {
    pub id: Uuid,
    pub name: String,
    pub language: String,
    pub code: String,
    pub pinned: bool,
    pub cursor: usize,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub remote_key: Option<String>,
    #[serde(default)]
    pub is_remote_assignment: bool,
}

impl Snippet {
    pub fn new(
        name: impl Into<String>,
        language: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        let code = normalize_newlines(&code.into());
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            language: language.into(),
            cursor: 0,
            code,
            pinned: false,
            revision: 0,
            remote_key: None,
            is_remote_assignment: false,
        }
    }

    pub fn grapheme_count(&self) -> usize {
        self.code.graphemes(true).count()
    }

    pub fn next_grapheme(&self) -> Option<&str> {
        self.code.graphemes(true).nth(self.cursor)
    }

    pub fn set_code(&mut self, code: &str) -> bool {
        let normalized = normalize_newlines(code);
        if self.code == normalized {
            return false;
        }
        self.code = normalized;
        self.cursor = 0;
        self.revision = self.revision.wrapping_add(1);
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    pub snippets: Vec<Snippet>,
    pub selected_id: Uuid,
    pub hotkeys: Hotkeys,
    #[serde(default)]
    pub room: RoomSettings,
    #[serde(default)]
    pub onboarding_complete: bool,
    #[serde(skip)]
    pub typing_state: TypingState,
}

/// Runtime typing modes. The variants make impossible combinations such as
/// "stopped but automatic" or "manual with a scheduled start" unrepresentable.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TypingState {
    #[default]
    Idle,
    SignalDriven,
    WaitingUntil(Instant),
    Automatic,
}

impl TypingState {
    pub fn is_active(self) -> bool {
        !matches!(self, Self::Idle)
    }
    pub fn is_automatic(self) -> bool {
        matches!(self, Self::WaitingUntil(_) | Self::Automatic)
    }
    pub fn automatic_ready(self, now: Instant) -> bool {
        match self {
            Self::Automatic => true,
            Self::WaitingUntil(deadline) => now >= deadline,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Hotkeys {
    pub toggle_typing: String,
    pub next_snippet: String,
    pub previous_snippet: String,
    pub restart: String,
    pub toggle_pin: String,
    pub emergency_stop: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            toggle_typing: "Ctrl+Alt+Space".into(),
            next_snippet: "Ctrl+Alt+ArrowRight".into(),
            previous_snippet: "Ctrl+Alt+ArrowLeft".into(),
            restart: "Ctrl+Alt+R".into(),
            toggle_pin: "Ctrl+Alt+L".into(),
            emergency_stop: "Ctrl+Alt+Escape".into(),
        }
    }
}

impl Default for Library {
    fn default() -> Self {
        let snippets = prepared_task_snippets();
        let selected_id = snippets[0].id;
        Self {
            snippets,
            selected_id,
            hotkeys: Hotkeys::default(),
            room: RoomSettings::default(),
            onboarding_complete: false,
            typing_state: TypingState::Idle,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("Фрагмент не найден.")]
    MissingSnippet,
    #[error("Закреплённый фрагмент нельзя изменять.")]
    Pinned,
    #[error("Нельзя удалить последний фрагмент.")]
    LastSnippet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    Started,
    Insert(String),
    Finished,
    Ignored,
}

impl Library {
    pub fn selected(&self) -> Option<&Snippet> {
        self.snippets
            .iter()
            .find(|snippet| snippet.id == self.selected_id)
    }

    pub fn selected_mut(&mut self) -> Option<&mut Snippet> {
        self.snippets
            .iter_mut()
            .find(|snippet| snippet.id == self.selected_id)
    }

    pub fn start(&mut self) -> InputAction {
        let automatic = self
            .selected()
            .is_some_and(|snippet| snippet.is_remote_assignment);
        self.start_with_mode(automatic, Duration::ZERO)
    }

    pub fn start_automatic_after(&mut self, delay: Duration) -> InputAction {
        self.start_with_mode(true, delay)
    }

    pub fn apply_remote_assignment(
        &mut self,
        task: Option<&RoomTaskView>,
        start_delay: Duration,
    ) -> Option<InputAction> {
        self.pause();
        let task = task?;
        let remote_key = format!(
            "{}|{}|{}",
            self.room.server_url.trim_end_matches('/'),
            self.room.room_code,
            task.id
        );
        let remote_id = self
            .snippets
            .iter()
            .find(|snippet| snippet.remote_key.as_deref() == Some(remote_key.as_str()))
            .map(|snippet| snippet.id);
        let snippet_id = remote_id.unwrap_or_else(|| {
            let mut snippet = Snippet::new("Задание комнаты", "Текст", "");
            snippet.pinned = true;
            snippet.remote_key = Some(remote_key.clone());
            snippet.is_remote_assignment = true;
            let id = snippet.id;
            self.snippets.push(snippet);
            id
        });
        self.selected_id = snippet_id;
        if let Some(snippet) = self.selected_mut() {
            snippet.name = format!("№{} · {}", task.number, task.title);
            snippet.language = task.language.clone();
            snippet.set_code(&task.code);
            snippet.cursor = 0;
            snippet.pinned = true;
            snippet.remote_key = Some(remote_key);
            snippet.is_remote_assignment = true;
        }
        Some(self.start_automatic_after(start_delay))
    }

    fn start_with_mode(&mut self, automatic: bool, delay: Duration) -> InputAction {
        if self.selected().and_then(Snippet::next_grapheme).is_some() {
            self.typing_state = if automatic {
                let deadline = Instant::now() + delay;
                if delay.is_zero() {
                    TypingState::Automatic
                } else {
                    TypingState::WaitingUntil(deadline)
                }
            } else {
                TypingState::SignalDriven
            };
            InputAction::Started
        } else {
            self.pause();
            InputAction::Finished
        }
    }

    pub fn commit_input(
        &mut self,
        snippet_id: Uuid,
        revision: u64,
        cursor: usize,
        finished: bool,
    ) -> bool {
        if !self.typing_state.is_active() || self.selected_id != snippet_id {
            return false;
        }
        let Some(snippet) = self.selected_mut() else {
            self.typing_state = TypingState::Idle;
            return false;
        };
        if snippet.revision != revision || snippet.cursor != cursor {
            return false;
        }
        snippet.cursor += 1;
        if finished {
            self.typing_state = TypingState::Idle;
        }
        true
    }

    pub fn pause(&mut self) {
        self.typing_state = TypingState::Idle;
    }

    pub fn reset(&mut self) {
        self.pause();
        if let Some(snippet) = self.selected_mut() {
            snippet.cursor = 0;
        }
    }

    pub fn replace_code(&mut self, code: &str) -> Result<(), CoreError> {
        if self.selected().is_some_and(|snippet| snippet.pinned) {
            return Err(CoreError::Pinned);
        }
        if self.selected().is_none() {
            return Err(CoreError::MissingSnippet);
        }
        self.pause();
        let snippet = self.selected_mut().ok_or(CoreError::MissingSnippet)?;
        snippet.set_code(code);
        Ok(())
    }

    pub fn toggle_pin(&mut self) -> Result<bool, CoreError> {
        let snippet = self.selected_mut().ok_or(CoreError::MissingSnippet)?;
        if snippet.is_remote_assignment {
            return Err(CoreError::Pinned);
        }
        snippet.pinned = !snippet.pinned;
        Ok(snippet.pinned)
    }

    pub fn select_offset(&mut self, offset: isize) {
        if self.snippets.is_empty() {
            return;
        }
        let current = self
            .snippets
            .iter()
            .position(|item| item.id == self.selected_id)
            .unwrap_or(0);
        let count = self.snippets.len() as isize;
        let next = (current as isize + offset).rem_euclid(count) as usize;
        self.select_by_index(next);
    }

    pub fn select_demo_offset(&mut self, offset: isize) {
        let count = self.snippets.len().min(DEMO_TASK_COUNT);
        if count == 0 {
            return;
        }
        let current = self
            .snippets
            .iter()
            .take(count)
            .position(|item| item.id == self.selected_id)
            .unwrap_or(0);
        let next = (current as isize + offset).rem_euclid(count as isize) as usize;
        self.select_by_index(next);
    }

    pub fn select_by_index(&mut self, index: usize) -> bool {
        if index >= self.snippets.len() {
            return false;
        }
        let snippet = &mut self.snippets[index];
        if snippet.cursor >= snippet.grapheme_count() {
            snippet.cursor = 0;
        }
        self.selected_id = snippet.id;
        true
    }

    pub fn select_and_start(&mut self, index: usize) -> Option<InputAction> {
        if !self.select_by_index(index) {
            return None;
        }
        if self.typing_state.is_active() {
            Some(InputAction::Started)
        } else {
            Some(self.start())
        }
    }

    pub fn add_snippet(&mut self) -> Uuid {
        self.pause();
        let snippet = Snippet::new("Новый фрагмент", "Текст", "");
        let id = snippet.id;
        self.snippets.push(snippet);
        self.selected_id = id;
        id
    }

    pub fn delete_selected(&mut self) -> Result<(), CoreError> {
        let index = self
            .snippets
            .iter()
            .position(|item| item.id == self.selected_id)
            .ok_or(CoreError::MissingSnippet)?;
        if self.snippets[index].pinned {
            return Err(CoreError::Pinned);
        }
        self.pause();
        if self.snippets.len() == 1 {
            return Err(CoreError::LastSnippet);
        }
        self.snippets.remove(index);
        self.selected_id = self.snippets[index.min(self.snippets.len() - 1)].id;
        Ok(())
    }
}

pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}
