use std::{
    collections::HashSet,
    str::FromStr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, unbounded};
use eframe::egui;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState, hotkey::HotKey};
use uuid::Uuid;

use crate::{
    core::{
        DEMO_TASK_COUNT, Hotkeys, InputAction, Library, ROOM_ASSIGNMENT_START_DELAY, Snippet,
        demo_task_hotkey,
    },
    input::{InputEvent, InputHub},
    room::RoomTaskView,
    room_client::{RoomClient, RoomClientEvent},
    storage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HotkeyAction {
    ToggleTyping,
    NextSnippet,
    PreviousSnippet,
    Restart,
    TogglePin,
    EmergencyStop,
}

const GROUND: egui::Color32 = egui::Color32::from_rgb(12, 13, 15);
const SURFACE: egui::Color32 = egui::Color32::from_rgb(21, 23, 26);
const RAISED: egui::Color32 = egui::Color32::from_rgb(34, 36, 40);
const RULE: egui::Color32 = egui::Color32::from_rgb(41, 43, 47);
const CONTROL: egui::Color32 = egui::Color32::from_rgb(57, 59, 64);
const TEXT: egui::Color32 = egui::Color32::from_rgb(244, 244, 245);
const MUTED: egui::Color32 = egui::Color32::from_rgb(180, 180, 180);
const FAINT: egui::Color32 = egui::Color32::from_rgb(148, 148, 148);
const MINT: egui::Color32 = egui::Color32::from_rgb(131, 226, 186);
const NEGATIVE: egui::Color32 = egui::Color32::from_rgb(240, 140, 140);

fn apply_visual_style(context: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = GROUND;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = GROUND;
    visuals.faint_bg_color = SURFACE;
    visuals.code_bg_color = GROUND;
    visuals.hyperlink_color = MINT;
    visuals.window_stroke = egui::Stroke::new(1.0, RULE);
    visuals.selection.bg_fill = GROUND;
    visuals.selection.stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.noninteractive.bg_fill = GROUND;
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.inactive.bg_fill = RAISED;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, CONTROL);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(42, 44, 48);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, CONTROL);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(54, 56, 60);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, CONTROL);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, TEXT);
    visuals.widgets.open = visuals.widgets.hovered;

    context.set_theme(egui::Theme::Dark);
    let mut style = (*context.style_of(egui::Theme::Dark)).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(12.0, 10.0);
    style.spacing.button_padding = egui::vec2(14.0, 9.0);
    style.spacing.interact_size = egui::vec2(40.0, 38.0);
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(20.0));
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    style
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(13.0));
    context.set_style_of(egui::Theme::Dark, style);
}

fn app_card(fill: egui::Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .corner_radius(18.0)
        .inner_margin(18.0)
}

fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(GROUND))
            .fill(TEXT)
            .stroke(egui::Stroke::NONE)
            .corner_radius(18.0),
    )
}

pub fn run() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([360.0, 520.0]),
        ..Default::default()
    };
    eframe::run_native(
        "LiveCode Typer — Demo",
        options,
        Box::new(|creation| Ok(Box::new(PortableApp::new(creation)))),
    )
}

struct PortableApp {
    library: Arc<Mutex<Library>>,
    input: InputHub,
    hotkey_manager: Option<GlobalHotKeyManager>,
    registered_hotkeys: Vec<(HotkeyAction, HotKey)>,
    task_hotkeys: Vec<(usize, HotKey)>,
    hotkey_events: Receiver<GlobalHotKeyEvent>,
    editor_id: Option<Uuid>,
    editor_revision: u64,
    editor_characters: usize,
    editor_name: String,
    editor_language: String,
    editor_text: String,
    settings_open: bool,
    help_open: bool,
    hotkey_draft: Hotkeys,
    capturing_hotkey: Option<HotkeyAction>,
    status: String,
    dirty: bool,
    save_at: Option<Instant>,
    settings_error: Option<String>,
    room_open: bool,
    room_url_draft: String,
    room_code_draft: String,
    room_name_draft: String,
    room_error: Option<String>,
    room_status: String,
    room_connected: bool,
    room_client: Option<RoomClient>,
}

impl PortableApp {
    fn new(creation: &eframe::CreationContext<'_>) -> Self {
        apply_visual_style(&creation.egui_ctx);
        let (hotkey_sender, hotkey_events) = unbounded();
        let (library, load_error) = match storage::load() {
            Ok(library) => (library, None),
            Err(error) => (
                Library::default(),
                Some(format!("Библиотека не прочитана: {error}")),
            ),
        };
        let mut status = load_error.unwrap_or_else(|| "Готово".into());
        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => Some(manager),
            Err(error) => {
                status = format!("Горячие клавиши недоступны: {error}");
                None
            }
        };
        let mut registered_hotkeys = Vec::new();
        let mut task_hotkeys = Vec::new();
        match (&manager, parse_demo_task_hotkeys()) {
            (Some(manager), Ok(tasks)) => {
                let mut bindings = match parse_hotkeys(&library.hotkeys) {
                    Ok(bindings) => bindings,
                    Err(error) => {
                        status = error;
                        parse_hotkeys(&Hotkeys::default()).unwrap_or_default()
                    }
                };
                let reserved = tasks
                    .iter()
                    .map(|(_, key)| key.id())
                    .collect::<HashSet<_>>();
                let collisions = bindings
                    .iter()
                    .filter(|(_, key)| reserved.contains(&key.id()))
                    .map(|(_, key)| format!("{key} (конфликт с заданиями Alt+1–7)"))
                    .collect::<Vec<_>>();
                bindings.retain(|(_, key)| !reserved.contains(&key.id()));

                let mut unavailable = collisions;
                for (action, key) in bindings {
                    match manager.register(key) {
                        Ok(()) => registered_hotkeys.push((action, key)),
                        Err(error) => unavailable.push(format!("{key}: {error}")),
                    }
                }
                for (task_index, key) in tasks {
                    match manager.register(key) {
                        Ok(()) => task_hotkeys.push((task_index, key)),
                        Err(error) => unavailable.push(format!("Alt+{}: {error}", task_index + 1)),
                    }
                }
                if !unavailable.is_empty() {
                    status = format!(
                        "Некоторые сочетания не зарегистрированы: {}. Остальные работают.",
                        unavailable.join("; ")
                    );
                }
            }
            (Some(_), Err(error)) => status = error,
            (None, _) => status = "Глобальные горячие клавиши недоступны в этой среде.".into(),
        }

