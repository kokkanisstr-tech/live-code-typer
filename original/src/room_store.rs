use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::Rng;
use tempfile::NamedTempFile;
use uuid::Uuid;

use crate::room::{
    AssignRequest, ClientPollResponse, CommandRequest, DeleteTaskRequest, JoinRequest,
    JoinResponse, LeaveRequest, ParticipantView, PollRequest, QuickSendRequest, RoomAction,
    RoomActionView, RoomParticipant, RoomSnapshot, RoomState, RoomTask, RoomTaskView,
    TaskUpsertRequest,
};

const MAX_ACTIONS: usize = 60;
const ONLINE_WINDOW: Duration = Duration::from_secs(20);
const MAX_CODE_BYTES: usize = 1_000_000;

pub struct RoomStore {
    file_path: PathBuf,
    state: Mutex<RoomState>,
}

#[derive(Debug)]
pub enum RoomError {
    Validation(String),
    Storage(anyhow::Error),
}

impl RoomStore {
    pub fn load(data_folder: &Path, configured_room_code: Option<&str>) -> Result<Self> {
        fs::create_dir_all(data_folder)
            .with_context(|| format!("Не удалось создать {}", data_folder.display()))?;
        let file_path = data_folder.join("room.json");
        let mut state = if file_path.exists() {
            let bytes = fs::read(&file_path)
                .with_context(|| format!("Не удалось прочитать {}", file_path.display()))?;
            serde_json::from_slice::<RoomState>(&bytes)
                .with_context(|| format!("Повреждён файл комнаты {}", file_path.display()))?
        } else if let Some(legacy_path) = legacy_room_file_path().filter(|path| path.is_file()) {
            let bytes = fs::read(&legacy_path).with_context(|| {
                format!(
                    "Не удалось прочитать старую комнату {}",
                    legacy_path.display()
                )
            })?;
            serde_json::from_slice::<RoomState>(&bytes).with_context(|| {
                format!(
                    "Не удалось перенести старую комнату {}",
                    legacy_path.display()
                )
            })?
        } else {
            RoomState::new(String::new())
        };

        if let Some(code) = configured_room_code.filter(|code| !code.trim().is_empty()) {
            state.room_code = code.trim().to_owned();
        } else if state.room_code.trim().is_empty() {
            state.room_code = rand::rng().random_range(100_000..1_000_000).to_string();
        }

        let expired = DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now);
        for participant in &mut state.participants {
            participant.last_seen_utc = expired;
        }
        save_state(&file_path, &state)?;
        Ok(Self {
            file_path,
            state: Mutex::new(state),
        })
    }

    pub fn room_code(&self) -> String {
        self.lock().room_code.clone()
    }

    pub fn is_room_code_valid(&self, room_code: &str) -> bool {
        self.lock().room_code.eq_ignore_ascii_case(room_code.trim())
    }

    pub fn snapshot(&self) -> RoomSnapshot {
        let state = self.lock();
        let tasks = state
            .tasks
            .iter()
            .filter(|task| !task.is_quick_send)
            .map(task_view)
            .collect::<Vec<_>>();
        let participants = state
            .participants
            .iter()
            .map(|participant| {
                let assigned_task_label = participant.assigned_task_id.as_ref().and_then(|id| {
                    state
                        .tasks
                        .iter()
                        .find(|task| task.id == *id)
                        .map(|task| format!("{} · {}", task.number, task.title))
                });
                ParticipantView {
                    id: participant.id.clone(),
                    name: participant.name.clone(),
                    is_online: (Utc::now() - participant.last_seen_utc)
                        .to_std()
                        .is_ok_and(|age| age <= ONLINE_WINDOW),
                    assigned_task_id: participant.assigned_task_id.clone(),
                    assigned_task_label,
                    assignment_revision: participant.assignment_revision,
                }
            })
            .collect::<Vec<_>>();
        let mut participants = participants;
        participants.sort_by_key(|item| item.name.to_lowercase());
        let mut actions = state
            .actions
            .iter()
            .map(|action| RoomActionView {
                actor_name: action.actor_name.clone(),
                action: action.action.clone(),
                target_name: action.target_name.clone(),
                details: action.details.clone(),
                created_at_utc: action.created_at_utc,
            })
            .collect::<Vec<_>>();
        actions.sort_by_key(|item| std::cmp::Reverse(item.created_at_utc));
        RoomSnapshot {
            version: state.version,
            tasks,
            participants,
            actions,
        }
    }

    pub fn join(&self, request: JoinRequest) -> Result<JoinResponse, RoomError> {
        self.update(|state| {
            let clean_name = clean_required(&request.name, 40, "Введите имя участника.")?;
            let participant = request
                .participant_id
                .as_deref()
                .filter(|id| !id.trim().is_empty())
                .and_then(|id| state.participants.iter_mut().find(|item| item.id == id));
            let participant = if let Some(participant) = participant {
                participant
            } else {
                state.participants.push(RoomParticipant {
                    id: Uuid::new_v4().simple().to_string(),
                    name: String::new(),
                    last_seen_utc: Utc::now(),
                    assigned_task_id: None,
                    assignment_revision: 0,
                    command_version: 0,
                    last_command: None,
                });
                state
                    .participants
                    .last_mut()
                    .expect("participant was just added")
            };
            participant.name = clean_name.clone();
            participant.last_seen_utc = Utc::now();
            let response = JoinResponse {
                participant_id: participant.id.clone(),
                assignment_revision: participant.assignment_revision,
                command_version: participant.command_version,
            };
            add_action(
                state,
                "Вход",
                &clean_name,
                "Комната",
                "Подключился к комнате.",
            );
            state.version = state.version.saturating_add(1);
            Ok(response)
        })
    }

    pub fn poll(&self, request: &PollRequest) -> Option<ClientPollResponse> {
        let mut state = self.lock();
        let index = state
            .participants
            .iter()
            .position(|item| item.id == request.participant_id)?;
        state.participants[index].last_seen_utc = Utc::now();
        let participant = &state.participants[index];
        let assigned_task = participant
            .assigned_task_id
            .as_ref()
            .and_then(|id| state.tasks.iter().find(|task| task.id == *id))
            .map(|task| task_view(task));
        Some(ClientPollResponse {
            participant_id: participant.id.clone(),
            name: participant.name.clone(),
            assignment_revision: participant.assignment_revision,
            assigned_task,
            command_version: participant.command_version,
            command: participant.last_command.clone(),
        })
    }

    pub fn leave(&self, request: LeaveRequest) -> Result<(), RoomError> {
        let known = self
            .lock()
            .participants
            .iter()
            .any(|item| item.id == request.participant_id);
        if !known {
            return Ok(());
        }
        self.update(|state| {
            if let Some(participant) = state
                .participants
                .iter_mut()
                .find(|item| item.id == request.participant_id)
            {
                participant.last_seen_utc =
                    DateTime::<Utc>::from_timestamp(0, 0).unwrap_or_else(Utc::now);
                let name = participant.name.clone();
                add_action(state, "Выход", &name, "Комната", "Отключился от комнаты.");
                state.version = state.version.saturating_add(1);
            }
            Ok(())
        })
    }

    pub fn upsert_task(&self, request: TaskUpsertRequest) -> Result<RoomTaskView, RoomError> {
        self.update(|state| {
            let number = clean_required(&request.number, 20, "Укажите номер задания.")?;
            let title = clean_required(&request.title, 100, "Укажите название задания.")?;
            let language = clean_required(&request.language, 30, "Укажите язык задания.")?;
            if request.code.len() > MAX_CODE_BYTES {
                return Err("Код задания должен быть не длиннее 1 МБ.".into());
            }
            let existing = request
                .id
                .as_deref()
                .and_then(|id| state.tasks.iter().position(|task| task.id == id));
            let action = if existing.is_some() {
                "Изменил задание"
            } else {
                "Добавил задание"
            };
            let mut task = existing
                .map(|index| state.tasks.remove(index))
                .unwrap_or_else(|| RoomTask {
                    id: Uuid::new_v4().simple().to_string(),
                    number: String::new(),
                    title: String::new(),
                    language: "C#".into(),
                    code: String::new(),
                    updated_at_utc: Utc::now(),
                    is_quick_send: false,
                    quick_send_for_participant_id: None,
                });
            task.number = number.clone();
            task.title = title.clone();
            task.language = language;
            task.code = request.code;
            task.updated_at_utc = Utc::now();
            let response = task_view(&task);
            state.tasks.push(task);
            add_action(
                state,
                action,
                &clean_actor(&request.actor_name),
                &format!("№{number}"),
                &title,
            );
            state.version = state.version.saturating_add(1);
            Ok(response)
        })
    }

    pub fn quick_send(&self, request: QuickSendRequest) -> Result<RoomTaskView, RoomError> {
        self.update(|state| {
            let text = request.text.unwrap_or_default();
            if text.is_empty() {
                return Err("Вставьте текст для быстрой отправки.".into());
            }
            if text.len() > MAX_CODE_BYTES {
                return Err("Текст должен быть не длиннее 1 МБ.".into());
            }
            let participant_index = state
                .participants
                .iter()
                .position(|item| item.id == request.participant_id)
                .ok_or_else(|| "Участник не найден.".to_owned())?;
            let participant_name = state.participants[participant_index].name.clone();
            let task_index = state.tasks.iter().position(|item| {
                item.is_quick_send
                    && item.quick_send_for_participant_id.as_deref()
                        == Some(request.participant_id.as_str())
            });
            let mut task = task_index
                .map(|index| state.tasks.remove(index))
                .unwrap_or_else(|| RoomTask {
                    id: Uuid::new_v4().simple().to_string(),
                    number: "ТЕКСТ".into(),
                    title: "Быстрое сообщение".into(),
                    language: "Текст".into(),
                    code: String::new(),
                    updated_at_utc: Utc::now(),
                    is_quick_send: true,
                    quick_send_for_participant_id: Some(request.participant_id.clone()),
                });
            task.code = text;
            task.updated_at_utc = Utc::now();
            let response = task_view(&task);
            let text_length = task.code.chars().count();
            state.tasks.push(task.clone());
            let participant = &mut state.participants[participant_index];
            participant.assigned_task_id = Some(task.id);
            participant.assignment_revision = participant.assignment_revision.saturating_add(1);
            add_action(
                state,
                "Отправил быстрый текст",
                &clean_actor(&request.actor_name),
                &participant_name,
                &format!("{text_length} символов"),
            );
            state.version = state.version.saturating_add(1);
            Ok(response)
        })
    }

    pub fn delete_task(&self, id: &str, request: DeleteTaskRequest) -> Result<(), RoomError> {
        self.update(|state| {
            let index = state
                .tasks
                .iter()
                .position(|item| item.id == id)
                .ok_or_else(|| "Такое задание уже удалено.".to_owned())?;
            let task = state.tasks.remove(index);
            for participant in state
                .participants
                .iter_mut()
                .filter(|item| item.assigned_task_id.as_deref() == Some(id))
            {
                participant.assigned_task_id = None;
                participant.assignment_revision = participant.assignment_revision.saturating_add(1);
            }
            add_action(
                state,
                "Удалил задание",
                &clean_actor(&request.actor_name),
                &format!("№{}", task.number),
                &task.title,
            );
            state.version = state.version.saturating_add(1);
            Ok(())
        })
    }

    pub fn assign_task(&self, request: AssignRequest) -> Result<(), RoomError> {
        self.update(|state| {
            let task = state
                .tasks
                .iter()
                .find(|item| item.id == request.task_id)
                .cloned()
                .ok_or_else(|| "Выбранное задание не найдено.".to_owned())?;
            let participant = state
                .participants
                .iter_mut()
                .find(|item| item.id == request.participant_id)
                .ok_or_else(|| "Участник не найден.".to_owned())?;
            let participant_name = participant.name.clone();
            participant.assigned_task_id = Some(task.id);
            participant.assignment_revision = participant.assignment_revision.saturating_add(1);
            add_action(
                state,
                "Назначил задание",
                &clean_actor(&request.actor_name),
                &participant_name,
                &format!("№{} · {}", task.number, task.title),
            );
            state.version = state.version.saturating_add(1);
            Ok(())
        })
    }

    pub fn send_command(&self, request: CommandRequest) -> Result<(), RoomError> {
        self.update(|state| {
            let command = request.command.trim().to_lowercase();
            if command != "pause" && command != "restart" {
                return Err("Поддерживаются команды паузы и перезапуска.".into());
            }
            let participant = state
                .participants
                .iter_mut()
                .find(|item| item.id == request.participant_id)
                .ok_or_else(|| "Участник не найден.".to_owned())?;
            let participant_name = participant.name.clone();
            participant.last_command = Some(command.clone());
            participant.command_version = participant.command_version.saturating_add(1);
            let (action, details) = if command == "pause" {
                ("Поставил на паузу", "Команда паузы.")
            } else {
                ("Сбросил позицию", "Команда начать сначала.")
            };
            add_action(
                state,
                action,
                &clean_actor(&request.actor_name),
                &participant_name,
                details,
            );
            state.version = state.version.saturating_add(1);
            Ok(())
        })
    }

    fn update<T>(
        &self,
        update: impl FnOnce(&mut RoomState) -> std::result::Result<T, String>,
    ) -> std::result::Result<T, RoomError> {
        let mut state = self.lock();
        let mut candidate = state.clone();
        let result = update(&mut candidate).map_err(RoomError::Validation)?;
        save_state(&self.file_path, &candidate).map_err(RoomError::Storage)?;
        *state = candidate;
        Ok(result)
    }

    fn lock(&self) -> MutexGuard<'_, RoomState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn save_state(path: &Path, state: &RoomState) -> Result<()> {
    let folder = path.parent().context("Нет каталога данных комнаты")?;
    fs::create_dir_all(folder)
        .with_context(|| format!("Не удалось создать {}", folder.display()))?;
    let bytes = serde_json::to_vec_pretty(state).context("Не удалось сериализовать комнату")?;
    let mut temporary = NamedTempFile::new_in(folder)
        .with_context(|| format!("Не удалось создать временный файл в {}", folder.display()))?;
    temporary
        .write_all(&bytes)
        .context("Не удалось записать комнату")?;
    temporary
        .as_file()
        .sync_all()
        .context("Не удалось синхронизировать файл комнаты")?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("Не удалось сохранить {}", path.display()))?;
    Ok(())
}

