//! C ABI for native platform adapters.
//!
//! The first version returns a compact JSON action list. This keeps Swift-side
//! integration simple while the action schema is still evolving.

use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use ime_core::{ImeAction, ImeEngine, InputEvent};

pub const EVENT_CHARACTER: u32 = 0;
pub const EVENT_SPACE: u32 = 1;
pub const EVENT_ENTER: u32 = 2;
pub const EVENT_ESCAPE: u32 = 3;
pub const EVENT_BACKSPACE: u32 = 4;
pub const EVENT_NEXT_CANDIDATE: u32 = 5;
pub const EVENT_PREVIOUS_CANDIDATE: u32 = 6;

pub struct ImeHandle {
    engine: ImeEngine,
}

#[repr(C)]
#[derive(Debug)]
pub struct ImeBuffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl ImeBuffer {
    fn from_string(value: String) -> Self {
        let mut bytes = value.into_bytes();
        let buffer = Self {
            data: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        };
        std::mem::forget(bytes);
        buffer
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ime_create() -> *mut ImeHandle {
    match catch_unwind(|| ImeHandle {
        engine: ImeEngine::bundled(),
    }) {
        Ok(handle) => Box::into_raw(Box::new(handle)),
        Err(_) => ptr::null_mut(),
    }
}

/// Destroys a handle returned by [`ime_create`].
///
/// # Safety
///
/// `handle` must be null or a live pointer returned by [`ime_create`]. It must
/// not be used again after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ime_destroy(handle: *mut ImeHandle) {
    if !handle.is_null() {
        // SAFETY: The caller promises ownership of a live `ime_create` pointer.
        drop(unsafe { Box::from_raw(handle) });
    }
}

/// Processes one input event and returns a UTF-8 JSON action list.
///
/// `scalar` is used only for [`EVENT_CHARACTER`] and must be a valid Unicode
/// scalar value. The returned buffer must be released with [`ime_buffer_destroy`].
///
/// # Safety
///
/// `handle` must be null or a live, exclusively accessed pointer returned by
/// [`ime_create`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ime_process(
    handle: *mut ImeHandle,
    event_kind: u32,
    scalar: u32,
) -> ImeBuffer {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() {
            return error_response("null_handle");
        }

        let event = match decode_event(event_kind, scalar) {
            Ok(event) => event,
            Err(error) => return error_response(error),
        };

        // SAFETY: The caller promises a live, exclusively accessed handle.
        let handle = unsafe { &mut *handle };
        let actions = handle.engine.handle(event);
        success_response(&actions)
    }));

    ImeBuffer::from_string(match result {
        Ok(response) => response,
        Err(_) => error_response("panic"),
    })
}

/// Releases a buffer returned by [`ime_process`].
///
/// # Safety
///
/// `buffer` must be an unmodified value returned by [`ime_process`] and may be
/// released exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ime_buffer_destroy(buffer: ImeBuffer) {
    if buffer.data.is_null() {
        return;
    }

    // SAFETY: The caller promises this is the original allocation triple.
    drop(unsafe { Vec::from_raw_parts(buffer.data, buffer.len, buffer.capacity) });
}

fn decode_event(event_kind: u32, scalar: u32) -> Result<InputEvent, &'static str> {
    match event_kind {
        EVENT_CHARACTER => char::from_u32(scalar)
            .map(InputEvent::Character)
            .ok_or("invalid_unicode_scalar"),
        EVENT_SPACE => Ok(InputEvent::Space),
        EVENT_ENTER => Ok(InputEvent::Enter),
        EVENT_ESCAPE => Ok(InputEvent::Escape),
        EVENT_BACKSPACE => Ok(InputEvent::Backspace),
        EVENT_NEXT_CANDIDATE => Ok(InputEvent::NextCandidate),
        EVENT_PREVIOUS_CANDIDATE => Ok(InputEvent::PreviousCandidate),
        _ => Err("invalid_event_kind"),
    }
}

fn success_response(actions: &[ImeAction]) -> String {
    let mut output = String::from("{\"ok\":true,\"actions\":[");
    for (index, action) in actions.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_action(&mut output, action);
    }
    output.push_str("]}");
    output
}