        let (editor_id, editor_revision, editor_name, editor_language, editor_text) =
            editor_state(&library);
        let editor_characters = library.selected().map_or(0, Snippet::grapheme_count);
        let help_open = !library.onboarding_complete;
        let room_settings = library.room.clone();
        let library = Arc::new(Mutex::new(library));
        let hotkey_context = creation.egui_ctx.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            let _ = hotkey_sender.send(event);
            hotkey_context.request_repaint();
        }));
        let input = InputHub::start(Arc::clone(&library));
        Self {
            library,
            input,
            hotkey_manager: manager,
            registered_hotkeys,
            task_hotkeys,
            hotkey_events,
            editor_id,
            editor_revision,
            editor_characters,
            editor_name,
            editor_language,
            editor_text,
            settings_open: false,
            help_open,
            hotkey_draft: Hotkeys::default(),
            capturing_hotkey: None,
            status,
            dirty: false,
            save_at: None,
            settings_error: None,
            room_open: false,
            room_url_draft: room_settings.server_url,
            room_code_draft: room_settings.room_code,
            room_name_draft: room_settings.participant_name,
            room_error: None,
            room_status: "Не подключено".into(),
            room_connected: false,
            room_client: None,
        }
    }

    fn open_room(&mut self) {
        let library = self
            .library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.room_url_draft = if is_loopback_room_url(&library.room.server_url) {
            String::new()
        } else {
            library.room.server_url.clone()
        };
        self.room_code_draft = library.room.room_code.clone();
        self.room_name_draft = library.room.participant_name.clone();
        drop(library);
        self.room_error = None;
        self.room_open = true;
    }

    fn connect_room(&mut self) {
        let (server_url, room_code) = match room_connection_details(
            self.room_url_draft.trim(),
            self.room_code_draft.trim(),
        ) {
            Ok(details) => details,
            Err(error) => {
                self.room_error = Some(error);
                return;
            }
        };
        let participant_name = self.room_name_draft.trim();
        if participant_name.is_empty() {
            self.room_error = Some("Введите своё имя.".into());
            return;
        }
        let mut library = self
            .library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        library.room.server_url = server_url.clone();
        library.room.room_code = room_code.clone();
        library.room.participant_name = participant_name.to_owned();
        self.room_url_draft = server_url;
        self.room_code_draft = room_code;
        let settings = library.room.clone();
        drop(library);
        self.mark_dirty();
        self.room_client.take();
        self.room_client = Some(RoomClient::connect(settings));
        self.room_status = "Подключение…".into();
        self.room_error = None;
        self.room_open = false;
    }

    fn apply_remote_assignment(&mut self, task: Option<RoomTaskView>) {
        let library_arc = Arc::clone(&self.library);
        let mut library = library_arc
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(task) = task else {
            library.apply_remote_assignment(None, ROOM_ASSIGNMENT_START_DELAY);
            drop(library);
            self.status = "Назначение снято. Локальные фрагменты сохранены.".into();
            self.mark_dirty();
            return;
        };
        let start_action =
            library.apply_remote_assignment(Some(&task), ROOM_ASSIGNMENT_START_DELAY);
        self.editor_id = None;
        self.sync_editor(&library);
        self.status = if matches!(start_action, Some(InputAction::Started)) {
            "Задание запущено. Через 3 секунды начнётся автоматическая печать. Переключись в нужное окно.".into()
        } else {
            format!(
                "Получено задание №{}: {}, но текст пустой.",
                task.number, task.title
            )
        };
        drop(library);
        self.mark_dirty();
    }

    fn apply_room_command(&mut self, command: &str) {
        let mut library = self
            .library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match command {
            "pause" => {
                library.pause();
                self.status = "Команда комнаты: печать остановлена.".into();
            }
            "restart" => {
                library.reset();
                self.status = "Команда комнаты: позиция сброшена. Включите печать вручную.".into();
            }
            _ => return,
        }
        drop(library);
        self.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.save_at = Some(Instant::now() + Duration::from_millis(450));
    }

    fn sync_editor(&mut self, library: &Library) {
        let Some(snippet) = library.selected() else {
            self.editor_id = None;
            self.editor_name.clear();
            self.editor_language.clear();
            self.editor_text.clear();
            return;
        };
        if self.editor_id != Some(snippet.id) || self.editor_revision != snippet.revision {
            self.editor_id = Some(snippet.id);
            self.editor_revision = snippet.revision;
            self.editor_characters = snippet.grapheme_count();
            self.editor_name = snippet.name.clone();
            self.editor_language = snippet.language.clone();
            self.editor_text = snippet.code.clone();
        }
    }

    fn handle_hotkey(&mut self, action: HotkeyAction) {
        let library_arc = Arc::clone(&self.library);
        let mut library = library_arc
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match action {
            HotkeyAction::ToggleTyping => {
                if library.typing_state.is_active() {
                    library.pause();
                    self.status = "Печать приостановлена".into();
                } else {
                    match library.start() {
                        InputAction::Started => {
                            self.status = if library.typing_state.is_automatic() {
                                "Автоматическая печать задания включена. Перейдите в нужное окно."
                                    .into()
                            } else {
                                "Печать включена. Перейдите в нужное приложение.".into()
                            }
                        }
                        _ => self.status = "В выбранном фрагменте больше нечего печатать.".into(),
                    }
                }
                self.mark_dirty();
            }
            HotkeyAction::NextSnippet => {
                library.select_offset(1);
                self.sync_editor(&library);
                self.status = "Выбран следующий фрагмент".into();
                self.mark_dirty();
            }
            HotkeyAction::PreviousSnippet => {
                library.select_offset(-1);
                self.sync_editor(&library);
                self.status = "Выбран предыдущий фрагмент".into();
                self.mark_dirty();
            }
            HotkeyAction::Restart => {
                library.reset();
                self.status = "Позиция сброшена в начало".into();
                self.mark_dirty();
            }
            HotkeyAction::TogglePin => {
                match library.toggle_pin() {
                    Ok(true) => self.status = "Фрагмент закреплён".into(),
                    Ok(false) => self.status = "Фрагмент откреплён".into(),
                    Err(error) => self.status = error.to_string(),
                }
                self.mark_dirty();
            }
            HotkeyAction::EmergencyStop => {
                library.pause();
                self.status = "Экстренная остановка: печать выключена".into();
                self.mark_dirty();
            }
        }
    }

    fn handle_demo_task_hotkey(&mut self, task_index: usize) {
        let library_arc = Arc::clone(&self.library);
        let mut library = library_arc
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if library.select_and_start(task_index).is_none() {
            self.status = format!("Задание №{} не найдено.", task_index + 1);
            return;
        }
        self.sync_editor(&library);
        self.status = if library.typing_state.is_active() {
            format!("Задание №{} включено. Печать продолжается.", task_index + 1)
        } else {
            format!(
                "Задание №{} выбрано. Нажми «Включить печать».",
                task_index + 1
            )
        };
        drop(library);
        self.mark_dirty();
    }

    fn open_settings(&mut self) {
        let library = self
            .library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.hotkey_draft = library.hotkeys.clone();
        drop(library);
        self.settings_error = None;
        self.capturing_hotkey = None;
        self.settings_open = true;
    }

    fn apply_hotkeys(&mut self) -> Result<(), String> {
        let next = parse_hotkeys(&self.hotkey_draft)?;
        if next.iter().any(|(_, key)| {
            self.task_hotkeys
                .iter()
                .any(|(_, task_key)| task_key.id() == key.id())
        }) {
            return Err("Alt+1–7 зарезервированы для выбора семи заданий.".into());
        }
        let manager = self
            .hotkey_manager
            .as_ref()
            .ok_or_else(|| "Глобальные сочетания клавиш недоступны в этой среде.".to_owned())?;
        let old = self.registered_hotkeys.clone();
        let mut removed = Vec::new();
        for (_, key) in old.iter().copied() {
            if let Err(error) = manager.unregister(key) {
                let mut restored = Vec::new();
                let mut restore_errors = Vec::new();
                for binding in old.iter().copied() {
                    if removed.contains(&binding.1) {
                        match manager.register(binding.1) {
                            Ok(()) => restored.push(binding),
                            Err(restore_error) => restore_errors.push(restore_error.to_string()),
                        }
                    } else {
                        restored.push(binding);
                    }
                }
                self.registered_hotkeys = restored;
                let suffix = if restore_errors.is_empty() {
                    String::new()
                } else {
                    format!(
                        " Не все старые сочетания удалось вернуть: {}.",
                        restore_errors.join("; ")
                    )
                };
                return Err(format!(
                    "Не удалось снять старое сочетание {key}: {error}.{suffix}"
                ));
            }
            removed.push(key);
        }

        let mut registered = Vec::new();
        for binding in next.iter().copied() {
            if let Err(error) = manager.register(binding.1) {
                let mut rollback_errors = Vec::new();
                for (_, key) in registered.iter().copied() {
                    if let Err(rollback_error) = manager.unregister(key) {
                        rollback_errors.push(rollback_error.to_string());
                    }
                }
                let mut restored = Vec::new();
                for old_binding in old.iter().copied() {
                    match manager.register(old_binding.1) {
                        Ok(()) => restored.push(old_binding),
                        Err(restore_error) => rollback_errors.push(restore_error.to_string()),
                    }
                }
                self.registered_hotkeys = restored;
                let suffix = if rollback_errors.is_empty() {
                    "Старые сочетания возвращены.".to_owned()
                } else {
                    format!(
                        "Проверьте горячие клавиши в настройках: {}.",
                        rollback_errors.join("; ")
                    )
                };
                return Err(format!(
                    "Не удалось назначить {0}: {error}. {suffix}",
                    binding.1
                ));
            }
            registered.push(binding);
        }

        self.registered_hotkeys = registered;
        let library_arc = Arc::clone(&self.library);
        let mut library = library_arc
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        library.hotkeys = self.hotkey_draft.clone();
        drop(library);
        self.mark_dirty();
        Ok(())
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui, library: &mut Library) {
        ui.horizontal(|ui| {
            ui.heading("Библиотека");
            ui.label(egui::RichText::new(library.snippets.len().to_string()).color(FAINT));
        });
        ui.label(egui::RichText::new("Тексты и код для набора").color(MUTED));
        ui.add_space(12.0);
        let items = library
            .snippets
            .iter()
            .map(|snippet| {
                (
                    snippet.id,
                    snippet.name.clone(),
                    snippet.language.clone(),
                    snippet.pinned,
                )
            })
            .collect::<Vec<_>>();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (id, name, language, pinned) in items {
                let selected = library.selected_id == id;
                let note = if pinned {
                    "Закреплён"
                } else {
                    "Изменяемый"
                };
                if ui
                    .add_sized(
                        [ui.available_width(), 54.0],
                        egui::Button::selectable(
                            selected,
                            egui::RichText::new(format!("{name}\n{note} · {language}"))
                                .color(if selected { TEXT } else { MUTED }),
                        )
                        .stroke(egui::Stroke::NONE)
                        .corner_radius(10.0),
                    )
                    .clicked()
                {
                    library.selected_id = id;
                    library.pause();
                    self.sync_editor(library);
                    self.status = "Фрагмент выбран".into();
                    self.mark_dirty();
                }
            }
        });
        ui.add_space(8.0);
        if ui.button("Новый фрагмент").clicked() {
            library.add_snippet();
            self.sync_editor(library);
            self.status = "Создан новый фрагмент".into();
            self.mark_dirty();
        }
    }

    fn render_editor(&mut self, ui: &mut egui::Ui, library: &mut Library) {
        let Some((selected_id, pinned, remote, cursor, name, language, next)) =
            library.selected().map(|snippet| {
                (
                    snippet.id,
                    snippet.pinned,
                    snippet.is_remote_assignment,
                    snippet.cursor,
                    snippet.name.clone(),
                    snippet.language.clone(),
                    snippet.next_grapheme().map(str::to_owned),
                )
            })
        else {
            ui.heading("Текст для набора");
            ui.label("Создай новый фрагмент, чтобы вставить код или текст.");
            return;
        };
        let is_typing = library.typing_state.is_active();

        if !library.onboarding_complete || self.help_open {
            app_card(RAISED).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading("Как начать");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Понятно").clicked() {
                            library.onboarding_complete = true;
                            self.help_open = false;
                            self.mark_dirty();
                        }
                    });
                });
                ui.label("1. Выбери задание: Alt+1–7 сразу включает соответствующий код.");
                ui.label("2. Перейди в окно редактора и нажимай случайные клавиши — будет печататься выбранный код.");
                ui.label(
                    "3. Кнопка мыши 4 переключает вперёд, кнопка 5 — назад. Активная печать не выключается.",
                );
                ui.label(
                    egui::RichText::new(
                        "Сочетания меняются в «Горячих клавишах». Закрепи фрагмент, чтобы защитить его от правок.",
                    )
                    .color(MUTED),
                );
            });
            ui.add_space(12.0);
        }

        app_card(SURFACE).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Текст для набора");
                    ui.label(egui::RichText::new(format!("{name} · {language}")).color(MUTED));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if remote {
                        ui.label(egui::RichText::new("Задание комнаты").color(MUTED));
                    } else if ui
                        .button(if pinned {
                            "Снять закрепление"
                        } else {
                            "Закрепить код"
                        })
                        .clicked()
                    {
                        match library.toggle_pin() {
                            Ok(true) => self.status = "Код закреплён от изменений".into(),
                            Ok(false) => self.status = "Код снова можно менять".into(),
                            Err(error) => self.status = error.to_string(),
                        }
                        self.mark_dirty();
                    }
                });
            });

            ui.add_space(10.0);
            let can_edit = !pinned && !remote && !is_typing;
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Название").color(MUTED));
                    let response = ui.add_sized(
                        [ui.available_width().min(440.0), 38.0],
                        egui::TextEdit::singleline(&mut self.editor_name)
                            .hint_text("Название фрагмента"),
                    );
                    if response.changed() {
                        if let Some(snippet) = library.selected_mut() {
                            snippet.name = if self.editor_name.trim().is_empty() {
                                "Без названия".into()
                            } else {
                                self.editor_name.trim().to_owned()
                            };
                        }
                        self.mark_dirty();
                    }
                });
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("Язык").color(MUTED));
                    ui.add_enabled_ui(can_edit, |ui| {
                        egui::ComboBox::from_id_salt("language-picker")
                            .selected_text(&self.editor_language)
                            .width(144.0)
                            .show_ui(ui, |ui| {
                                for language in [
                                    "C#",
                                    "Python",
                                    "JavaScript",
                                    "TypeScript",
                                    "HTML",
                                    "CSS",
                                    "SQL",
                                    "Java",
                                    "C++",
                                    "Текст",
                                ] {
                                    ui.selectable_value(
                                        &mut self.editor_language,
                                        language.to_owned(),
                                        language,
                                    );
                                }
                            });
                    });
                });
            });

            if let Some(snippet) = library.selected_mut() {
                if snippet.language != self.editor_language {
                    snippet.language = self.editor_language.clone();
                    self.mark_dirty();
                }
            }

            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                let toggle_label = if is_typing {
                    "Остановить печать"
                } else {
                    "Включить печать"
                };
                if primary_button(ui, toggle_label).clicked() {
                    if library.typing_state.is_active() {
                        library.pause();
                        self.status = "Печать приостановлена".into();
                    } else if matches!(library.start(), InputAction::Started) {
                        self.status = if library.typing_state.is_automatic() {
                            "Задание печатается автоматически. Перейди в нужное окно.".into()
                        } else {
                            "Печать включена. Перейди в нужное окно и нажимай любые клавиши.".into()
                        };
                        library.onboarding_complete = true;
                        self.help_open = false;
                    } else {
                        self.status = "Вставь текст: выбранный фрагмент пуст.".into();
                    }
                    self.mark_dirty();
                }
                if ui.button("Сначала").clicked() {
                    library.reset();
                    self.status = "Позиция сброшена в начало".into();
                    self.mark_dirty();
                }
                if ui.button("Дублировать").clicked() {
                    if let Some(source) = library.selected().cloned() {
                        let duplicate = Snippet::new(
                            format!("{} — копия", source.name),
                            source.language,
                            source.code,
                        );
                        library.snippets.push(duplicate);
                        library.selected_id = library
                            .snippets
                            .last()
                            .map(|item| item.id)
                            .unwrap_or(selected_id);
                        self.sync_editor(library);
                        self.status = "Фрагмент скопирован".into();
                        self.mark_dirty();
                    }
                }
                ui.add_enabled_ui(can_edit, |ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Удалить").color(NEGATIVE))
                                .stroke(egui::Stroke::new(1.0, CONTROL)),
                        )
                        .clicked()
                    {
                        match library.delete_selected() {
                            Ok(()) => {
                                self.sync_editor(library);
                                self.status = "Фрагмент удалён".into();
                                self.mark_dirty();
                            }
                            Err(error) => self.status = error.to_string(),
                        }
                    }
                });
            });

            let next_label = match next.as_deref() {
                Some("\n") => "Enter".to_owned(),
                Some("\t") => "Tab".to_owned(),
                Some(" ") => "Пробел".to_owned(),
                Some(character) => character.to_owned(),
                None if self.editor_characters == 0 => "Вставь текст".to_owned(),
                None => "Готово".to_owned(),
            };
            ui.add_space(12.0);
            egui::Frame::new()
                .fill(GROUND)
                .corner_radius(10.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Следующий символ").color(MUTED));
                        ui.label(egui::RichText::new(next_label).color(MINT).monospace());
                    });
                    let progress = if self.editor_characters == 0 {
                        0.0
                    } else {
                        cursor as f32 / self.editor_characters as f32
                    };
                    ui.add(
                        egui::ProgressBar::new(progress.clamp(0.0, 1.0))
                            .fill(MINT)
                            .text(format!("{cursor} / {} символов", self.editor_characters)),
                    );
                });

            ui.add_space(12.0);
            if pinned || remote {
                ui.label(
                    egui::RichText::new("Код закреплён и защищён от случайных изменений.")
                        .color(MUTED),
                );
            } else if is_typing {
                ui.label(
                    egui::RichText::new("Переключись в нужное приложение и нажимай любые клавиши.")
                        .color(MUTED),
                );
            } else {
                ui.label(
                    egui::RichText::new(
                        "После включения любая нажатая клавиша отправит следующий символ.",
                    )
                    .color(MUTED),
                );
            }
            ui.add_space(6.0);
            let editor = egui::TextEdit::multiline(&mut self.editor_text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(12)
                .interactive(can_edit)
                .frame(egui::Frame::NONE)
                .hint_text("Вставь сюда код или любой текст…");
            let editor_height = ui.available_height().max(220.0);
            let response = ui.add_sized([ui.available_width(), editor_height], editor);
            if response.changed() {
                match library.replace_code(&self.editor_text) {
                    Ok(()) => {
                        if let Some(snippet) = library.selected() {
                            self.editor_revision = snippet.revision;
                            self.editor_characters = snippet.grapheme_count();
                        }
                        self.mark_dirty();
                    }
                    Err(error) => self.status = error.to_string(),
                }
            }
        });
    }

    fn render_settings(&mut self, context: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = self.settings_open;
        let dialog_width = context.content_rect().width().clamp(320.0, 560.0);
        egui::Window::new("Горячие клавиши")
            .open(&mut open)
            .resizable(true)
            .default_width(dialog_width)
            .min_width(320.0)
            .show(context, |ui| {
                ui.label("Выбери «Изменить», нажми сочетание и сохрани изменения.");
                ui.label(
                    egui::RichText::new("Сочетания действуют в любом приложении.").color(MUTED),
                );
                if cfg!(windows) {
                    ui.label(
                        egui::RichText::new(
                            "Windows резервирует Alt+Space для системного меню; используй Ctrl+Alt+Space.",
                        )
                        .color(MUTED),
                    );
                }
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| {
                        let rows = [
                            (
                                HotkeyAction::ToggleTyping,
                                "Включить или приостановить печать",
                                self.hotkey_draft.toggle_typing.clone(),
                            ),
                            (
                                HotkeyAction::NextSnippet,
                                "Следующий фрагмент",
                                self.hotkey_draft.next_snippet.clone(),
                            ),
                            (
                                HotkeyAction::PreviousSnippet,
                                "Предыдущий фрагмент",
                                self.hotkey_draft.previous_snippet.clone(),
                            ),
                            (
                                HotkeyAction::Restart,
                                "Начать выбранный текст сначала",
                                self.hotkey_draft.restart.clone(),
                            ),
                            (
                                HotkeyAction::TogglePin,
                                "Закрепить или открепить код",
                                self.hotkey_draft.toggle_pin.clone(),
                            ),
                            (
                                HotkeyAction::EmergencyStop,
                                "Экстренно остановить печать",
                                self.hotkey_draft.emergency_stop.clone(),
                            ),
                        ];
                        for (action, label, value) in rows {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(label);
                                    ui.label(egui::RichText::new(value).color(MUTED).monospace());
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .button(if self.capturing_hotkey == Some(action) {
                                                "Нажми сочетание…"
                                            } else {
                                                "Изменить"
                                            })
                                            .clicked()
                                        {
                                            self.capturing_hotkey = Some(action);
                                            self.settings_error = None;
                                        }
                                    },
                                );
                            });
                            ui.separator();
                        }
                    });
                if let Some(error) = &self.settings_error {
                    ui.colored_label(NEGATIVE, error);
                }
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("По умолчанию").clicked() {
                        self.hotkey_draft = Hotkeys::default();
                        self.settings_error = None;
                        self.capturing_hotkey = None;
                    }
                    if ui.button("Отмена").clicked() {
                        self.settings_open = false;
                        self.capturing_hotkey = None;
                    }
                    if primary_button(ui, "Сохранить сочетания").clicked() {
                        match self.apply_hotkeys() {
                            Ok(()) => {
                                self.settings_open = false;
                                self.capturing_hotkey = None;
                                self.settings_error = None;
                                self.status = "Горячие клавиши сохранены".into();
                            }
                            Err(error) => self.settings_error = Some(error),
                        }
                    }
                });
            });
        self.settings_open = open && self.settings_open;

        if let Some(action) = self.capturing_hotkey {
            let events = context.input(|input| input.events.clone());
            for event in events {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } = event
                {
                    if key == egui::Key::Escape && modifiers.is_none() {
                        self.capturing_hotkey = None;
                        self.status = "Назначение сочетания отменено".into();
                        break;
                    }
                    let Some(key_name) = egui_key_name(key) else {
                        continue;
                    };
                    if !modifiers.ctrl && !modifiers.command && !modifiers.alt && !modifiers.shift {
                        self.settings_error = Some("Добавьте Ctrl, Cmd, Alt или Shift.".into());
                        continue;
                    }
                    let mut parts = Vec::new();
                    if modifiers.command && cfg!(target_os = "macos") {
                        parts.push("Cmd");
                    } else if modifiers.ctrl || modifiers.command {
                        parts.push("Ctrl");
                    }
                    if modifiers.alt {
                        parts.push("Alt");
                    }
                    if modifiers.shift {
                        parts.push("Shift");
                    }
                    parts.push(key_name);
                    let value = parts.join("+");
                    set_hotkey_value(&mut self.hotkey_draft, action, value);
                    self.settings_error = None;
                    self.capturing_hotkey = None;
                    break;
                }
            }
        }
    }

    fn render_room(&mut self, context: &egui::Context) {
        if !self.room_open {
            return;
        }
        let mut open = self.room_open;
        let mut disconnect = false;
        let mut close_after_connect = false;
        let dialog_width = context.content_rect().width().clamp(320.0, 520.0);
        egui::Window::new("Онлайн-комната")
            .open(&mut open)
            .resizable(true)
            .default_width(dialog_width)
            .min_width(320.0)
            .show(context, |ui| {
                ui.heading("Общий код для группы");
                ui.label("Вставь ссылку комнаты, введи имя и подключись к той же сети.");
                ui.label("Чтобы текст появился в редакторе, получатель запускает LiveCode Typer.");
                ui.label(egui::RichText::new("У всех одинаковые права.").color(MUTED));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(if self.room_connected {
                        "Подключено"
                    } else {
                        "Не подключено"
                    });
                    ui.label(
                        egui::RichText::new(&self.room_status).color(if self.room_connected {
                            MINT
                        } else {
                            MUTED
                        }),
                    );
                });
                if self.room_connected {
                    ui.add_space(8.0);
                    ui.label(
                        "Открой веб-панель, чтобы выбрать участника и отправить ему код или текст.",
                    );
                    ui.horizontal_wrapped(|ui| {
                        if primary_button(ui, "Открыть веб-панель").clicked() {
                            match room_invite_url(
                                self.room_url_draft.trim(),
                                self.room_code_draft.trim(),
                            ) {
                                Ok(url) => ui.ctx().open_url(egui::OpenUrl::new_tab(url)),
                                Err(error) => self.room_error = Some(error),
                            }
                        }
                    });
                } else {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Ссылка комнаты или адрес сервера").color(MUTED));
                    ui.label("Вставь ссылку с компьютера ведущего. localhost доступен только на его компьютере.");
                    ui.add_sized(
                        [ui.available_width(), 38.0],
                        egui::TextEdit::singleline(&mut self.room_url_draft)
                            .hint_text("Вставь ссылку от ведущего"),
                    );
                    ui.label(egui::RichText::new("Код комнаты").color(MUTED));
                    ui.add_sized(
                        [ui.available_width(), 38.0],
                        egui::TextEdit::singleline(&mut self.room_code_draft)
                            .hint_text("Код от ведущего"),
                    );
                    ui.label(egui::RichText::new("Твоё имя").color(MUTED));
                    ui.add_sized(
                        [ui.available_width(), 38.0],
                        egui::TextEdit::singleline(&mut self.room_name_draft)
                            .hint_text("Имя для группы"),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        if primary_button(ui, "Подключиться").clicked() {
                            self.connect_room();
                            close_after_connect = !self.room_open;
                        }
                        if ui.button("Открыть веб-панель").clicked() {
                            match room_invite_url(
                                self.room_url_draft.trim(),
                                self.room_code_draft.trim(),
                            ) {
                                Ok(url) => ui.ctx().open_url(egui::OpenUrl::new_tab(url)),
                                Err(error) => self.room_error = Some(error),
                            }
                        }
                    });
                    if let Some(error) = &self.room_error {
                        ui.colored_label(NEGATIVE, error);
                    }
                }
                if self.room_connected {
                    if let Some(error) = &self.room_error {
                        ui.colored_label(NEGATIVE, error);
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Отключиться").color(NEGATIVE))
                                .stroke(egui::Stroke::new(1.0, CONTROL)),
                        )
                        .clicked()
                    {
                        disconnect = true;
                    }
                }
            });
        self.room_open = open && !disconnect && !close_after_connect;
        if disconnect {
            self.room_client.take();
            self.room_connected = false;
            self.room_status = "Отключено".into();
            self.status = "Вы отключились от онлайн-комнаты.".into();
        }
    }

    fn render_footer(&self, root: &mut egui::Ui, typing: bool, hotkeys: &Hotkeys) {
        egui::Panel::bottom("footer")
            .frame(
                egui::Frame::new()
                    .fill(GROUND)
                    .stroke(egui::Stroke::new(1.0, RULE))
                    .inner_margin(14.0),
            )
            .show(root, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(if typing {
                            "Печать включена"
                        } else {
                            "Печать выключена"
                        })
                        .color(if typing { MINT } else { MUTED }),
                    );
                    ui.separator();
                    ui.label(egui::RichText::new(&self.status).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Экстренная остановка · {}",
                                hotkeys.emergency_stop
                            ))
                            .color(FAINT)
                            .monospace(),
                        );
                    });
                    ui.label(
                        egui::RichText::new("Alt+1–7 · задания  |  мышь 4 → вперёд · 5 → назад")
                            .color(FAINT),
                    );
                });
            });
    }

    fn save_if_due(&mut self) {
        if !self.dirty
            || self
                .save_at
                .is_none_or(|deadline| Instant::now() < deadline)
        {
            return;
        }
        let library = self
            .library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match storage::save(&library) {
            Ok(()) => {
                self.dirty = false;
                self.save_at = None;
            }
            Err(error) => {
                self.status = format!("Автосохранение не удалось: {error}");
                self.save_at = Some(Instant::now() + Duration::from_secs(2));
            }
        }
    }
}

