use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSettings {
    #[serde(default = "default_server_url")]
    pub server_url: String,
    #[serde(default)]
    pub room_code: String,
    #[serde(default)]
    pub participant_name: String,
    #[serde(default)]
    pub participant_id: Option<String>,
}

impl Default for RoomSettings {
    fn default() -> Self {
        Self {
            server_url: default_server_url(),
            room_code: String::new(),
            participant_name: String::new(),
            participant_id: None,
        }
    }
}

fn default_server_url() -> String {
    "http://localhost:5080".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomState {
    #[serde(default, alias = "RoomCode")]
    pub room_code: String,
    #[serde(default, alias = "Version")]
    pub version: u64,
    #[serde(default, alias = "Tasks")]
    pub tasks: Vec<RoomTask>,
    #[serde(default, alias = "Participants")]
    pub participants: Vec<RoomParticipant>,
    #[serde(default, alias = "Actions")]
    pub actions: Vec<RoomAction>,
}

impl RoomState {
    pub fn new(room_code: String) -> Self {
        Self {
            room_code,
            version: 0,
            tasks: Vec::new(),
            participants: Vec::new(),
            actions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomTask {
    #[serde(default, alias = "Id")]
    pub id: String,
    #[serde(default, alias = "Number")]
    pub number: String,
    #[serde(default, alias = "Title")]
    pub title: String,
    #[serde(default = "default_language", alias = "Language")]
    pub language: String,
    #[serde(default, alias = "Code")]
    pub code: String,
    #[serde(
        default = "now",
        alias = "UpdatedAtUtc",
        deserialize_with = "deserialize_utc_datetime"
    )]
    pub updated_at_utc: DateTime<Utc>,
    #[serde(default, alias = "IsQuickSend")]
    pub is_quick_send: bool,
    #[serde(default, alias = "QuickSendForParticipantId")]
    pub quick_send_for_participant_id: Option<String>,
}

fn default_language() -> String {
    "C#".into()
}

fn now() -> DateTime<Utc> {
    Utc::now()
}

fn deserialize_utc_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if let Ok(value) = DateTime::parse_from_rfc3339(&value) {
        return Ok(value.with_timezone(&Utc));
    }
    NaiveDateTime::parse_from_str(&value, "%Y-%m-%dT%H:%M:%S%.f")
        .or_else(|_| NaiveDateTime::parse_from_str(&value, "%Y-%m-%d %H:%M:%S%.f"))
        .map(|value| Utc.from_utc_datetime(&value))
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomParticipant {
    #[serde(default, alias = "Id")]
    pub id: String,
    #[serde(default, alias = "Name")]
    pub name: String,
    #[serde(
        default = "now",
        alias = "LastSeenUtc",
        deserialize_with = "deserialize_utc_datetime"
    )]
    pub last_seen_utc: DateTime<Utc>,
    #[serde(default, alias = "AssignedTaskId")]
    pub assigned_task_id: Option<String>,
    #[serde(default, alias = "AssignmentRevision")]
    pub assignment_revision: u64,
    #[serde(default, alias = "CommandVersion")]
    pub command_version: u64,
    #[serde(default, alias = "LastCommand")]
    pub last_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomAction {
    #[serde(default, alias = "ActorName")]
    pub actor_name: String,
    #[serde(default, alias = "Action")]
    pub action: String,
    #[serde(default, alias = "TargetName")]
    pub target_name: String,
    #[serde(default, alias = "Details")]
    pub details: String,
    #[serde(
        default = "now",
        alias = "CreatedAtUtc",
        deserialize_with = "deserialize_utc_datetime"
    )]
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoomTaskView {
    pub id: String,
    pub number: String,
    pub title: String,
    pub language: String,
    pub code: String,
    pub updated_at_utc: DateTime<Utc>,
    #[serde(default)]
    pub is_quick_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParticipantView {
    pub id: String,
    pub name: String,
    pub is_online: bool,
    pub assigned_task_id: Option<String>,
    pub assigned_task_label: Option<String>,
    pub assignment_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomActionView {
    pub actor_name: String,
    pub action: String,
    pub target_name: String,
    pub details: String,
    pub created_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSnapshot {
    pub version: u64,
    pub tasks: Vec<RoomTaskView>,
    pub participants: Vec<ParticipantView>,
    pub actions: Vec<RoomActionView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRequest {
    pub room_code: String,
    pub name: String,
    pub participant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinResponse {
    pub participant_id: String,
    pub assignment_revision: u64,
    pub command_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollRequest {
    pub room_code: String,
    pub participant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPollResponse {
    pub participant_id: String,
    pub name: String,
    pub assignment_revision: u64,
    pub assigned_task: Option<RoomTaskView>,
    pub command_version: u64,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskUpsertRequest {
    pub room_code: String,
    pub actor_name: String,
    pub id: Option<String>,
    pub number: String,
    pub title: String,
    pub language: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickSendRequest {
    pub room_code: String,
    pub actor_name: String,
    pub participant_id: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignRequest {
    pub room_code: String,
    pub actor_name: String,
    pub participant_id: String,
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    pub room_code: String,
    pub actor_name: String,
    pub participant_id: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaveRequest {
    pub room_code: String,
    pub participant_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTaskRequest {
    pub room_code: String,
    pub actor_name: String,
}
