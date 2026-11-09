use std::{
    collections::HashSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use crossbeam_channel::{Receiver, Sender, unbounded};
use enigo::{Direction, Enigo, Key as OutputKey, Keyboard, Settings};
#[cfg(all(not(windows), not(target_os = "macos")))]
use rdev::{Button, Event, EventType, Key};
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

use crate::core::{Library, demo_mouse_offset};

#[derive(Debug, Clone)]
pub enum InputEvent {
    Progress {
        snippet_id: Uuid,
        cursor: usize,
        finished: bool,
    },
    Error(String),
    Ready,
    Selection {
        snippet_id: Uuid,
    },
}

#[derive(Clone, Copy)]
pub(crate) enum InputMode {
    KeyboardSignal,
    Automatic,
}

pub(crate) enum TypeResult {
    Progress(InputEvent),
    ConsumeSignal,
    Idle,
}

#[derive(Default)]
pub(crate) struct CursorCache {
    snippet_id: Option<Uuid>,
    revision: u64,
    cursor: usize,
    byte_position: usize,
}

pub struct InputHub {
    app_focused: Arc<AtomicBool>,
    events: Receiver<InputEvent>,
}

fn app_has_keyboard_focus(fallback: &AtomicBool) -> bool {
    #[cfg(windows)]
    if let Some(focused) = windows_hook::current_process_is_foreground() {
        return focused;
    }

    fallback.load(Ordering::Acquire)
}

impl InputHub {
    pub fn start(library: Arc<Mutex<Library>>) -> Self {
        let app_focused = Arc::new(AtomicBool::new(false));
        let worker_focus = Arc::clone(&app_focused);
        let (sender, events) = unbounded();
        let automatic_library = Arc::clone(&library);
        let automatic_focus = Arc::clone(&app_focused);
        let automatic_sender = sender.clone();
        thread::Builder::new()
            .name("live-code-automatic-typing".into())
            .spawn(move || {
                automatic_typing_loop(automatic_library, automatic_focus, automatic_sender)
            })
            .expect("Не удалось создать поток автоматической печати");
        thread::Builder::new()
            .name("live-code-keyboard-grab".into())
            .spawn(move || keyboard_loop(library, worker_focus, sender))
            .expect("Не удалось создать поток клавиатурного перехвата");
        Self {
            app_focused,
            events,
        }
    }

    pub fn set_focus(&self, focused: bool) {
        self.app_focused.store(focused, Ordering::Release);
    }

    pub fn events(&self) -> &Receiver<InputEvent> {
        &self.events
    }
}

fn navigate_mouse(library: &Arc<Mutex<Library>>, sender: &Sender<InputEvent>, button: u8) -> bool {
    let mut library = library
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !crate::core::navigate_demo_mouse(&mut library, button) {
        return false;
    }
    let snippet_id = library.selected_id;
    let _ = sender.send(InputEvent::Selection { snippet_id });
    true
}

fn automatic_typing_loop(
    library: Arc<Mutex<Library>>,
    app_focused: Arc<AtomicBool>,
    sender: Sender<InputEvent>,
) {
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(enigo) => enigo,
        Err(error) => {
            let _ = sender.send(InputEvent::Error(format!(
                "Не удалось включить автоматическую печать: {error}"
            )));
            return;
        }
    };
    let mut cache = CursorCache::default();
    loop {
        if app_has_keyboard_focus(&app_focused) {
            thread::sleep(Duration::from_millis(80));
            continue;
        }
        match type_next_with_enigo(&mut enigo, &library, &mut cache, InputMode::Automatic) {
            Ok(TypeResult::Progress(progress)) => {
                let _ = sender.send(progress);
                thread::sleep(Duration::from_millis(45));
            }
            Ok(TypeResult::Idle | TypeResult::ConsumeSignal) => {
                thread::sleep(Duration::from_millis(35));
            }
            Err(error) => {
                let _ = sender.send(InputEvent::Error(error));
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn keyboard_loop(
    library: Arc<Mutex<Library>>,
    app_focused: Arc<AtomicBool>,
    sender: Sender<InputEvent>,
) {
    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(enigo) => {
            let _ = sender.send(InputEvent::Ready);
            enigo
        }
        Err(error) => {
            let _ = sender.send(InputEvent::Error(format!(
                "Не удалось включить отправку клавиш: {error}"
            )));
            return;
        }
    };

    let injecting = Arc::new(AtomicBool::new(false));
    let callback_injecting = Arc::clone(&injecting);
    let mut cache = CursorCache::default();
    let mut suppressed = HashSet::new();
    let mut modifiers = HashSet::new();
    let callback = move |event: Event| -> Option<Event> {
        if callback_injecting.load(Ordering::Acquire) {
            return Some(event);
        }

        if let EventType::ButtonPress(Button::Unknown(button @ (4 | 5))) = event.event_type
            && demo_mouse_offset(button).is_some()
            && navigate_mouse(&library, &sender, button)
        {
            return None;
        }

        let key = match event.event_type {
            EventType::KeyPress(key) => key,
            EventType::KeyRelease(key) => {
                if is_modifier_key(key) {
                    modifiers.remove(&key);
                    return Some(event);
                }
                return if suppressed.remove(&key) {
                    None
                } else {
                    Some(event)
                };
            }
            _ => return Some(event),
        };

        if is_modifier_key(key) {
            modifiers.insert(key);
            return Some(event);
        }
        if !is_text_rdev_key(key) || modifiers.iter().any(is_blocking_modifier_key) {
            return Some(event);
        }
        if app_focused.load(Ordering::Acquire) {
            return Some(event);
        }
        if suppressed.contains(&key) {
            return None;
        }

        callback_injecting.store(true, Ordering::Release);
        let sent =
            type_next_with_enigo(&mut enigo, &library, &mut cache, InputMode::KeyboardSignal);
        callback_injecting.store(false, Ordering::Release);
        match sent {
            Ok(TypeResult::Progress(progress)) => {
                suppressed.insert(key);
                let _ = sender.send(progress);
                None
            }
            Ok(TypeResult::ConsumeSignal) => {
                suppressed.insert(key);
                None
            }
            Ok(TypeResult::Idle) => Some(event),
            Err(error) => {
                suppressed.insert(key);
                let _ = sender.send(InputEvent::Error(error));
                None
            }
        }
    };

    if let Err(error) = rdev::grab(callback) {
        let _ = sender.send(InputEvent::Error(format!(
            "Не удалось перехватить клавиатуру: {error}. На macOS разрешите LiveCode Typer в разделе «Универсальный доступ»."
        )));
    }
}

#[cfg(target_os = "macos")]
fn keyboard_loop(
    library: Arc<Mutex<Library>>,
    app_focused: Arc<AtomicBool>,
    sender: Sender<InputEvent>,
) {
    let mut settings = Settings::default();
    settings.event_source_user_data = Some(macos_hook::EVENT_MARKER);
    let enigo = match Enigo::new(&settings) {
        Ok(enigo) => enigo,
        Err(error) => {
            let _ = sender.send(InputEvent::Error(format!(
                "Не удалось включить отправку клавиш: {error}"
            )));
            return;
        }
    };
    macos_hook::run(enigo, library, app_focused, sender);
}

#[cfg(windows)]
fn keyboard_loop(
    library: Arc<Mutex<Library>>,
    app_focused: Arc<AtomicBool>,
    sender: Sender<InputEvent>,
) {
    let mut settings = Settings::default();
    settings.windows_dw_extra_info = Some(windows_hook::EVENT_MARKER);
    let enigo = match Enigo::new(&settings) {
        Ok(enigo) => enigo,
        Err(error) => {
            let _ = sender.send(InputEvent::Error(format!(
                "Не удалось включить отправку клавиш: {error}"
            )));
            return;
        }
    };
    windows_hook::run(enigo, library, app_focused, sender);
}

#[cfg(windows)]
mod windows_hook {
    use std::{cell::Cell, ptr::null_mut};

    use windows_sys::Win32::{
        Foundation::{LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::{
            CallNextHookEx, GetForegroundWindow, GetMessageW, GetWindowThreadProcessId, HC_ACTION,
            KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, MSLLHOOKSTRUCT, SetWindowsHookExW,
            UnhookWindowsHookEx, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
            WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
        },
    };

    use super::*;

    pub const EVENT_MARKER: usize = 0x4C43_5459;

    pub(super) fn current_process_is_foreground() -> Option<bool> {
        let window = unsafe { GetForegroundWindow() };
        if window.is_null() {
            return None;
        }
        let mut process_id = 0;
        if unsafe { GetWindowThreadProcessId(window, &mut process_id) } == 0 || process_id == 0 {
            return None;
        }
        Some(process_id == std::process::id())
    }

    struct HookContext {
        enigo: Enigo,
        library: Arc<Mutex<Library>>,
        app_focused: Arc<AtomicBool>,
        sender: Sender<InputEvent>,
        cache: CursorCache,
        suppressed: HashSet<u32>,
        mouse_suppressed: HashSet<u8>,
        modifiers: HashSet<u32>,
    }

    thread_local! {
        static CONTEXT: Cell<*mut HookContext> = const { Cell::new(null_mut()) };
    }

    pub fn run(
        enigo: Enigo,
        library: Arc<Mutex<Library>>,
        app_focused: Arc<AtomicBool>,
        sender: Sender<InputEvent>,
    ) {
        let mut context = HookContext {
            enigo,
            library,
            app_focused,
            sender: sender.clone(),
            cache: CursorCache::default(),
            suppressed: HashSet::new(),
            mouse_suppressed: HashSet::new(),
            modifiers: HashSet::new(),
        };
        CONTEXT.with(|slot| slot.set(&mut context));

        let hook =
            unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_callback), null_mut(), 0) };
        if hook.is_null() {
            CONTEXT.with(|slot| slot.set(null_mut()));
            let _ = sender.send(InputEvent::Error(
                "Windows не разрешила перехват клавиатуры. Запустите приложение в обычном сеансе рабочего стола.".into(),
            ));
            return;
        }
        let mouse_hook =
            unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_callback), null_mut(), 0) };
        let _ = sender.send(InputEvent::Ready);
        if mouse_hook.is_null() {
            let _ = sender.send(InputEvent::Error(
                "Боковые кнопки мыши недоступны; Alt+1–7 продолжают работать.".into(),
            ));
        }
        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, null_mut(), 0, 0) } > 0 {}
        if !mouse_hook.is_null() {
            unsafe { UnhookWindowsHookEx(mouse_hook) };
        }
        unsafe { UnhookWindowsHookEx(hook) };
        CONTEXT.with(|slot| slot.set(null_mut()));
    }

    unsafe extern "system" fn mouse_callback(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 && data != 0 {
            let event = unsafe { &*(data as *const MSLLHOOKSTRUCT) };
            if let Some(result) = CONTEXT.with(|slot| {
                let context = slot.get();
                if context.is_null() {
                    None
                } else {
                    Some(unsafe { process_mouse_event(&mut *context, message as u32, event) })
                }
            }) {
                if result != 0 {
                    return result;
                }
            }
        }
        unsafe { CallNextHookEx(null_mut(), code, message, data) }
    }

    unsafe fn process_mouse_event(
        context: &mut HookContext,
        message: u32,
        event: &MSLLHOOKSTRUCT,
    ) -> LRESULT {
        if message != WM_XBUTTONDOWN && message != WM_XBUTTONUP {
            return 0;
        }
        let button = match (event.mouseData >> 16) as u16 {
            1 => 4,
            2 => 5,
            _ => return 0,
        };
        if message == WM_XBUTTONUP {
            return if context.mouse_suppressed.remove(&button) {
                1
            } else {
                0
            };
        }
        if demo_mouse_offset(button).is_some()
            && navigate_mouse(&context.library, &context.sender, button)
        {
            context.mouse_suppressed.insert(button);
            1
        } else {
            0
        }
    }

    unsafe extern "system" fn keyboard_callback(
        code: i32,
        message: WPARAM,
        data: LPARAM,
    ) -> LRESULT {
        if code == HC_ACTION as i32 && data != 0 {
            let key = unsafe { &*(data as *const KBDLLHOOKSTRUCT) };
            if key.flags & LLKHF_INJECTED != 0 && key.dwExtraInfo == EVENT_MARKER {
                return unsafe { CallNextHookEx(null_mut(), code, message, data) };
            }
            if let Some(result) = CONTEXT.with(|slot| {
                let context = slot.get();
                if context.is_null() {
                    None
                } else {
                    Some(unsafe { process_event(&mut *context, key.vkCode, message as u32) })
                }
            }) {
                if result != 0 {
                    return result;
                }
            }
        }
        unsafe { CallNextHookEx(null_mut(), code, message, data) }
    }

    unsafe fn process_event(context: &mut HookContext, virtual_key: u32, message: u32) -> LRESULT {
        let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
        let released = matches!(message, WM_KEYUP | WM_SYSKEYUP);
        if !pressed && !released {
            return 0;
        }
        if is_modifier(virtual_key) {
            if pressed {
                context.modifiers.insert(virtual_key);
            } else {
                context.modifiers.remove(&virtual_key);
            }
            return 0;
        }
        if released {
            return if context.suppressed.remove(&virtual_key) {
                1
            } else {
                0
            };
        }
        if !is_text_key(virtual_key) {
            return 0;
        }
        if context.suppressed.contains(&virtual_key) {
            return 1;
        }
        if context.modifiers.iter().copied().any(is_blocking_modifier)
            || app_has_keyboard_focus(&context.app_focused)
        {
            return 0;
        }

        match type_next_with_enigo(
            &mut context.enigo,
            &context.library,
            &mut context.cache,
            InputMode::KeyboardSignal,
        ) {
            Ok(TypeResult::Progress(progress)) => {
                context.suppressed.insert(virtual_key);
                let _ = context.sender.send(progress);
                1
            }
            Ok(TypeResult::ConsumeSignal) => {
                context.suppressed.insert(virtual_key);
                1
            }
            Ok(TypeResult::Idle) => 0,
            Err(error) => {
                context.suppressed.insert(virtual_key);
                let _ = context.sender.send(InputEvent::Error(error));
                1
            }
        }
    }

    fn is_modifier(key: u32) -> bool {
        matches!(key, 0x10 | 0x11 | 0x12 | 0x5B | 0x5C | 0xA0..=0xA5)
    }

    fn is_blocking_modifier(key: u32) -> bool {
        matches!(key, 0x11 | 0x12 | 0x5B | 0x5C | 0xA2..=0xA5)
    }

    fn is_text_key(key: u32) -> bool {
        matches!(
            key,
            0x20 | 0x0D
                | 0x30..=0x39
                | 0x41..=0x5A
                | 0x60..=0x69
                | 0x6A..=0x6F
                | 0xBA..=0xC0
                | 0xDB..=0xDF
                | 0xE2
        )
    }
}

