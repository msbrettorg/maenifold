// Pure helpers for composing the embedding-model activation confirm-dialog
// warning. Extracted from SettingsEmbeddings.vue so the warning logic can be
// unit-tested without mounting the component.

/**
 * Warning text shown when activating a restricted-license model. Kept identical
 * to the previous inline string so existing UX copy is unchanged.
 */
export const RESTRICTED_LICENSE_ACTIVATION_WARNING =
  "Restricted-license activations require backend opt-in if the daemon enforces license confirmation.";

/**
 * Returns a dimension-change warning when the model being activated has a
 * different embedding dimension than the currently active model. Switching
 * dimensions invalidates every stored embedding, so semantic search is broken
 * until vectors are regenerated.
 *
 * Returns `null` when either dimension is unknown (no active model yet) or when
 * the dimensions match (activating a same-dimension model is non-destructive).
 */
export function dimensionChangeWarning(
  targetDimensions: number | null | undefined,
  currentDimensions: number | null | undefined,
): string | null {
  if (
    typeof targetDimensions !== "number" ||
    typeof currentDimensions !== "number"
  ) {
    return null;
  }

  if (targetDimensions === currentDimensions) {
    return null;
  }

  return (
    `Activating a ${targetDimensions}-dimension model replaces the current ` +
    `${currentDimensions}-dimension model. Existing embeddings are incompatible ` +
    `and must be regenerated before semantic search works again.`
  );
}

/**
 * Composes the activation confirm-dialog warning from the dimension-change and
 * restricted-license signals. When both apply, the dimension warning leads
 * (it is the more consequential, data-invalidating change) and the
 * restricted-license note is appended — neither is silently dropped.
 */
export function composeActivationWarning(options: {
  targetDimensions: number | null | undefined;
  currentDimensions: number | null | undefined;
  restrictedLicense: boolean;
}): string | null {
  const dimWarning = dimensionChangeWarning(
    options.targetDimensions,
    options.currentDimensions,
  );
  const licenseWarning = options.restrictedLicense
    ? RESTRICTED_LICENSE_ACTIVATION_WARNING
    : null;

  if (dimWarning && licenseWarning) {
    return `${dimWarning} ${licenseWarning}`;
  }

  return dimWarning ?? licenseWarning;
}
