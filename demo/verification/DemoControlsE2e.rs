#![allow(dead_code)]

#[path = "../src/core.rs"]
mod core;
#[path = "../src/room.rs"]
mod room;

use core::{
    DEMO_HOTKEY_COUNT, DEMO_TASK_COUNT, InputAction, Library, PREPARED_TASKS, demo_mouse_offset,
    demo_task_hotkey, navigate_demo_mouse,
};

fn main() -> Result<(), String> {
    let mut library = Library::default();
    if library.snippets.len() != DEMO_TASK_COUNT {
        return Err(format!(
            "Expected {DEMO_TASK_COUNT} prepared tasks, got {}.",
            library.snippets.len()
        ));
    }
    for index in 0..DEMO_TASK_COUNT {
        let snippet = &library.snippets[index];
        if snippet.name != format!("Задание {} · {}", index + 1, PREPARED_TASKS[index].0)
            || snippet.language != "C#"
            || !snippet.pinned
            || snippet.code != PREPARED_TASKS[index].1
        {
            return Err(format!(
                "Prepared task {} is missing or editable.",
                index + 1
            ));
        }
        if index < DEMO_HOTKEY_COUNT
            && demo_task_hotkey(index + 1).as_deref() != Some(format!("Alt+{}", index + 1).as_str())
        {
            return Err(format!("Alt+{} does not map to its task.", index + 1));
        }
        if index >= DEMO_HOTKEY_COUNT && demo_task_hotkey(index + 1).is_some() {
            return Err(format!(
                "Task {} unexpectedly has a numeric hotkey.",
                index + 1
            ));
        }
    }
    if demo_task_hotkey(0).is_some() || demo_task_hotkey(DEMO_HOTKEY_COUNT + 1).is_some() {
        return Err("Task shortcuts accept an out-of-range number.".into());
    }
    if demo_mouse_offset(4) != Some(1)
        || demo_mouse_offset(5) != Some(-1)
        || demo_mouse_offset(3).is_some()
        || demo_mouse_offset(6).is_some()
    {
        return Err("Mouse buttons 4 and 5 are mapped incorrectly.".into());
    }

    if navigate_demo_mouse(&mut library, 4) {
        return Err("Mouse navigation should not switch tasks while typing is inactive.".into());
    }
    if library
        .selected()
        .is_none_or(|snippet| !snippet.name.starts_with("Задание 1 · "))
    {
        return Err("Inactive mouse navigation changed the selected task.".into());
    }
    if library.select_and_start(2) != Some(InputAction::Started)
        || !library.typing_state.is_active()
    {
        return Err("Alt+3 did not select and start task 3.".into());
    }
    let third_task = library.selected_id;
    let selected = library.selected().ok_or("Task 3 was not selected.")?;
    let revision = selected.revision;
    if !library.commit_input(third_task, revision, 0, false) {
        return Err("Could not advance the active task before switching.".into());
    }
    if !navigate_demo_mouse(&mut library, 4) {
        return Err("Mouse button 4 was not handled during typing.".into());
    }
    if !library.typing_state.is_active()
        || library
            .selected()
            .is_none_or(|snippet| !snippet.name.starts_with("Задание 4 · "))
    {
        return Err("The first forward mouse press stopped typing or missed task 4.".into());
    }
    if !navigate_demo_mouse(&mut library, 4) {
        return Err("Second mouse button 4 press was not handled.".into());
    }
    if !library.typing_state.is_active()
        || library
            .selected()
            .is_none_or(|snippet| !snippet.name.starts_with("Задание 5 · "))
    {
        return Err("Two forward mouse presses did not keep typing on task 5.".into());
    }
    if !navigate_demo_mouse(&mut library, 5) {
        return Err("Mouse button 5 was not handled during typing.".into());
    }
    if !library.typing_state.is_active()
        || library
            .selected()
            .is_none_or(|snippet| !snippet.name.starts_with("Задание 4 · "))
    {
        return Err("The backward mouse press stopped typing or missed task 4.".into());
    }
    library.select_and_start(6);
    if !library.typing_state.is_active()
        || library
            .selected()
            .is_none_or(|snippet| !snippet.name.starts_with("Задание 7 · "))
    {
        return Err("Alt+7 did not switch to task 7 while keeping typing active.".into());
    }
    if library
        .snippets
        .iter()
        .find(|snippet| snippet.id == third_task)
        .is_none_or(|snippet| snippet.cursor != 1)
    {
        return Err("Switching tasks lost the previous task's cursor.".into());
    }
    library.select_and_start(DEMO_TASK_COUNT - 1);
    if !navigate_demo_mouse(&mut library, 4)
        || library
            .selected()
            .is_none_or(|snippet| !snippet.name.starts_with("Задание 1 · "))
        || !navigate_demo_mouse(&mut library, 5)
        || library
            .selected()
            .is_none_or(|snippet| !snippet.name.starts_with("Задание 25 · "))
    {
        return Err("Mouse navigation did not wrap safely between task 25 and task 1.".into());
    }

    println!(
        "PASS: 25 pinned C# tasks from the DOCX, Alt+1–7 mapping, active-only mouse 4/5 selection, cursor retention, and wraparound are verified headlessly."
    );
    Ok(())
}