#[cfg(target_os = "macos")]
mod macos_hook {
    use std::{
        collections::HashSet,
        ffi::c_void,
        ptr::{null, null_mut},
    };

    use core_graphics::event::EventField;
    use enigo::{Enigo, Keyboard};

    use super::*;

    pub const EVENT_MARKER: i64 = 0x4C43_5459;
    const KEY_DOWN: u32 = 10;
    const KEY_UP: u32 = 11;
    const FLAGS_CHANGED: u32 = 12;
    const OTHER_MOUSE_DOWN: u32 = 25;
    const OTHER_MOUSE_UP: u32 = 26;
    const NULL_EVENT: u32 = 0;
    const CONTROL_FLAG: u64 = 1 << 18;
    const OPTION_FLAG: u64 = 1 << 19;
    const COMMAND_FLAG: u64 = 1 << 20;

    struct HookContext {
        enigo: Enigo,
        library: Arc<Mutex<Library>>,
        app_focused: Arc<AtomicBool>,
        sender: Sender<InputEvent>,
        cache: CursorCache,
        suppressed: HashSet<u16>,
        mouse_suppressed: HashSet<u8>,
        tap: *const c_void,
    }

    type EventTapCallback = unsafe extern "C" fn(
        proxy: *const c_void,
        event_type: u32,
        event: *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: EventTapCallback,
            user_info: *mut c_void,
        ) -> *const c_void;
        fn CGEventTapEnable(tap: *const c_void, enable: bool);
        fn CGEventGetIntegerValueField(event: *mut c_void, field: u32) -> i64;
        fn CGEventGetFlags(event: *mut c_void) -> u64;
        fn CGEventSetType(event: *mut c_void, event_type: u32);
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            port: *const c_void,
            order: isize,
        ) -> *const c_void;
        fn CFRunLoopGetCurrent() -> *const c_void;
        fn CFRunLoopAddSource(run_loop: *const c_void, source: *const c_void, mode: *const c_void);
        fn CFRunLoopRun();
        static kCFRunLoopCommonModes: *const c_void;
    }

    pub fn run(
        enigo: Enigo,
        library: Arc<Mutex<Library>>,
        app_focused: Arc<AtomicBool>,
        sender: Sender<InputEvent>,
    ) {
        let mut context = HookContext {
            enigo,
            library,
            app_focused,
            sender: sender.clone(),
            cache: CursorCache::default(),
            suppressed: HashSet::new(),
            mouse_suppressed: HashSet::new(),
            tap: null(),
        };
        let context_ptr = &mut context as *mut HookContext as *mut c_void;
        let event_mask = (1u64 << KEY_DOWN)
            | (1u64 << KEY_UP)
            | (1u64 << FLAGS_CHANGED)
            | (1u64 << OTHER_MOUSE_DOWN)
            | (1u64 << OTHER_MOUSE_UP);
        let tap = unsafe { CGEventTapCreate(0, 0, 0, event_mask, event_callback, context_ptr) };
        if tap.is_null() {
            let _ = sender.send(InputEvent::Error(
                "macOS не разрешила перехват клавиатуры. Разрешите LiveCode Typer в настройках «Универсальный доступ» и «Мониторинг ввода».".into(),
            ));
            return;
        }
        context.tap = tap;
        let source = unsafe { CFMachPortCreateRunLoopSource(null(), tap, 0) };
        if source.is_null() {
            let _ = sender.send(InputEvent::Error(
                "Не удалось создать системный цикл перехвата клавиатуры macOS.".into(),
            ));
            return;
        }
        let run_loop = unsafe { CFRunLoopGetCurrent() };
        unsafe {
            CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
        }
        let _ = sender.send(InputEvent::Ready);
        unsafe { CFRunLoopRun() };
    }

    unsafe extern "C" fn event_callback(
        _proxy: *const c_void,
        event_type: u32,
        event: *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void {
        if event.is_null() || user_info.is_null() {
            return event;
        }
        let context = unsafe { &mut *(user_info as *mut HookContext) };
        if event_type >= u32::MAX - 2 {
            unsafe { CGEventTapEnable(context.tap, true) };
            return event;
        }
        let marker = unsafe {
            CGEventGetIntegerValueField(event, EventField::EVENT_SOURCE_USER_DATA as u32)
        };
        if marker == EVENT_MARKER || event_type == FLAGS_CHANGED {
            return event;
        }

        if event_type == OTHER_MOUSE_DOWN || event_type == OTHER_MOUSE_UP {
            let button_number = unsafe {
                CGEventGetIntegerValueField(event, EventField::MOUSE_EVENT_BUTTON_NUMBER as u32)
            }
            .clamp(0, u8::MAX as i64) as u8;
            if event_type == OTHER_MOUSE_UP {
                if context.mouse_suppressed.remove(&button_number) {
                    unsafe { CGEventSetType(event, NULL_EVENT) };
                }
                return event;
            }
            let button = button_number.saturating_add(1);
            if demo_mouse_offset(button).is_some()
                && navigate_mouse(&context.library, &context.sender, button)
            {
                context.mouse_suppressed.insert(button_number);
                unsafe { CGEventSetType(event, NULL_EVENT) };
            }
            return event;
        }

        let key_code = unsafe {
            CGEventGetIntegerValueField(event, EventField::KEYBOARD_EVENT_KEYCODE as u32)
        } as u16;
        if event_type == KEY_UP {
            if context.suppressed.remove(&key_code) {
                unsafe { CGEventSetType(event, NULL_EVENT) };
            }
            return event;
        }
        if event_type != KEY_DOWN {
            return event;
        }
        if context.suppressed.contains(&key_code) {
            unsafe { CGEventSetType(event, NULL_EVENT) };
            return event;
        }
        if !is_text_key(key_code) || context.app_focused.load(Ordering::Acquire) {
            return event;
        }
        let flags = unsafe { CGEventGetFlags(event) };
        if flags & (CONTROL_FLAG | OPTION_FLAG | COMMAND_FLAG) != 0 {
            return event;
        }
        match type_next_with_enigo(
            &mut context.enigo,
            &context.library,
            &mut context.cache,
            InputMode::KeyboardSignal,
        ) {
            Ok(TypeResult::Progress(progress)) => {
                context.suppressed.insert(key_code);
                let _ = context.sender.send(progress);
                unsafe { CGEventSetType(event, NULL_EVENT) };
            }
            Ok(TypeResult::ConsumeSignal) => {
                context.suppressed.insert(key_code);
                unsafe { CGEventSetType(event, NULL_EVENT) };
            }
            Ok(TypeResult::Idle) => {}
            Err(error) => {
                context.suppressed.insert(key_code);
                let _ = context.sender.send(InputEvent::Error(error));
                unsafe { CGEventSetType(event, NULL_EVENT) };
            }
        }
        event
    }

    fn is_text_key(key_code: u16) -> bool {
        matches!(
            key_code,
            0x00..=0x2F
                | 0x31..=0x32
                | 0x41
                | 0x43
                | 0x45
                | 0x4B..=0x4C
                | 0x4E
                | 0x51..=0x5C
        )
    }
}

