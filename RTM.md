# Requirements Traceability Matrix (RTM)

## T-QUAL-FSC2: FindSimilarConcepts plateau remediation

| T-ID | PRD FR | Requirement (Atomic) | Component(s) | Test(s) | Status |
|------|--------|----------------------|--------------|---------|--------|
| T-QUAL-FSC2.1 | FR-7.4 | Tokenization MUST be compatible with model vocab/IDs; mismatches fail closed. | src/Utils/VectorTools.Onnx.cs, src/Utils/VectorTools.Embeddings.cs | tests/Maenifold.Tests/TQualFsc2TokenizerDiagnosticsTests.cs | In Progress |
| T-QUAL-FSC2.2 | FR-7.4 | FindSimilarConcepts MUST reject degenerate embeddings and similarity plateaus. | src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/FindSimilarConceptsPlateauRegressionTests.cs | In Progress |
| T-QUAL-FSC2.3 | FR-7.4 | Similarity output MUST be bounded to <= 1.000. | src/Tools/VectorSearchTools.cs | tests/Maenifold.Tests/FindSimilarConceptsPlateauRegressionTests.cs | In Progress |
| T-QUAL-FSC2.4 | FR-7.4 | Diagnostics MUST identify embedding collapse (hash duplicates / zero-distance counts). | tests/Maenifold.Tests/TQualFsc2HardMeasurementsTests.cs | tests/Maenifold.Tests/TQualFsc2HardMeasurementsTests.cs | In Progress |
