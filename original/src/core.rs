use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::room::{RoomSettings, RoomTaskView};

pub const ROOM_ASSIGNMENT_START_DELAY: Duration = Duration::from_secs(3);

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
    pub typing: bool,
    #[serde(skip)]
    pub automatic_typing: bool,
    #[serde(skip)]
    pub automatic_start_at: Option<Instant>,
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
        let first = Snippet::new(
            "Пример C#",
            "C#",
            "using System;\n\nclass Program\n{\n    static void Main()\n    {\n        Console.WriteLine(\"Hello, world!\");\n    }\n}",
        );
        let selected_id = first.id;
        Self {
            snippets: vec![
                first,
                Snippet::new(
                    "Пример Python",
                    "Python",
                    "def greet(name):\n    print(f\"Hello, {name}!\")\n\ngreet(\"world\")",
                ),
            ],
            selected_id,
            hotkeys: Hotkeys::default(),
            room: RoomSettings::default(),
            onboarding_complete: false,
            typing: false,
            automatic_typing: false,
            automatic_start_at: None,
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
            self.typing = true;
            self.automatic_typing = automatic;
            self.automatic_start_at = automatic.then(|| Instant::now() + delay);
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
        if !self.typing || self.selected_id != snippet_id {
            return false;
        }
        let Some(snippet) = self.selected_mut() else {
            self.typing = false;
            self.automatic_typing = false;
            self.automatic_start_at = None;
            return false;
        };
        if snippet.revision != revision || snippet.cursor != cursor {
            return false;
        }
        snippet.cursor += 1;
        if finished {
            self.typing = false;
            self.automatic_typing = false;
            self.automatic_start_at = None;
        }
        true
    }

    pub fn pause(&mut self) {
        self.typing = false;
        self.automatic_typing = false;
        self.automatic_start_at = None;
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
        self.pause();
        let current = self
            .snippets
            .iter()
            .position(|item| item.id == self.selected_id)
            .unwrap_or(0);
        let count = self.snippets.len() as isize;
        let next = (current as isize + offset).rem_euclid(count) as usize;
        self.selected_id = self.snippets[next].id;
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