fn type_next_with_enigo(
    enigo: &mut Enigo,
    library: &Arc<Mutex<Library>>,
    cache: &mut CursorCache,
    mode: InputMode,
) -> Result<TypeResult, String> {
    type_next(
        &mut |text| send_grapheme(enigo, text).map_err(|error| error.to_string()),
        library,
        cache,
        mode,
    )
}

pub(crate) fn type_next(
    write_text: &mut impl FnMut(&str) -> Result<(), String>,
    library: &Arc<Mutex<Library>>,
    cache: &mut CursorCache,
    mode: InputMode,
) -> Result<TypeResult, String> {
    let mut library = library
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if !library.typing {
        return Ok(TypeResult::Idle);
    }
    match mode {
        InputMode::KeyboardSignal if library.automatic_typing => {
            return Ok(TypeResult::ConsumeSignal);
        }
        InputMode::Automatic
            if !library.automatic_typing
                || library
                    .automatic_start_at
                    .is_some_and(|start_at| Instant::now() < start_at) =>
        {
            return Ok(TypeResult::Idle);
        }
        _ => {}
    }
    let next = {
        let Some(snippet) = library.selected() else {
            library.pause();
            return Ok(TypeResult::Idle);
        };
        if cache.snippet_id != Some(snippet.id)
            || cache.revision != snippet.revision
            || cache.cursor != snippet.cursor
        {
            cache.snippet_id = Some(snippet.id);
            cache.revision = snippet.revision;
            cache.cursor = snippet.cursor;
            cache.byte_position = snippet
                .code
                .grapheme_indices(true)
                .nth(snippet.cursor)
                .map_or(snippet.code.len(), |(byte, _)| byte);
        }
        snippet.code.get(cache.byte_position..).and_then(|tail| {
            tail.graphemes(true).next().map(|grapheme| {
                let next_byte = cache.byte_position + grapheme.len();
                (
                    snippet.id,
                    snippet.revision,
                    snippet.cursor,
                    grapheme.to_owned(),
                    next_byte,
                    next_byte == snippet.code.len(),
                )
            })
        })
    };
    let Some((snippet_id, revision, cursor, grapheme, next_byte, finished)) = next else {
        library.pause();
        *cache = CursorCache::default();
        return Ok(TypeResult::Idle);
    };
    if let Err(error) = write_text(&grapheme) {
        library.pause();
        return Err(format!("Не удалось ввести следующий символ: {error}"));
    }
    if !library.commit_input(snippet_id, revision, cursor, finished) {
        library.pause();
        *cache = CursorCache::default();
        return Err("Внутреннее состояние позиции изменилось во время ввода.".into());
    }
    cache.snippet_id = Some(snippet_id);
    cache.revision = revision;
    cache.cursor = cursor + 1;
    cache.byte_position = next_byte;
    Ok(TypeResult::Progress(InputEvent::Progress {
        snippet_id,
        cursor: cursor + 1,
        finished,
    }))
}

