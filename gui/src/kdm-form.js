function fileName(path) {
  const name = String(path || "").split(/[/\\]/).pop();
  return name || "";
}

export function joinPath(directory, name) {
  const sep = directory.includes("\\") && !directory.includes("/") ? "\\" : "/";
  return `${directory.replace(/[/\\]+$/, "")}${sep}${name}`;
}

export function chainPaths(directory) {
  return {
    signingCert: joinPath(directory, "signer.pem"),
    signingKey: joinPath(directory, "signer.key"),
    signingIntermediate: joinPath(directory, "intermediate.pem"),
    signingIntermediateKey: joinPath(directory, "intermediate.key"),
    signingRoot: joinPath(directory, "root.pem"),
  };
}

export function recipientPaths(directory) {
  return {
    recipientCert: joinPath(directory, "recipient.pem"),
    recipientKey: joinPath(directory, "recipient.key"),
  };
}

export function signingChainCommand(organization, directory) {
  return ["certificate", "chain", "--organization", organization, "--output", directory];
}

export function recipientCertificateCommand(fields) {
  const paths = recipientPaths(fields.directory);
  return {
    args: [
      "certificate", "generate", "-t", "leaf",
      "--cn", fields.name,
      "--organization", fields.organization || fields.name,
      "--issuer-cert", fields.intermediateCert,
      "--issuer-key", fields.intermediateKey,
      "--output-cert", paths.recipientCert,
      "--output-key", paths.recipientKey,
    ],
    paths,
  };
}

export function recipientCreationRefusals(prefs) {
  if (prefs.signingIntermediate && prefs.signingIntermediateKey) return [];
  return ["Create the signing chain first. The recipient certificate is signed by its intermediate."];
}

export function signingChainFrom(prefs) {
  return [prefs.signingIntermediate, prefs.signingRoot].filter((path) => path);
}

export function describeSigner(prefs) {
  if (!prefs.signingCert || !prefs.signingKey) return "No signing certificate in Settings yet.";
  if (!prefs.signingIntermediate || !prefs.signingRoot) {
    return "Settings has a signing leaf and no chain. A KDM needs the intermediate and the root too.";
  }
  return `Signs with ${fileName(prefs.signingCert)} from Settings.`;
}

// a KDM timestamp is 25 characters with a numeric offset and no Z
export function kdmTimestamp(localValue, offsetMinutes) {
  const match = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})(?::(\d{2}))?$/.exec(localValue || "");
  if (!match) return "";
  const offset = offsetMinutes || 0;
  const sign = offset >= 0 ? "+" : "-";
  const abs = Math.abs(offset);
  const hours = String(Math.floor(abs / 60)).padStart(2, "0");
  const minutes = String(abs % 60).padStart(2, "0");
  return `${match[1]}T${match[2]}:${match[3] || "00"}${sign}${hours}:${minutes}`;
}

function signerArgs(fields) {
  const refusals = [];
  if (!fields.signerCert || !fields.signerKey) {
    refusals.push("Set the signing certificate and key in Settings");
  }
  if (!fields.signerIntermediate || !fields.signerRoot) {
    refusals.push("Set the signing intermediate and root in Settings");
  }
  const args = [];
  if (fields.signerCert) args.push("--signer-cert", fields.signerCert);
  if (fields.signerKey) args.push("--signer-key", fields.signerKey);
  if (fields.signerIntermediate) args.push("--signer-chain", fields.signerIntermediate);
  if (fields.signerRoot) args.push("--signer-chain", fields.signerRoot);
  return { args, refusals };
}

function forensicArgs(fields) {
  const args = [];
  const refusals = [];
  if (fields.formulation) args.push("--formulation", fields.formulation);
  if (fields.noForensicPicture) args.push("--disable-forensic-marking-picture");
  if (fields.audioMarking === "off") args.push("--disable-forensic-marking-audio");
  if (fields.audioMarking === "above") {
    if (fields.audioMarkingChannel === "" || fields.audioMarkingChannel == null) {
      refusals.push("Name the channel where audio marking stops");
    } else {
      args.push("--disable-forensic-marking-audio", String(fields.audioMarkingChannel));
    }
  }
  return { args, refusals };
}

function windowArgs(fields, required) {
  const refusals = [];
  const from = kdmTimestamp(fields.validFrom, fields.offsetMinutes);
  const to = kdmTimestamp(fields.validTo, fields.offsetMinutes);
  const any = fields.validFrom || fields.validTo;
  if (required && (!fields.validFrom || !fields.validTo)) {
    refusals.push("A KDM needs a validity window");
  } else if (any && (!fields.validFrom || !fields.validTo)) {
    refusals.push("Set both dates, or leave both empty to keep the DKDM window");
  } else if (any) {
    if (!from) refusals.push("Valid from is not a date");
    if (!to) refusals.push("Valid to is not a date");
  }
  const args = [];
  if (from && to && refusals.length === 0) args.push("-f", from, "-t", to);
  return { args, refusals };
}

const SIGNER = {
  signerCert: "/certs/signer.pem",
  signerKey: "/certs/signer.key",
  signerIntermediate: "/certs/intermediate.pem",
  signerRoot: "/certs/root.pem",
};

export function kdmRequest(fields) {
  const recipient = fields.forThisMachine ? fields.recipientFromSettings : fields.recipientCert;
  const refusals = [];
  if (!fields.cplId) refusals.push("A KDM needs the CPL id");
  if (!fields.contentTitle) refusals.push("A KDM needs the content title");
  if (!recipient) {
    refusals.push(fields.forThisMachine
      ? "Set this machine's recipient certificate in Settings"
      : "A KDM needs the recipient certificate");
  }
  if (!fields.keys) refusals.push("A KDM needs the keys file from the encrypted build");
  if (!fields.output) refusals.push("Choose where to write the KDM");
  const signer = signerArgs(fields);
  const forensic = forensicArgs(fields);
  const window = windowArgs(fields, true);
  refusals.push(...signer.refusals, ...forensic.refusals, ...window.refusals);
  return {
    refusals,
    args: [
      "kdm",
      "--cpl-id", fields.cplId,
      "--content-title", fields.contentTitle,
      "--cert", recipient,
      ...signer.args,
      "--keys", fields.keys,
      ...window.args,
      ...forensic.args,
      "-o", fields.output,
    ],
  };
}

export function rewrapRequest(fields) {
  const refusals = [];
  if (!fields.dkdm) refusals.push("Choose the DKDM");
  if (!fields.dkdmKey) refusals.push("A DKDM needs the private key it was issued to");
  if (!fields.recipientCert) refusals.push("Choose the new recipient certificate");
  if (!fields.output) refusals.push("Choose where to write the KDM");
  const signer = signerArgs(fields);
  const forensic = forensicArgs(fields);
  const window = windowArgs(fields, false);
  refusals.push(...signer.refusals, ...forensic.refusals, ...window.refusals);
  return {
    refusals,
    args: [
      "kdm-rewrap",
      "--dkdm", fields.dkdm,
      "--dkdm-key", fields.dkdmKey,
      "--cert", fields.recipientCert,
      ...signer.args,
      ...window.args,
      ...forensic.args,
      "-o", fields.output,
    ],
  };
}

export const SIGNER_FIELDS = SIGNER;

export function packageSignerArgs({ signingCert, signingKey, signingChain }) {
  if (!signingCert || !signingKey) return [];
  const chainArgs = signingChain.flatMap((path) => ["--signer-chain", path]);
  return ["--signer-cert", signingCert, "--signer-key", signingKey, ...chainArgs];
}
