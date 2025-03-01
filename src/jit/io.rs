use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{BufWriter, Write},
    sync::atomic::{AtomicBool, Ordering},
};

// INPUT
// =====
#[no_mangle]
pub extern "C" fn io_input(input_buffer: *const i64) -> u8 {
    let input_buffer =
        unsafe { std::mem::transmute::<*const i64, &mut VecDeque<char>>(input_buffer) };

    if input_buffer.len() == 0 {
        let mut line = String::new();
        std::io::stdin()
            .read_line(&mut line)
            .expect("Failed to read line");
        line = line.replace("\r\n", "\n");
        input_buffer.extend(line.chars());
    }

    let character = input_buffer.pop_front().expect("No more input");

    if character == '\n' {
        10
    } else {
        character as u8
    }
}
#[no_mangle]
pub extern "C" fn io_input_noop(_: i64) -> u8 {
    0
}
// ======
// OUTPUT
// ======
thread_local! {
    static WRITER: RefCell<Option<(BufWriter<std::io::Stdout>, usize)>> = RefCell::new(None);
}
static ATEXIT_REGISTERED: AtomicBool = AtomicBool::new(false);
extern "C" {
    fn atexit(cb: extern "C" fn()) -> i32;
}
extern "C" fn flush_at_exit() {
    WRITER.with(|w| {
        if let Some(mut writer) = w.borrow_mut().take() {
            let _ = writer.0.flush();
        }
    });
}
#[no_mangle]
pub extern "C" fn io_output(value: u8) {
    // Register atexit handler if not already done
    if !ATEXIT_REGISTERED.load(Ordering::Relaxed) {
        unsafe {
            atexit(flush_at_exit);
        }
        ATEXIT_REGISTERED.store(true, Ordering::Relaxed);
    }

    const FLUSH_THRESHOLD: usize = 80; // Flush after this many characters

    WRITER.with(|w| {
        let mut w_ref = w.borrow_mut();
        if w_ref.is_none() {
            *w_ref = Some((BufWriter::with_capacity(4096, std::io::stdout()), 0));
        }

        let (writer, char_count) = w_ref.as_mut().unwrap();

        if value == 10 {
            if cfg!(windows) {
                writer
                    .write_all(b"\r\n")
                    .expect("Failed to write to stdout");
            } else {
                writer.write_all(b"\n").expect("Failed to write to stdout");
            }
            // Always flush on newlines for interactive behavior
            writer.flush().expect("Failed to flush stdout");
            *char_count = 0; // Reset counter after flush
        } else {
            writer
                .write_all(&[value])
                .expect("Failed to write to stdout");

            *char_count += 1;

            // Also flush after threshold characters without a newline
            if *char_count >= FLUSH_THRESHOLD {
                writer.flush().expect("Failed to flush stdout");
                *char_count = 0; // Reset counter after flush
            }
        }
    });
}
#[no_mangle]
pub extern "C" fn io_output_noop(_: u8) {}
// ======
