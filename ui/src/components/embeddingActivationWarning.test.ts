import { describe, expect, it } from "vitest";

import {
  RESTRICTED_LICENSE_ACTIVATION_WARNING,
  composeActivationWarning,
  dimensionChangeWarning,
} from "./embeddingActivationWarning";

describe("dimensionChangeWarning", () => {
  it("warns when activating a model with different dimensions", () => {
    // bge-large-en-v1.5 (1024d) -> all-MiniLM-L6-v2 (384d)
    const warning = dimensionChangeWarning(384, 1024);
    expect(warning).not.toBeNull();
    expect(warning).toContain("384-dimension");
    expect(warning).toContain("1024-dimension");
    expect(warning).toContain("must be regenerated");
  });

  it("returns null when dimensions match", () => {
    expect(dimensionChangeWarning(1024, 1024)).toBeNull();
  });

  it("returns null when there is no active model (current unknown)", () => {
    expect(dimensionChangeWarning(384, null)).toBeNull();
    expect(dimensionChangeWarning(384, undefined)).toBeNull();
  });

  it("returns null when the target dimension is unknown", () => {
    expect(dimensionChangeWarning(null, 1024)).toBeNull();
  });
});

describe("composeActivationWarning", () => {
  it("returns only the dimension warning when license is permissive", () => {
    const warning = composeActivationWarning({
      targetDimensions: 384,
      currentDimensions: 1024,
      restrictedLicense: false,
    });
    expect(warning).toContain("384-dimension");
    expect(warning).not.toContain(RESTRICTED_LICENSE_ACTIVATION_WARNING);
  });

  it("returns only the restricted-license warning when dimensions match", () => {
    const warning = composeActivationWarning({
      targetDimensions: 1024,
      currentDimensions: 1024,
      restrictedLicense: true,
    });
    expect(warning).toBe(RESTRICTED_LICENSE_ACTIVATION_WARNING);
  });

  it("concatenates both warnings when both apply, dimension first", () => {
    const warning = composeActivationWarning({
      targetDimensions: 384,
      currentDimensions: 1024,
      restrictedLicense: true,
    });
    expect(warning).not.toBeNull();
    const dimIndex = warning!.indexOf("384-dimension");
    const licenseIndex = warning!.indexOf(
      RESTRICTED_LICENSE_ACTIVATION_WARNING,
    );
    expect(dimIndex).toBeGreaterThanOrEqual(0);
    expect(licenseIndex).toBeGreaterThan(dimIndex);
  });

  it("returns null when neither warning applies", () => {
    expect(
      composeActivationWarning({
        targetDimensions: 1024,
        currentDimensions: 1024,
        restrictedLicense: false,
      }),
    ).toBeNull();
  });
});