fn room_connection_details(input: &str, fallback_code: &str) -> Result<(String, String), String> {
    let mut url = reqwest::Url::parse(input).map_err(|_| {
        "Вставь ссылку комнаты, например http://192.168.1.20:5080/?roomCode=482731.".to_owned()
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Ссылка комнаты должна начинаться с http:// или https://.".into());
    }
    if url.path() != "/" {
        return Err("Укажи ссылку на главную страницу комнаты без дополнительного пути.".into());
    }
    let invite_code = url
        .query_pairs()
        .find(|(key, _)| key == "roomCode")
        .map(|(_, value)| value.into_owned())
        .filter(|code| !code.trim().is_empty());
    url.set_query(None);
    url.set_fragment(None);
    let server_url = url.as_str().trim_end_matches('/').to_owned();
    let room_code = invite_code.unwrap_or_else(|| fallback_code.trim().to_owned());
    if room_code.is_empty() {
        return Err("В ссылке нет кода комнаты. Вставь ссылку ведущего или введи код.".into());
    }
    Ok((server_url, room_code))
}

fn is_loopback_room_url(input: &str) -> bool {
    reqwest::Url::parse(input)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
        .is_some_and(|host| {
            let host = host
                .strip_prefix('[')
                .and_then(|host| host.strip_suffix(']'))
                .unwrap_or(&host);
            host.eq_ignore_ascii_case("localhost")
                || host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        })
}

fn room_invite_url(input: &str, fallback_code: &str) -> Result<String, String> {
    let (server_url, room_code) = room_connection_details(input, fallback_code)?;
    let mut url = reqwest::Url::parse(&server_url)
        .map_err(|_| "Не удалось прочитать адрес сервера комнаты.".to_owned())?;
    url.query_pairs_mut().append_pair("roomCode", &room_code);
    Ok(url.into())
}

impl eframe::App for PortableApp {
    fn ui(&mut self, root: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = root.ctx().clone();
        self.input.set_focus(root.input(|input| input.focused));
        let input_events = self.input.events().try_iter().collect::<Vec<_>>();
        for event in input_events {
            match event {
                InputEvent::Ready => {
                    self.status = "Готово · клавиатура и кнопки мыши подключены".into()
                }
                InputEvent::Error(error) => self.status = error,
                InputEvent::Selection { snippet_id } => {
                    let library_arc = Arc::clone(&self.library);
                    let library = library_arc
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if library.selected_id == snippet_id {
                        let name = library
                            .selected()
                            .map(|snippet| snippet.name.clone())
                            .unwrap_or_else(|| "задание".into());
                        self.sync_editor(&library);
                        self.status = format!("Выбрано {name}. Печать продолжается.");
                        drop(library);
                        self.mark_dirty();
                    }
                }
                InputEvent::Progress {
                    snippet_id,
                    cursor,
                    finished,
                } => {
                    let is_current = {
                        let library = self
                            .library
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        library.selected_id == snippet_id
                            && library
                                .selected()
                                .is_some_and(|snippet| snippet.cursor == cursor)
                    };
                    if is_current {
                        self.status = if finished {
                            "Печать завершена".into()
                        } else {
                            "Печать…".into()
                        };
                        self.mark_dirty();
                    }
                }
            }
        }

        let hotkey_events = self.hotkey_events.try_iter().collect::<Vec<_>>();
        for event in hotkey_events {
            if event.state() != HotKeyState::Pressed {
                continue;
            }
            let task_index = self
                .task_hotkeys
                .iter()
                .find(|(_, key)| key.id() == event.id())
                .map(|(task_index, _)| *task_index);
            if let Some(task_index) = task_index {
                self.handle_demo_task_hotkey(task_index);
                continue;
            }
            if let Some((action, _)) = self
                .registered_hotkeys
                .iter()
                .find(|(_, key)| key.id() == event.id())
            {
                self.handle_hotkey(*action);
            }
        }

        let room_events = self
            .room_client
            .as_ref()
            .map(|client| client.events().try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for event in room_events {
            match event {
                RoomClientEvent::Connection {
                    connected,
                    message,
                    participant_id,
                } => {
                    self.room_connected = connected;
                    self.room_status = message.clone();
                    self.status = message;
                    if let Some(participant_id) = participant_id {
                        let mut library = self
                            .library
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner());
                        library.room.participant_id = Some(participant_id);
                        drop(library);
                        self.mark_dirty();
                    }
                }
                RoomClientEvent::Assignment(task) => self.apply_remote_assignment(task),
                RoomClientEvent::Command(command) => self.apply_room_command(&command),
            }
        }

        let wide_layout = root.available_width() >= 780.0;
        let mut open_settings = false;
        let mut open_room = false;
        let mut open_help = false;
        let library_arc = Arc::clone(&self.library);
        let mut library = library_arc
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.sync_editor(&library);

        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(GROUND)
                    .stroke(egui::Stroke::new(1.0, RULE))
                    .inner_margin(14.0),
            )
            .show(root, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("›").color(MINT).size(26.0).strong());
                        ui.vertical(|ui| {
                            ui.label(
                                egui::RichText::new("LiveCode Typer · DEMO")
                                    .size(16.0)
                                    .strong(),
                            );
                            ui.label(
                                egui::RichText::new(
                                    "Alt+1–7 · задания    |    Мышь 4 — вперёд, 5 — назад",
                                )
                                .color(FAINT)
                                .size(12.0),
                            );
                        });
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if wide_layout {
                            if ui.button("Справка").clicked() {
                                open_help = true;
                            }
                            if ui.button("Горячие клавиши").clicked() {
                                open_settings = true;
                            }
                            if ui.button("Комната").clicked() {
                                open_room = true;
                            }
                        } else {
                            ui.menu_button("Меню", |menu| {
                                if menu.button("Как пользоваться").clicked() {
                                    open_help = true;
                                    menu.close();
                                }
                                if menu.button("Горячие клавиши").clicked() {
                                    open_settings = true;
                                    menu.close();
                                }
                                if menu.button("Онлайн-комната").clicked() {
                                    open_room = true;
                                    menu.close();
                                }
                            });
                        }
                        ui.label(
                            egui::RichText::new(if library.typing_state.is_active() {
                                "Печать включена"
                            } else {
                                "Печать выключена"
                            })
                            .color(
                                if library.typing_state.is_active() {
                                    MINT
                                } else {
                                    MUTED
                                },
                            ),
                        );
                    });
                });
            });

        self.render_footer(root, library.typing_state.is_active(), &library.hotkeys);

        if wide_layout {
            egui::Panel::left("snippet-library")
                .resizable(true)
                .default_size(232.0)
                .size_range(196.0..=320.0)
                .frame(
                    egui::Frame::new()
                        .fill(GROUND)
                        .stroke(egui::Stroke::new(1.0, RULE))
                        .inner_margin(16.0),
                )
                .show(root, |ui| self.render_sidebar(ui, &mut library));
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(GROUND)
                    .inner_margin(if wide_layout { 20.0 } else { 12.0 }),
            )
            .show(root, |ui| {
                if !wide_layout {
                    ui.horizontal_wrapped(|ui| {
                        let current = library
                            .selected()
                            .map(|snippet| snippet.name.clone())
                            .unwrap_or_else(|| "Выбери фрагмент".into());
                        egui::ComboBox::from_id_salt("narrow-snippet-picker")
                            .selected_text(current)
                            .width((ui.available_width() - 92.0).max(150.0))
                            .show_ui(ui, |ui| {
                                let items = library
                                    .snippets
                                    .iter()
                                    .map(|item| (item.id, item.name.clone()))
                                    .collect::<Vec<_>>();
                                for (id, name) in items {
                                    if ui
                                        .selectable_label(library.selected_id == id, name)
                                        .clicked()
                                    {
                                        library.selected_id = id;
                                        library.pause();
                                        self.sync_editor(&library);
                                        self.mark_dirty();
                                    }
                                }
                            });
                        if ui.button("Новый").clicked() {
                            library.add_snippet();
                            self.sync_editor(&library);
                            self.status = "Создан новый фрагмент".into();
                            self.mark_dirty();
                        }
                    });
                    ui.add_space(12.0);
                }
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.render_editor(ui, &mut library));
            });

        if library.typing_state.is_active() {
            context.request_repaint_after(Duration::from_millis(25));
        }
        drop(library);
        if open_settings {
            self.open_settings();
        }
        if open_room {
            self.open_room();
        }
        if open_help {
            self.help_open = true;
        }
        self.render_settings(&context);
        self.render_room(&context);
        self.save_if_due();
        if self.dirty {
            let until_save = self
                .save_at
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
                .unwrap_or(Duration::ZERO);
            context.request_repaint_after(until_save.max(Duration::from_millis(1)));
        }
        if self.room_client.is_some() {
            context.request_repaint_after(Duration::from_millis(100));
        }
    }
}

