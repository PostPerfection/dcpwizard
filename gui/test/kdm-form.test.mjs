import assert from "node:assert/strict";
import test from "node:test";
import {
  chainPaths,
  describeSigner,
  kdmRequest,
  kdmTimestamp,
  recipientCertificateCommand,
  recipientCreationRefusals,
  rewrapRequest,
  signingChainCommand,
  signingChainFrom,
  SIGNER_FIELDS,
} from "../src/kdm-form.js";

const READY = {
  ...SIGNER_FIELDS,
  cplId: "96558952-39b8-42d3-825e-9ddd31298219",
  contentTitle: "My Film",
  forThisMachine: false,
  recipientCert: "/cinemas/screen.pem",
  recipientFromSettings: "/certs/recipient.pem",
  keys: "/keys/my-film.json",
  validFrom: "2026-10-05T09:00",
  validTo: "2026-10-19T09:00",
  offsetMinutes: 0,
  formulation: "",
  noForensicPicture: false,
  audioMarking: "on",
  audioMarkingChannel: "",
  output: "/out/screen.kdm.xml",
};

test("a datetime-local value becomes a 25-character KDM timestamp", () => {
  assert.equal(kdmTimestamp("2026-10-05T09:00", 0), "2026-10-05T09:00:00+00:00");
  assert.equal(kdmTimestamp("2026-10-05T09:00", 120), "2026-10-05T09:00:00+02:00");
  assert.equal(kdmTimestamp("2026-10-05T09:00", -330), "2026-10-05T09:00:00-05:30");
  assert.equal(kdmTimestamp("", 0), "");
});

test("a KDM is signed with the chain from Settings", () => {
  const request = kdmRequest(READY);
  assert.deepEqual(request.refusals, []);
  assert.deepEqual(request.args, [
    "kdm",
    "--cpl-id", READY.cplId,
    "--content-title", "My Film",
    "--cert", "/cinemas/screen.pem",
    "--signer-cert", "/certs/signer.pem",
    "--signer-key", "/certs/signer.key",
    "--signer-chain", "/certs/intermediate.pem",
    "--signer-chain", "/certs/root.pem",
    "--keys", "/keys/my-film.json",
    "-f", "2026-10-05T09:00:00+00:00",
    "-t", "2026-10-19T09:00:00+00:00",
    "-o", "/out/screen.kdm.xml",
  ]);
});

test("a DKDM for this machine uses the recipient certificate in Settings", () => {
  const request = kdmRequest({ ...READY, forThisMachine: true, recipientCert: "" });
  assert.deepEqual(request.refusals, []);
  assert.equal(request.args[request.args.indexOf("--cert") + 1], "/certs/recipient.pem");
});

test("a KDM without the signing chain is refused", () => {
  const request = kdmRequest({
    ...READY,
    signerIntermediate: "",
    signerRoot: "",
    keys: "",
    validFrom: "",
    validTo: "",
  });
  assert.ok(request.refusals.includes("Set the signing intermediate and root in Settings"));
  assert.ok(request.refusals.includes("A KDM needs the keys file from the encrypted build"));
  assert.ok(request.refusals.includes("A KDM needs a validity window"));
});

test("audio marking above a channel names that channel", () => {
  const request = kdmRequest({ ...READY, audioMarking: "above", audioMarkingChannel: "12" });
  assert.deepEqual(request.refusals, []);
  assert.deepEqual(
    request.args.slice(request.args.indexOf("--disable-forensic-marking-audio"), -2),
    ["--disable-forensic-marking-audio", "12"],
  );
});

test("re-wrapping keeps the DKDM window when the dates are empty", () => {
  const request = rewrapRequest({
    ...SIGNER_FIELDS,
    dkdm: "/keys/my.dkdm.xml",
    dkdmKey: "/certs/recipient.key",
    recipientCert: "/cinemas/screen.pem",
    validFrom: "",
    validTo: "",
    offsetMinutes: 0,
    formulation: "modified-transitional-1",
    noForensicPicture: true,
    audioMarking: "on",
    audioMarkingChannel: "",
    output: "/out/screen.kdm.xml",
  });
  assert.deepEqual(request.refusals, []);
  assert.equal(request.args[0], "kdm-rewrap");
  assert.equal(request.args.includes("-f"), false);
  assert.ok(request.args.includes("--formulation"));
  assert.ok(request.args.includes("--disable-forensic-marking-picture"));
});

test("one date on a re-wrap is refused", () => {
  const request = rewrapRequest({
    ...SIGNER_FIELDS,
    dkdm: "/keys/my.dkdm.xml",
    dkdmKey: "/certs/recipient.key",
    recipientCert: "/cinemas/screen.pem",
    validFrom: "2026-10-05T09:00",
    validTo: "",
    offsetMinutes: 0,
    formulation: "",
    noForensicPicture: false,
    audioMarking: "on",
    output: "/out/screen.kdm.xml",
  });
  assert.deepEqual(request.refusals, [
    "Set both dates, or leave both empty to keep the DKDM window",
  ]);
});

test("creating a chain fills the five Settings paths", () => {
  assert.deepEqual(signingChainCommand("Studio", "/certs"), [
    "certificate", "chain", "--organization", "Studio", "--output", "/certs",
  ]);
  assert.deepEqual(chainPaths("/certs"), {
    signingCert: "/certs/signer.pem",
    signingKey: "/certs/signer.key",
    signingIntermediate: "/certs/intermediate.pem",
    signingIntermediateKey: "/certs/intermediate.key",
    signingRoot: "/certs/root.pem",
  });
  assert.deepEqual(signingChainFrom(chainPaths("/certs")), [
    "/certs/intermediate.pem",
    "/certs/root.pem",
  ]);
});

test("a recipient certificate is refused until the signing chain exists", () => {
  assert.deepEqual(recipientCreationRefusals({ signingIntermediate: "", signingIntermediateKey: "" }), [
    "Create the signing chain first. The recipient certificate is signed by its intermediate.",
  ]);
  const created = recipientCertificateCommand({
    name: "Studio",
    organization: "Studio",
    intermediateCert: "/certs/intermediate.pem",
    intermediateKey: "/certs/intermediate.key",
    directory: "/certs",
  });
  assert.equal(created.args[0], "certificate");
  assert.equal(created.paths.recipientCert, "/certs/recipient.pem");
  assert.equal(created.paths.recipientKey, "/certs/recipient.key");
});

test("the signer line names the leaf Settings will use", () => {
  assert.match(describeSigner({
    signingCert: "/certs/signer.pem",
    signingKey: "/certs/signer.key",
    signingIntermediate: "/certs/intermediate.pem",
    signingRoot: "/certs/root.pem",
  }), /signer\.pem/);
  assert.match(describeSigner({ signingCert: "", signingKey: "" }), /No signing certificate/);
});
