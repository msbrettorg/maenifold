# Requirements Traceability Matrix

| Requirement | Component | Test Case | Type |
|-------------|-----------|-----------|------|
| FR-1 | src/auth/ | TC-1: login returns JWT | integration |
| FR-1 | src/auth/ | TC-2: expired token rejected | unit |
| FR-2 | src/validation/ | TC-3: rejects malformed input | unit |
| FR-2 | src/validation/ | TC-4: accepts valid payload | integration |
| NFR-1 | src/api/ | TC-5: p99 latency under 200ms | benchmark |
