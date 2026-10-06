import assert from "node:assert/strict";
import test from "node:test";

import {
  outputFileInFolder,
  qcReportPathBeside,
  SUBTITLE_CONVERSION_EXTENSION,
  BURN_IN_EXTENSION,
  BURN_IN_NAME_PART,
  TARGET_CONVERSION_EXTENSION,
} from "../src/tool-output.js";

test("the subtitle conversion writes the input's name as xml in the chosen folder", () => {
  const output = outputFileInFolder({ folder: "/out", input: "/media/film.en.srt", extension: SUBTITLE_CONVERSION_EXTENSION });

  assert.equal(output, "/out/film.en.xml");
});

test("burn-in names the file after the video with a burnin part", () => {
  const output = outputFileInFolder({
    folder: "/out/",
    input: "/media/film.mov",
    nameParts: [BURN_IN_NAME_PART],
    extension: BURN_IN_EXTENSION,
  });

  assert.equal(output, "/out/film_burnin.mp4");
});

test("target conversion puts the target container in the name", () => {
  const output = outputFileInFolder({
    folder: "/out",
    input: "/media/film.mp4",
    nameParts: ["2k-flat"],
    extension: TARGET_CONVERSION_EXTENSION,
  });

  assert.equal(output, "/out/film_2k-flat.mov");
});

test("with no folder chosen the file goes beside the input", () => {
  const output = outputFileInFolder({ folder: "", input: "/media/film.srt", extension: SUBTITLE_CONVERSION_EXTENSION });

  assert.equal(output, "/media/film.xml");
});

test("a Windows folder keeps backslashes", () => {
  const output = outputFileInFolder({
    folder: "C:\\out",
    input: "C:\\media\\film.mov",
    nameParts: [BURN_IN_NAME_PART],
    extension: BURN_IN_EXTENSION,
  });

  assert.equal(output, "C:\\out\\film_burnin.mp4");
});

test("the QC report sits beside the package, named after its folder", () => {
  assert.equal(qcReportPathBeside("/films/MyDCP"), "/films/MyDCP.qc.html");
});

test("a trailing slash on the package folder still puts the report beside it", () => {
  assert.equal(qcReportPathBeside("/films/MyDCP/"), "/films/MyDCP.qc.html");
});

test("a Windows package folder gets its report beside it", () => {
  assert.equal(qcReportPathBeside("C:\\films\\MyDCP\\"), "C:\\films\\MyDCP.qc.html");
});
