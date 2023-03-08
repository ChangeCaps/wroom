#![allow(dead_code)]

use std::io;

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

mod app;
mod audio;
mod clip;
mod device_select;
mod engine;
mod play;
mod track;

#[macro_export]
macro_rules! gag {
    () => {
        #[cfg(target_os = "linux")]
        let _gag_stdout = gag::Gag::stdout();
        #[cfg(target_os = "linux")]
        let _gag_stderr = gag::Gag::stderr();
    };
}

#[cfg(target_os = "linux")]
unsafe extern "C" fn alsa_handler(
    _file: *const std::ffi::c_char,
    _line: std::ffi::c_int,
    _function: *const std::ffi::c_char,
    _err: std::ffi::c_int,
    _fmt: *const std::ffi::c_char,
    _args: *mut std::ffi::c_void,
) {
}

#[cfg(target_os = "linux")]
unsafe fn set_alsa_handler() {
    let handler = std::mem::transmute(alsa_handler as usize);

    alsa_sys::snd_lib_error_set_handler(Some(handler));
}

fn main() -> io::Result<()> {
    #[cfg(target_os = "linux")]
    unsafe {
        set_alsa_handler();
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    res
}
