#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// pipeline + preview

#[allow(unused_imports)]
use tauri::Manager;

mod pipeline;
mod preview_server;

#[cfg(unix)]
fn fork_terminal_guard() {
    unsafe {
        if libc::isatty(libc::STDIN_FILENO) == 0 {
            return;
        }

        let mut saved: libc::termios = std::mem::zeroed();
        libc::tcgetattr(libc::STDIN_FILENO, &mut saved);

        let pid = libc::fork();
        if pid < 0 {
            return;
        }
        if pid > 0 {
            let mut status: libc::c_int = 0;
            libc::waitpid(pid, &mut status, 0);
            libc::usleep(100_000);
            libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, &saved);
            libc::system(b"stty sane 2>/dev/null\0".as_ptr() as *const _);
            let exit_code = if libc::WIFEXITED(status) {
                libc::WEXITSTATUS(status)
            } else {
                1
            };
            std::process::exit(exit_code);
        }
        let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_RDONLY);
        if devnull >= 0 {
            libc::dup2(devnull, libc::STDIN_FILENO);
            libc::close(devnull);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(unix)]
    fork_terminal_guard();

    let mpv = preview_server::MpvPlayer::new();
    let job_queue = pipeline::JobQueue::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .manage(mpv)
        .manage(job_queue)
        .invoke_handler(tauri::generate_handler![
            preview_server::preview_load,
            preview_server::preview_play_pause,
            preview_server::preview_seek,
            preview_server::preview_stop,
            preview_server::preview_load_dcp,
            pipeline::submit_job,
            pipeline::cancel_job,
            pipeline::pause_job,
            pipeline::resume_job,
            pipeline::list_jobs,
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
