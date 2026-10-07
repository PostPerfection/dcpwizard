import assert from "node:assert/strict";
import test from "node:test";

import { verifyReportPathBeside, VERIFY_REPORT_EXTENSION, VERIFY_PDF_EXTENSION } from "../src/verify-report.js";

test("the report is named after the package folder and offered beside it", () => {
  assert.equal(verifyReportPathBeside("/films/Feature_FTR", VERIFY_REPORT_EXTENSION), "/films/Feature_FTR-verify.html");
});

test("a trailing separator does not leave the report inside the package", () => {
  assert.equal(verifyReportPathBeside("/films/Feature_FTR/", VERIFY_PDF_EXTENSION), "/films/Feature_FTR-verify.pdf");
  assert.equal(verifyReportPathBeside("C:\\films\\Feature_FTR\\", VERIFY_PDF_EXTENSION), "C:\\films\\Feature_FTR-verify.pdf");
});
