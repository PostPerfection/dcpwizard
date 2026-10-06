#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

const PROJECT_FILE_EXTENSION: &str = "dcpwizard";
const MAIN_WINDOW_LABEL: &str = "main";
#[cfg(target_os = "linux")]
const MAIN_WEBVIEW_LABEL: &str = "main-webview";
#[cfg(target_os = "linux")]
const MAIN_WINDOW_TITLE: &str = "DCP Wizard — DCP Creator";
#[cfg(target_os = "linux")]
const MAIN_WINDOW_WIDTH: f64 = 900.0;
#[cfg(target_os = "linux")]
const MAIN_WINDOW_HEIGHT: f64 = 700.0;
#[cfg(target_os = "linux")]
const MAIN_WINDOW_MINIMUM_WIDTH: f64 = 700.0;
#[cfg(target_os = "linux")]
const MAIN_WINDOW_MINIMUM_HEIGHT: f64 = 500.0;
#[cfg(target_os = "linux")]
const MAIN_WINDOW_BACKGROUND: tauri::window::Color = tauri::window::Color(0, 0, 0, 255);

mod crash_log;
mod export;
mod library;
mod pipeline;
mod preferences;
mod timeline;

#[tauri::command]
fn cpl_identities(dcp_dir: String) -> Result<Vec<dcpwizard_core::info::CplIdentity>, String> {
    dcpwizard_core::info::cpl_identities(std::path::Path::new(&dcp_dir))
}

#[tauri::command]
fn component_versions(
    preview_player: tauri::State<'_, guikit::preview::PreviewPlayer>,
) -> Vec<postkit::component_versions::ComponentVersion> {
    guikit::component_versions::installed_components(
        "DCP Wizard",
        env!("CARGO_PKG_VERSION"),
        &preview_player,
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    guikit::startup::prefer_shared_memory_webkit_frames_on_nvidia();
    postkit::grok_encoder::set_packaged_gpu_plugin_path("dcpwizard");

    #[cfg(unix)]
    guikit::startup::fork_terminal_guard();
    crash_log::install();

    let job_queue = pipeline::JobQueue::new(pipeline::jobs_path());
    job_queue.load_jobs_file();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_fs::init())
        .manage(job_queue)
        .manage(export::ExportState::default())
        .manage(guikit::launch_project::LaunchProject::default())
        .invoke_handler(tauri::generate_handler![
            guikit::preview::preview_load,
            guikit::preview::preview_play_pause,
            guikit::preview::preview_seek,
            guikit::preview::preview_seek_absolute,
            guikit::preview::preview_frame_step,
            guikit::preview::preview_frame_back_step,
            guikit::preview::preview_stop,
            guikit::preview::preview_load_dcp,
            guikit::preview::preview_needs_content_keys,
            guikit::preview::preview_get_position,
            guikit::preview::preview_get_duration,
            guikit::preview::preview_get_metadata,
            guikit::preview::preview_set_surface,
            guikit::preview::preview_is_embedded,
            guikit::preview::preview_set_overlays,
            guikit::preview::preview_set_decode_scale,
            guikit::preview::preview_set_subtitle_file,
            guikit::preview::preview_set_subtitle_visibility,
            guikit::gpu::set_gpu,
            component_versions,
            guikit::launch_project::take_launch_project_path,
            preferences::load_preferences,
            preferences::save_preferences,
            preferences::reset_preferences,
            preferences::export_recipient_certificate,
            preferences::find_dcpomatic_config,
            preferences::import_dcpomatic_identity,
            cpl_identities,
            pipeline::submit_job,
            pipeline::cancel_job,
            pipeline::move_job,
            pipeline::pause_job,
            pipeline::resume_job,
            pipeline::list_jobs,
            pipeline::delete_dcp,
            pipeline::retitle_dcp,
            pipeline::disk_space,
            pipeline::list_profiles,
            pipeline::marker_labels,
            pipeline::detect_source_crop,
            pipeline::subtitle_file_for_preview,
            pipeline::probe_audio_map,
            pipeline::isdcf_name_preview,
            pipeline::create_vf,
            export::export_dcp,
            export::export_cancel,
            library::library_list,
            library::library_add,
            library::library_remove,
            library::library_needs_duration,
            timeline::list_cpls,
            timeline::get_timeline,
        ])
        .setup(|app| {
            #[cfg(target_os = "linux")]
            guikit::startup::create_main_window(
                app,
                &guikit::startup::MainWindow {
                    label: MAIN_WINDOW_LABEL,
                    webview_label: MAIN_WEBVIEW_LABEL,
                    title: MAIN_WINDOW_TITLE,
                    width: MAIN_WINDOW_WIDTH,
                    height: MAIN_WINDOW_HEIGHT,
                    minimum_width: MAIN_WINDOW_MINIMUM_WIDTH,
                    minimum_height: MAIN_WINDOW_MINIMUM_HEIGHT,
                    background: MAIN_WINDOW_BACKGROUND,
                },
            )?;
            app.manage(guikit::preview::create_player(app, MAIN_WINDOW_LABEL));
            guikit::launch_project::store_from_args(app.handle(), PROJECT_FILE_EXTENSION);
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            guikit::launch_project::handle_run_event(app, &event, PROJECT_FILE_EXTENSION);
            if let tauri::RunEvent::ExitRequested { .. } = event {
                app.state::<guikit::preview::PreviewPlayer>().shutdown();
                app.state::<pipeline::JobQueue>().stop_for_exit();
            }
        });
}
