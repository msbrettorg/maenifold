# Product Requirements Document: Maenifold Embeddings Quality

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-31 | PM Agent | Net-new PRD for T-QUAL-FSC2 (FindSimilarConcepts embedding quality) |

---

## 1. Executive Summary

**Product Area**: Maenifold graph embeddings and semantic similarity
**Objective**: Eliminate similarity plateaus caused by embedding collapse, ensuring FindSimilarConcepts returns meaningful rankings for valid inputs.

---

## 2. Problem Statement

Current `FindSimilarConcepts` results can degenerate into a similarity plateau (many unrelated concepts at 1.000 similarity) when embeddings collapse to identical vectors. This undermines semantic search quality and user trust.

---

## 3. Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-7.4 | System SHALL discover semantically similar concepts via embeddings and return meaningful rankings for valid inputs. | P1 |

---

## 4. Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-7.4.1 | Tokenization MUST be compatible with the embedding model’s vocab and IDs; mismatches SHALL fail closed. | Required |
| NFR-7.4.2 | Similarity output SHALL NOT exceed 1.000. | Required |

---

## 5. Out of Scope

- Changing the embedding model itself.
- Introducing new ranking algorithms beyond cosine similarity.
