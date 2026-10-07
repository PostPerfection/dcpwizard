import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";

export const VERIFY_REPORT_EXTENSION = "html";
export const VERIFY_PDF_EXTENSION = "pdf";
const VERIFY_REPORT_NAME_ENDING = "-verify";

const SAVE_REPORT_BUTTON = "verify-save-report";
const SAVE_PDF_BUTTON = "verify-save-pdf";

export function verifyReportPathBeside(packageDirectory, extension) {
  const trimmed = packageDirectory.replace(/[/\\]+$/, "");
  return `${trimmed}${VERIFY_REPORT_NAME_ENDING}.${extension}`;
}

export async function verifyReportOutputArgs() {
  return ["--output", await invoke("verify_report_path")];
}

export function setVerifyReportSavable(savable) {
  for (const id of [SAVE_REPORT_BUTTON, SAVE_PDF_BUTTON]) {
    const button = document.getElementById(id);
    if (button) button.disabled = !savable;
  }
}

async function saveReportAs({ packageDirectory, extension, filterName, command, setStatus }) {
  const destination = await save({
    defaultPath: verifyReportPathBeside(packageDirectory(), extension),
    filters: [{ name: filterName, extensions: [extension] }],
  });
  if (!destination) return;
  try {
    await invoke(command, { destination });
    setStatus(`Saved the report to ${destination}`);
  } catch (error) {
    setStatus(`Could not save the report: ${error}`);
  }
}

export async function initVerifyReport({ packageDirectory, setStatus }) {
  document.getElementById(SAVE_REPORT_BUTTON)?.addEventListener("click", () =>
    saveReportAs({
      packageDirectory,
      extension: VERIFY_REPORT_EXTENSION,
      filterName: "HTML report",
      command: "save_verify_report",
      setStatus,
    }),
  );
  const pdfButton = document.getElementById(SAVE_PDF_BUTTON);
  if (!pdfButton) return;
  pdfButton.hidden = !(await invoke("verify_report_prints_to_pdf"));
  pdfButton.addEventListener("click", () =>
    saveReportAs({
      packageDirectory,
      extension: VERIFY_PDF_EXTENSION,
      filterName: "PDF",
      command: "print_verify_report_to_pdf",
      setStatus,
    }),
  );
}
