export function prefilledRecipientKey(preferences, currentValue) {
  return currentValue || preferences.recipientKey || "";
}
