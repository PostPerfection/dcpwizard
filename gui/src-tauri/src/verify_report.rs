use std::path::PathBuf;
use tauri::{AppHandle, Manager};

const VERIFY_REPORT_FILE_NAME: &str = "verify-report.html";

#[cfg(target_os = "linux")]
const PDF_WINDOW_LABEL: &str = "verify-report-pdf";
// gtk names its file printer with this string translated through its own domain
#[cfg(target_os = "linux")]
const PRINT_TO_FILE_PRINTER: &str = "Print to File";
#[cfg(target_os = "linux")]
const GTK_TRANSLATION_DOMAIN: &str = "gtk30";
#[cfg(target_os = "linux")]
const PDF_FILE_FORMAT: &str = "pdf";

fn verify_report_file(app: &AppHandle) -> Result<PathBuf, String> {
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|error| error.to_string())?;
    Ok(cache.join(VERIFY_REPORT_FILE_NAME))
}

// a run that writes no report must not leave the last one to be saved
#[tauri::command]
pub fn verify_report_path(app: AppHandle) -> Result<String, String> {
    let report = verify_report_file(&app)?;
    if let Some(cache) = report.parent() {
        std::fs::create_dir_all(cache)
            .map_err(|error| format!("Could not create {}: {error}", cache.display()))?;
    }
    match std::fs::remove_file(&report) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("Could not remove {}: {error}", report.display())),
    }
    Ok(report.display().to_string())
}

#[tauri::command]
pub fn save_verify_report(app: AppHandle, destination: String) -> Result<(), String> {
    let report = verify_report_file(&app)?;
    std::fs::copy(&report, &destination)
        .map(|_| ())
        .map_err(|error| {
            format!(
                "Could not copy {} to {destination}: {error}",
                report.display()
            )
        })
}

#[tauri::command]
pub fn verify_report_prints_to_pdf() -> bool {
    cfg!(target_os = "linux")
}

#[cfg(target_os = "linux")]
type PrintOutcome = tauri::async_runtime::Sender<Result<(), String>>;

#[cfg(target_os = "linux")]
#[tauri::command]
pub async fn print_verify_report_to_pdf(app: AppHandle, destination: String) -> Result<(), String> {
    let report = verify_report_file(&app)?;
    if !report.is_file() {
        return Err(format!("No report at {}", report.display()));
    }
    let report_url = tauri::Url::from_file_path(&report)
        .map_err(|()| format!("{} is not an absolute path", report.display()))?;
    let pdf_url = tauri::Url::from_file_path(&destination)
        .map_err(|()| format!("{destination} is not an absolute path"))?;

    let (sender, mut receiver) = tauri::async_runtime::channel(1);
    let window = tauri::WebviewWindowBuilder::new(
        &app,
        PDF_WINDOW_LABEL,
        tauri::WebviewUrl::External(report_url),
    )
    .visible(false)
    .on_page_load(move |window, payload| {
        if payload.event() != tauri::webview::PageLoadEvent::Finished {
            return;
        }
        let printer_sender = sender.clone();
        let pdf_url = pdf_url.clone();
        let started = window.with_webview(move |webview| {
            print_to_file(&webview.inner(), pdf_url.as_str(), printer_sender)
        });
        if let Err(error) = started {
            let _ = sender.try_send(Err(error.to_string()));
        }
    })
    .build()
    .map_err(|error| error.to_string())?;

    let printed = receiver
        .recv()
        .await
        .unwrap_or_else(|| Err("The report window closed before it printed".into()));
    window.destroy().map_err(|error| error.to_string())?;
    printed
}

#[cfg(target_os = "linux")]
fn print_to_file(webview: &webkit2gtk::WebView, pdf_uri: &str, outcome: PrintOutcome) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use webkit2gtk::PrintOperationExt;

    let settings = gtk::PrintSettings::new();
    settings.set_printer(&gtk::glib::dgettext(
        Some(GTK_TRANSLATION_DOMAIN),
        PRINT_TO_FILE_PRINTER,
    ));
    settings.set(gtk::PRINT_SETTINGS_OUTPUT_URI, Some(pdf_uri));
    settings.set(
        gtk::PRINT_SETTINGS_OUTPUT_FILE_FORMAT,
        Some(PDF_FILE_FORMAT),
    );

    let operation = webkit2gtk::PrintOperation::new(webview);
    operation.set_print_settings(&settings);
    // webkit emits finished after failed too
    let failure = Rc::new(RefCell::new(None));
    let failure_seen = failure.clone();
    operation.connect_failed(move |_, error| {
        failure_seen.replace(Some(error.to_string()));
    });
    operation.connect_finished(move |_| {
        let printed = failure.take().map_or(Ok(()), Err);
        let _ = outcome.try_send(printed);
    });
    operation.print();
}
