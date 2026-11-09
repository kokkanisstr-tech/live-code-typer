use std::{
    net::{IpAddr, UdpSocket},
    path::PathBuf,
    sync::Arc,
};

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post},
};
use directories::ProjectDirs;
use serde::Deserialize;

use live_code_typer::{
    room::{
        AssignRequest, ClientPollResponse, CommandRequest, DeleteTaskRequest, JoinRequest,
        JoinResponse, LeaveRequest, PollRequest, QuickSendRequest, RoomSnapshot, RoomTaskView,
        TaskUpsertRequest,
    },
    room_store::{RoomError, RoomStore},
};

const INDEX: &str = include_str!("../../web/index.html");
const APP_JS: &str = include_str!("../../web/app.js");
const STYLES: &str = include_str!("../../web/styles.css");
const INTER_FONT: &[u8] = include_bytes!("../../web/fonts/InterVariable.woff2");
const FONT_LICENSE: &str = include_str!("../../web/fonts/OFL.txt");

#[derive(Clone)]
struct AppState {
    room: Arc<RoomStore>,
    invite_bases: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RoomCodeQuery {
    room_code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteTaskQuery {
    room_code: String,
    actor_name: String,
}

#[derive(serde::Serialize)]
struct ApiErrorBody {
    error: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let migrate_only = std::env::args_os()
        .skip(1)
        .any(|argument| argument == "--migrate-data");
    let configured_port = std::env::var("LIVECODE_SERVER_PORT")
        .ok()
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(|_| anyhow::anyhow!("Некорректный номер порта сервера: {value}"))
        })
        .transpose()?;
    let requested_port = configured_port.unwrap_or(5080);
    let host = std::env::var("LIVECODE_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let data_folder = std::env::var_os("LIVECODE_ROOM_DATA_FOLDER")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.parent().map(|parent| parent.join("portable-data")))
        })
        .or_else(|| {
            ProjectDirs::from("org", "LiveCodeTyperDemo", "LiveCodeTyperDemo")
                .map(|dirs| dirs.config_dir().join("Server"))
        })
        .ok_or_else(|| anyhow::anyhow!("Не удалось определить каталог данных сервера"))?;
    let room = Arc::new(RoomStore::load(
        &data_folder,
        std::env::var("LIVECODE_ROOM_CODE").ok().as_deref(),
    )?);
    if migrate_only {
        return Ok(());
    }
    let listener = match tokio::net::TcpListener::bind((host.as_str(), requested_port)).await {
        Ok(listener) => listener,
        Err(error)
            if configured_port.is_none() && error.kind() == std::io::ErrorKind::AddrInUse =>
        {
            println!("Порт 5080 уже занят. Выбираю свободный порт автоматически.");
            tokio::net::TcpListener::bind((host.as_str(), 0))
                .await
                .map_err(|error| anyhow::anyhow!("Не удалось открыть сервер комнаты: {error}"))?
        }
        Err(error) => {
            return Err(anyhow::anyhow!(
                "Не удалось запустить сервер на {host}:{requested_port}: {error}. Закройте другой сервер комнаты и попробуйте ещё раз."
            ));
        }
    };
    let port = listener.local_addr()?.port();
    let invite_bases = invite_bases_for_host(&host, port);

    let app = Router::new()
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/app.js", get(app_js))
        .route("/styles.css", get(styles))
        .route("/fonts/InterVariable.woff2", get(inter_font))
        .route("/fonts/OFL.txt", get(font_license))
        .route("/api/info", get(info))
        .route("/api/room", get(get_room))
        .route("/api/join", post(join))
        .route("/api/poll", post(poll))
        .route("/api/leave", post(leave))
        .route("/api/tasks", post(upsert_task))
        .route("/api/tasks/{id}", delete(delete_task))
        .route("/api/quick-send", post(quick_send))
        .route("/api/assign", post(assign_task))
        .route("/api/command", post(send_command))
        .route("/health", get(health))
        .fallback(get(index))
        .layer(DefaultBodyLimit::max(8 * 1024 * 1024))
        .with_state(AppState {
            room: Arc::clone(&room),
            invite_bases: invite_bases.clone(),
        });

    println!("\nLiveCode Typer — сервер комнаты");
    println!("Порт: {port}");
    println!("Код комнаты: {}", room.room_code());
    if !invite_bases.is_empty() {
        println!("\nСсылки для участников в локальной сети:");
        for base in &invite_bases {
            println!("  {base}/?roomCode={}", room.room_code());
        }
    } else {
        println!("\nНе удалось определить ссылку для локальной сети.");
        println!("Узнайте IP ведущего и откройте http://IP:порт/?roomCode=код комнаты.");
    }
    println!(
        "Только на этом компьютере: http://localhost:{port}/?roomCode={}",
        room.room_code()
    );
    println!("Откройте ссылку из раздела выше на Windows или Mac.");
    println!(
        "Если подключение не проходит: подключите устройства к одной сети и разрешите сервер в частной сети Windows."
    );
    println!("У всех пользователей комнаты одинаковые права администратора.\n");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(INDEX)
}