fn error_response(error: &str) -> String {
    let mut output = String::from("{\"ok\":false,\"error\":");
    write_json_string(&mut output, error);
    output.push('}');
    output
}

fn write_action(output: &mut String, action: &ImeAction) {
    match action {
        ImeAction::UpdatePreedit(text) => {
            output.push_str("{\"type\":\"update_preedit\",\"text\":");
            write_json_string(output, text);
            output.push('}');
        }
        ImeAction::ShowCandidates {
            candidates,
            selected,
        } => {
            output.push_str("{\"type\":\"show_candidates\",\"selected\":");
            write!(output, "{selected}").expect("writing to String cannot fail");
            output.push_str(",\"candidates\":[");
            for (index, candidate) in candidates.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_json_string(output, candidate);
            }
            output.push_str("]}");
        }
        ImeAction::HideCandidates => output.push_str("{\"type\":\"hide_candidates\"}"),
        ImeAction::Commit(text) => {
            output.push_str("{\"type\":\"commit\",\"text\":");
            write_json_string(output, text);
            output.push('}');
        }
        ImeAction::Clear => output.push_str("{\"type\":\"clear\"}"),
        ImeAction::ForwardKey => output.push_str("{\"type\":\"forward_key\"}"),
    }
}

fn write_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("writing to String cannot fail");
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::{
        EVENT_CHARACTER, EVENT_SPACE, ImeBuffer, ime_buffer_destroy, ime_create, ime_destroy,
        ime_process,
    };

    unsafe fn copy_buffer(buffer: &ImeBuffer) -> String {
        // SAFETY: Tests read a live buffer before handing it back to its destructor.
        let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) };
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[test]
    fn ffi_round_trip_returns_utf8_actions() {
        let handle = ime_create();
        assert!(!handle.is_null());

        for character in "nihon".chars() {
            // SAFETY: `handle` is live and accessed serially in this test.
            let buffer = unsafe { ime_process(handle, EVENT_CHARACTER, character.into()) };
            // SAFETY: `buffer` is live until the destroy call below.
            let json = unsafe { copy_buffer(&buffer) };
            assert!(json.contains("\"ok\":true"));
            // SAFETY: `buffer` has not previously been released.
            unsafe { ime_buffer_destroy(buffer) };
        }

        // SAFETY: `handle` is live and accessed serially in this test.
        let buffer = unsafe { ime_process(handle, EVENT_SPACE, 0) };
        // SAFETY: `buffer` is live until the destroy call below.
        let json = unsafe { copy_buffer(&buffer) };
        assert!(json.contains("日本"));
        assert!(json.contains("show_candidates"));

        // SAFETY: Resources are live and each is destroyed exactly once.
        unsafe {
            ime_buffer_destroy(buffer);
            ime_destroy(handle);
        }
    }

    #[test]
    fn invalid_event_is_reported_without_panicking() {
        let handle = ime_create();
        // SAFETY: `handle` is live and accessed serially in this test.
        let buffer = unsafe { ime_process(handle, 999, 0) };
        // SAFETY: `buffer` is live until the destroy call below.
        let json = unsafe { copy_buffer(&buffer) };

        assert_eq!(json, "{\"ok\":false,\"error\":\"invalid_event_kind\"}");

        // SAFETY: Resources are live and each is destroyed exactly once.
        unsafe {
            ime_buffer_destroy(buffer);
            ime_destroy(handle);
        }
    }

    #[test]
    fn null_handle_is_an_error() {
        // SAFETY: A null handle is explicitly accepted and reported as an error.
        let buffer = unsafe { ime_process(std::ptr::null_mut(), EVENT_SPACE, 0) };
        // SAFETY: `buffer` is live until the destroy call below.
        let json = unsafe { copy_buffer(&buffer) };
        assert_eq!(json, "{\"ok\":false,\"error\":\"null_handle\"}");
        // SAFETY: `buffer` has not previously been released.
        unsafe { ime_buffer_destroy(buffer) };
    }
}
