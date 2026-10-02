export function contentKeysFrom(fields) {
  const refusals = [];
  if (!fields.kdm && !fields.recipientKey && !fields.keys) refusals.push("Choose a KDM and its recipient private key, or a KEYS.json");
  if (fields.kdm && !fields.recipientKey) refusals.push("A KDM needs the recipient private key it was issued to");
  if (fields.recipientKey && !fields.kdm) refusals.push("A recipient private key needs a KDM");

  if (refusals.length > 0) return { refusals };
  return {
    contentKeys: {
      kdm: fields.kdm || null,
      recipient_key: fields.recipientKey || null,
      keys: fields.keys || null,
    },
  };
}