async fn app_js() -> Response {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/javascript; charset=utf-8",
        )],
        APP_JS,
    )
        .into_response()
}

async fn styles() -> Response {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES,
    )
        .into_response()
}

async fn inter_font() -> Response {
    (
        [
            (axum::http::header::CONTENT_TYPE, "font/woff2"),
            (axum::http::header::CACHE_CONTROL, "public, max-age=86400"),
        ],
        INTER_FONT,
    )
        .into_response()
}

async fn font_license() -> Response {
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; charset=utf-8",
        )],
        FONT_LICENSE,
    )
        .into_response()
}

async fn info(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "LiveCode Typer Room",
        "requiresRoomCode": true,
        "roomInviteBases": state.invite_bases
    }))
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn get_room(
    State(state): State<AppState>,
    Query(query): Query<RoomCodeQuery>,
) -> Result<Json<RoomSnapshot>, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &query.room_code)?;
    Ok(Json(state.room.snapshot()))
}

async fn join(
    State(state): State<AppState>,
    Json(request): Json<JoinRequest>,
) -> Result<Json<JoinResponse>, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &request.room_code)?;
    state.room.join(request).map(Json).map_err(map_room_error)
}

async fn poll(
    State(state): State<AppState>,
    Json(request): Json<PollRequest>,
) -> Result<Json<ClientPollResponse>, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &request.room_code)?;
    state.room.poll(&request).map(Json).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ApiErrorBody {
                error: "Участник не найден.".into(),
            }),
        )
    })
}

async fn leave(
    State(state): State<AppState>,
    Json(request): Json<LeaveRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &request.room_code)?;
    state.room.leave(request).map_err(map_room_error)?;
    Ok(StatusCode::OK)
}

async fn upsert_task(
    State(state): State<AppState>,
    Json(request): Json<TaskUpsertRequest>,
) -> Result<Json<RoomTaskView>, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &request.room_code)?;
    state
        .room
        .upsert_task(request)
        .map(Json)
        .map_err(map_room_error)
}

async fn quick_send(
    State(state): State<AppState>,
    Json(request): Json<QuickSendRequest>,
) -> Result<Json<RoomTaskView>, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &request.room_code)?;
    state
        .room
        .quick_send(request)
        .map(Json)
        .map_err(map_room_error)
}

async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<DeleteTaskQuery>,
) -> Result<StatusCode, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &query.room_code)?;
    state
        .room
        .delete_task(
            &id,
            DeleteTaskRequest {
                room_code: query.room_code,
                actor_name: query.actor_name,
            },
        )
        .map_err(map_room_error)?;
    Ok(StatusCode::OK)
}

async fn assign_task(
    State(state): State<AppState>,
    Json(request): Json<AssignRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &request.room_code)?;
    state.room.assign_task(request).map_err(map_room_error)?;
    Ok(StatusCode::OK)
}

async fn send_command(
    State(state): State<AppState>,
    Json(request): Json<CommandRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiErrorBody>)> {
    check_room(&state.room, &request.room_code)?;
    state.room.send_command(request).map_err(map_room_error)?;
    Ok(StatusCode::OK)
}

fn check_room(room: &RoomStore, code: &str) -> Result<(), (StatusCode, Json<ApiErrorBody>)> {
    if room.is_room_code_valid(code) {
        Ok(())
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiErrorBody {
                error: "Неверный код комнаты.".into(),
            }),
        ))
    }
}

fn map_room_error(error: RoomError) -> (StatusCode, Json<ApiErrorBody>) {
    match error {
        RoomError::Validation(message) => (
            StatusCode::BAD_REQUEST,
            Json(ApiErrorBody { error: message }),
        ),
        RoomError::Storage(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiErrorBody {
                error: format!("Не удалось сохранить комнату: {error}"),
            }),
        ),
    }
}

fn lan_addresses() -> Vec<String> {
    let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
        return Vec::new();
    };
    if socket.connect("192.0.2.1:9").is_err() {
        return Vec::new();
    }
    socket
        .local_addr()
        .ok()
        .map(|address| address.ip().to_string())
        .filter(|address| address != "0.0.0.0" && address != "127.0.0.1")
        .into_iter()
        .collect()
}

fn invite_bases_for_host(host: &str, port: u16) -> Vec<String> {
    match host.parse::<IpAddr>() {
        Ok(address) if address.is_unspecified() => lan_addresses()
            .into_iter()
            .map(|address| format!("http://{address}:{port}"))
            .collect(),
        Ok(address) if !address.is_loopback() => vec![format!("http://{address}:{port}")],
        _ => Vec::new(),
    }
}