fn legacy_room_file_path() -> Option<PathBuf> {
    if let Some(folder) = std::env::var_os("LIVECODETYPER_LEGACY_ROOM_DATA_FOLDER") {
        return Some(PathBuf::from(folder).join("room.json"));
    }

    #[cfg(target_os = "windows")]
    return std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|folder| {
            folder
                .join("LiveCodeTyper")
                .join("Server")
                .join("room.json")
        });

    #[cfg(not(target_os = "windows"))]
    None
}

fn task_view(task: &RoomTask) -> RoomTaskView {
    RoomTaskView {
        id: task.id.clone(),
        number: task.number.clone(),
        title: task.title.clone(),
        language: task.language.clone(),
        code: task.code.clone(),
        updated_at_utc: task.updated_at_utc,
        is_quick_send: task.is_quick_send,
    }
}

fn add_action(
    state: &mut RoomState,
    action: &str,
    actor_name: &str,
    target_name: &str,
    details: &str,
) {
    state.actions.push(RoomAction {
        actor_name: actor_name.to_owned(),
        action: action.to_owned(),
        target_name: target_name.to_owned(),
        details: details.to_owned(),
        created_at_utc: Utc::now(),
    });
    if state.actions.len() > MAX_ACTIONS {
        let remove_count = state.actions.len() - MAX_ACTIONS;
        state.actions.drain(..remove_count);
    }
}

fn clean_required(value: &str, max_length: usize, message: &str) -> Result<String, String> {
    let clean = value.trim();
    if clean.is_empty() {
        return Err(message.into());
    }
    if clean.chars().count() > max_length {
        return Err(format!(
            "Значение не должно превышать {max_length} символов."
        ));
    }
    Ok(clean.to_owned())
}

fn clean_actor(value: &str) -> String {
    let actor = value.trim();
    if actor.is_empty() {
        "Участник".into()
    } else {
        actor.chars().take(40).collect()
    }
}