fn editor_state(library: &Library) -> (Option<Uuid>, u64, String, String, String) {
    library.selected().map_or_else(
        || (None, 0, String::new(), String::new(), String::new()),
        |snippet| {
            (
                Some(snippet.id),
                snippet.revision,
                snippet.name.clone(),
                snippet.language.clone(),
                snippet.code.clone(),
            )
        },
    )
}

fn parse_hotkeys(hotkeys: &Hotkeys) -> Result<Vec<(HotkeyAction, HotKey)>, String> {
    let entries = [
        (HotkeyAction::ToggleTyping, hotkeys.toggle_typing.as_str()),
        (HotkeyAction::NextSnippet, hotkeys.next_snippet.as_str()),
        (
            HotkeyAction::PreviousSnippet,
            hotkeys.previous_snippet.as_str(),
        ),
        (HotkeyAction::Restart, hotkeys.restart.as_str()),
        (HotkeyAction::TogglePin, hotkeys.toggle_pin.as_str()),
        (HotkeyAction::EmergencyStop, hotkeys.emergency_stop.as_str()),
    ];
    let mut seen = HashSet::new();
    let mut bindings = Vec::new();
    for (action, text) in entries {
        let parts = text.split('+').map(str::trim).collect::<Vec<_>>();
        let is_windows_alt_space = cfg!(windows)
            && parts.len() == 2
            && parts.iter().any(|part| part.eq_ignore_ascii_case("alt"))
            && parts.iter().any(|part| part.eq_ignore_ascii_case("space"));
        if is_windows_alt_space {
            return Err(
                "Windows резервирует Alt+Space для системного меню. Выберите Ctrl+Alt+Space или другое сочетание.".into(),
            );
        }
        let key = HotKey::from_str(text)
            .map_err(|error| format!("Сочетание «{text}» не распознано: {error}"))?;
        if !seen.insert(key.id()) {
            return Err(format!(
                "Сочетание «{text}» назначено нескольким действиям."
            ));
        }
        bindings.push((action, key));
    }
    Ok(bindings)
}

