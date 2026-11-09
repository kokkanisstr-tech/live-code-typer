#![allow(dead_code)]

#[path = "../src/core.rs"]
mod core;
#[path = "../src/input.rs"]
mod input;
#[path = "../src/room.rs"]
mod room;
#[path = "../src/room_client.rs"]
mod room_client;

use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use core::{InputAction, Library, ROOM_ASSIGNMENT_START_DELAY};
use input::{CursorCache, InputEvent, InputMode, TypeResult};
use room::RoomSettings;
use room_client::{RoomClient, RoomClientEvent};

fn main() -> Result<(), String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 4 {
        return Err("Expected server URL, room code, participant name, and participant ID.".into());
    }
    let settings = RoomSettings {
        server_url: arguments[0].clone(),
        room_code: arguments[1].clone(),
        participant_name: arguments[2].clone(),
        participant_id: Some(arguments[3].clone()),
    };
    let client = RoomClient::connect(settings.clone());
    let deadline = Instant::now() + Duration::from_secs(12);
    let task = loop {
        if Instant::now() >= deadline {
            return Err("The connected room client did not receive an assignment in time.".into());
        }
        let mut received = None;
        for event in client.events().try_iter() {
            match event {
                RoomClientEvent::Assignment(Some(task)) => received = Some(task),
                RoomClientEvent::Connection {
                    connected: false,
                    message,
                    ..
                } => return Err(format!("Room client failed to connect: {message}")),
                _ => {}
            }
        }
        if let Some(task) = received {
            break task;
        }
        thread::sleep(Duration::from_millis(20));
    };

    let mut library = Library::default();
    library.room = settings;
    let start_result = library.apply_remote_assignment(Some(&task), ROOM_ASSIGNMENT_START_DELAY);
    if start_result != Some(InputAction::Started) || !library.typing_state.is_automatic() {
        return Err("Receiving the assignment did not enable automatic typing.".into());
    }

    let shared_library = Arc::new(Mutex::new(library));
    let mut cache = CursorCache::default();
    let mut output = String::new();
    {
        let mut sink = |text: &str| {
            output.push_str(text);
            Ok(())
        };
        match input::type_next(
            &mut sink,
            &shared_library,
            &mut cache,
            InputMode::KeyboardSignal,
        )
        .map_err(|error| format!("Random-key suppression failed: {error}"))?
        {
            TypeResult::ConsumeSignal => {}
            _ => {
                return Err(
                    "A random key was not consumed while a room task was auto-typing.".into(),
                );
            }
        }
    }
    if !output.is_empty() {
        return Err("A random key wrote text before the automatic typing started.".into());
    }

    let mut sink = |text: &str| {
        output.push_str(text);
        Ok(())
    };
    match input::type_next(&mut sink, &shared_library, &mut cache, InputMode::Automatic)
        .map_err(|error| format!("Automatic countdown failed: {error}"))?
    {
        TypeResult::Idle => {}
        _ => return Err("The assignment started before the editor-switch delay elapsed.".into()),
    }

    let start_at = match shared_library
        .lock()
        .map_err(|_| "Typing state lock was poisoned.".to_owned())?
        .typing_state
    {
        live_code_typer::core::TypingState::WaitingUntil(deadline) => deadline,
        _ => return Err("The assignment has no automatic start time.".into()),
    };
    thread::sleep(start_at.saturating_duration_since(Instant::now()));
    loop {
        match input::type_next(&mut sink, &shared_library, &mut cache, InputMode::Automatic)
            .map_err(|error| format!("Automatic typing failed: {error}"))?
        {
            TypeResult::Progress(InputEvent::Progress { .. }) => {}
            TypeResult::Idle => break,
            TypeResult::ConsumeSignal => {
                return Err("Automatic typing returned a keyboard-only result.".into());
            }
            TypeResult::Progress(_) => {
                return Err("Unexpected input event from the typing core.".into());
            }
        }
    }
    drop(sink);

    let expected = task.code.replace("\r\n", "\n").replace('\r', "\n");
    if output != expected {
        return Err(format!(
            "The text sent by the room does not match the simulated editor output. Expected {:?}, got {:?}.",
            expected, output
        ));
    }
    let final_state = shared_library
        .lock()
        .map_err(|_| "Typing state lock was poisoned.".to_owned())?;
    if final_state.typing_state.is_active()
        || final_state
            .selected()
            .is_none_or(|snippet| snippet.cursor != snippet.grapheme_count() || !snippet.pinned)
    {
        return Err("Automatic typing did not finish at the end of the pinned assignment.".into());
    }

    println!(
        "PASS: room assignment auto-started; random keys were ignored; delayed Unicode and multiline text reached the simulated editor exactly."
    );
    Ok(())
}
