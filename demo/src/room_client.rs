use std::{thread, time::Duration};

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use reqwest::blocking::{Client, Response};
use serde::de::DeserializeOwned;

use crate::room::{
    ClientPollResponse, JoinRequest, JoinResponse, LeaveRequest, PollRequest, RoomSettings,
    RoomTaskView,
};

#[derive(Debug, Clone)]
pub enum RoomClientEvent {
    Connection {
        connected: bool,
        message: String,
        participant_id: Option<String>,
    },
    Assignment(Option<RoomTaskView>),
    Command(String),
}

enum WorkerCommand {
    Stop,
}

pub struct RoomClient {
    commands: Sender<WorkerCommand>,
    events: Receiver<RoomClientEvent>,
    worker: Option<thread::JoinHandle<()>>,
}

impl RoomClient {
    pub fn connect(settings: RoomSettings) -> Self {
        let (commands, command_rx) = unbounded();
        let (event_tx, events) = unbounded();
        let worker = thread::Builder::new()
            .name("live-code-room-client".into())
            .spawn(move || room_worker(settings, command_rx, event_tx))
            .expect("Не удалось создать поток онлайн-комнаты");
        Self {
            commands,
            events,
            worker: Some(worker),
        }
    }

    pub fn events(&self) -> &Receiver<RoomClientEvent> {
        &self.events
    }
}

impl Drop for RoomClient {
    fn drop(&mut self) {
        let _ = self.commands.send(WorkerCommand::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn room_worker(
    settings: RoomSettings,
    commands: Receiver<WorkerCommand>,
    events: Sender<RoomClientEvent>,
) {
    let server_url = settings.server_url.trim().trim_end_matches('/').to_owned();
    if !(server_url.starts_with("http://") || server_url.starts_with("https://")) {
        send_connection(
            &events,
            false,
            "Адрес сервера должен начинаться с http:// или https://.",
            None,
        );
        return;
    }
    let client = match Client::builder().timeout(Duration::from_secs(3)).build() {
        Ok(client) => client,
        Err(error) => {
            send_connection(
                &events,
                false,
                &format!("Не удалось создать HTTP-клиент: {error}"),
                None,
            );
            return;
        }
    };
    let join_request = JoinRequest {
        room_code: settings.room_code.trim().to_owned(),
        name: settings.participant_name.trim().to_owned(),
        participant_id: settings.participant_id.clone(),
    };
    let join_url = format!("{server_url}/api/join");
    let joined = match post_json::<_, JoinResponse>(&client, &join_url, &join_request) {
        Ok(joined) => joined,
        Err(error) => {
            send_connection(&events, false, &error, None);
            return;
        }
    };
    let participant_id = joined.participant_id;
    send_connection(
        &events,
        true,
        "Подключено к комнате.",
        Some(participant_id.clone()),
    );

    let poll_url = format!("{server_url}/api/poll");
    let leave_url = format!("{server_url}/api/leave");
    let mut command_version = joined.command_version;
    let mut assignment_revision = None;
    let mut previous_assignment: Option<RoomTaskView> = None;
    let mut connected = true;
    loop {
        match commands.try_recv() {
            Ok(WorkerCommand::Stop) | Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }

        let request = PollRequest {
            room_code: settings.room_code.trim().to_owned(),
            participant_id: participant_id.clone(),
        };
        match post_json::<_, ClientPollResponse>(&client, &poll_url, &request) {
            Ok(state) => {
                if !connected {
                    send_connection(&events, true, "Связь с комнатой восстановлена.", None);
                    connected = true;
                }
                if assignment_revision != Some(state.assignment_revision)
                    || previous_assignment != state.assigned_task
                {
                    assignment_revision = Some(state.assignment_revision);
                    previous_assignment = state.assigned_task.clone();
                    let _ = events.send(RoomClientEvent::Assignment(state.assigned_task));
                }
                if state.command_version > command_version {
                    command_version = state.command_version;
                    if let Some(command) = state.command {
                        let _ = events.send(RoomClientEvent::Command(command));
                    }
                }
            }
            Err(error) => {
                if connected {
                    send_connection(
                        &events,
                        false,
                        &format!("Нет связи с сервером: {error}"),
                        None,
                    );
                    connected = false;
                }
            }
        }
        match commands.recv_timeout(Duration::from_millis(750)) {
            Ok(WorkerCommand::Stop) | Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                break;
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
        }
    }

    let _ = client
        .post(leave_url)
        .json(&LeaveRequest {
            room_code: settings.room_code,
            participant_id,
        })
        .send();
}

fn post_json<Request, ResponseBody>(
    client: &Client,
    url: &str,
    request: &Request,
) -> Result<ResponseBody, String>
where
    Request: serde::Serialize,
    ResponseBody: DeserializeOwned,
{
    let response = client
        .post(url)
        .json(request)
        .send()
        .map_err(|error| error.to_string())?;
    parse_response(response)
}

fn parse_response<ResponseBody: DeserializeOwned>(
    response: Response,
) -> Result<ResponseBody, String> {
    let status = response.status();
    if status.is_success() {
        return response
            .json::<ResponseBody>()
            .map_err(|error| format!("Сервер вернул неправильный ответ: {error}"));
    }
    let body = response.text().unwrap_or_default();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body)
        && let Some(message) = value.get("error").and_then(serde_json::Value::as_str)
    {
        return Err(message.to_owned());
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        Err("Неверный код комнаты.".into())
    } else {
        Err(format!("Ошибка сервера: HTTP {status}."))
    }
}

fn send_connection(
    events: &Sender<RoomClientEvent>,
    connected: bool,
    message: &str,
    participant_id: Option<String>,
) {
    let _ = events.send(RoomClientEvent::Connection {
        connected,
        message: message.to_owned(),
        participant_id,
    });
}