fn send_grapheme(enigo: &mut Enigo, text: &str) -> enigo::InputResult<()> {
    if text == "\n" {
        enigo.key(OutputKey::Return, Direction::Click)
    } else {
        enigo.text(text)
    }
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn is_modifier_key(key: Key) -> bool {
    matches!(
        key,
        Key::ControlLeft
            | Key::ControlRight
            | Key::Alt
            | Key::AltGr
            | Key::MetaLeft
            | Key::MetaRight
            | Key::ShiftLeft
            | Key::ShiftRight
    )
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn is_blocking_modifier_key(key: &Key) -> bool {
    matches!(
        key,
        Key::ControlLeft | Key::ControlRight | Key::Alt | Key::MetaLeft | Key::MetaRight
    )
}

#[cfg(all(not(windows), not(target_os = "macos")))]
fn is_text_rdev_key(key: Key) -> bool {
    matches!(
        key,
        Key::Space
            | Key::Return
            | Key::KpReturn
            | Key::Num0
            | Key::Num1
            | Key::Num2
            | Key::Num3
            | Key::Num4
            | Key::Num5
            | Key::Num6
            | Key::Num7
            | Key::Num8
            | Key::Num9
            | Key::KeyA
            | Key::KeyB
            | Key::KeyC
            | Key::KeyD
            | Key::KeyE
            | Key::KeyF
            | Key::KeyG
            | Key::KeyH
            | Key::KeyI
            | Key::KeyJ
            | Key::KeyK
            | Key::KeyL
            | Key::KeyM
            | Key::KeyN
            | Key::KeyO
            | Key::KeyP
            | Key::KeyQ
            | Key::KeyR
            | Key::KeyS
            | Key::KeyT
            | Key::KeyU
            | Key::KeyV
            | Key::KeyW
            | Key::KeyX
            | Key::KeyY
            | Key::KeyZ
            | Key::BackQuote
            | Key::Minus
            | Key::Equal
            | Key::LeftBracket
            | Key::RightBracket
            | Key::SemiColon
            | Key::Quote
            | Key::BackSlash
            | Key::IntlBackslash
            | Key::Comma
            | Key::Dot
            | Key::Slash
            | Key::Kp0
            | Key::Kp1
            | Key::Kp2
            | Key::Kp3
            | Key::Kp4
            | Key::Kp5
            | Key::Kp6
            | Key::Kp7
            | Key::Kp8
            | Key::Kp9
            | Key::KpMinus
            | Key::KpPlus
            | Key::KpMultiply
            | Key::KpDivide
    )
}