fn parse_demo_task_hotkeys() -> Result<Vec<(usize, HotKey)>, String> {
    (1..=DEMO_TASK_COUNT)
        .map(|task_number| {
            let text = demo_task_hotkey(task_number)
                .ok_or_else(|| format!("Не удалось создать клавишу задания №{task_number}."))?;
            let key = HotKey::from_str(&text)
                .map_err(|error| format!("Сочетание «{text}» не распознано: {error}"))?;
            Ok((task_number - 1, key))
        })
        .collect()
}

fn set_hotkey_value(hotkeys: &mut Hotkeys, action: HotkeyAction, value: String) {
    match action {
        HotkeyAction::ToggleTyping => hotkeys.toggle_typing = value,
        HotkeyAction::NextSnippet => hotkeys.next_snippet = value,
        HotkeyAction::PreviousSnippet => hotkeys.previous_snippet = value,
        HotkeyAction::Restart => hotkeys.restart = value,
        HotkeyAction::TogglePin => hotkeys.toggle_pin = value,
        HotkeyAction::EmergencyStop => hotkeys.emergency_stop = value,
    }
}

fn egui_key_name(key: egui::Key) -> Option<&'static str> {
    use egui::Key;
    Some(match key {
        Key::ArrowDown => "ArrowDown",
        Key::ArrowLeft => "ArrowLeft",
        Key::ArrowRight => "ArrowRight",
        Key::ArrowUp => "ArrowUp",
        Key::Escape => "Escape",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Minus => "Minus",
        Key::Plus => "Equal",
        Key::Equals => "Equal",
        Key::OpenBracket => "BracketLeft",
        Key::CloseBracket => "BracketRight",
        Key::Semicolon => "Semicolon",
        Key::Quote => "Quote",
        Key::Backslash => "Backslash",
        Key::Comma => "Comma",
        Key::Period => "Period",
        Key::Slash => "Slash",
        Key::Backtick => "Backquote",
        _ => return None,
    })
}

impl Drop for PortableApp {
    fn drop(&mut self) {
        let library = self
            .library
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = storage::save(&library);
    }
}
