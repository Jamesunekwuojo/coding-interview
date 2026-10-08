# Implementation Overview

## 1. What I implemented

The assignment was implemented around the required flow:

**Company registers materials → Investor views available materials → Investor selects evidence → Investor creates a review for a criterion → Investor can edit the review/evidence → Investor checks review progress**

I kept the implementation within the architecture already provided by the skeleton instead of introducing a separate application structure.

The main responsibilities are split as follows:

- **DataRoom** owns document/material management.
- **Review Plugin** owns review criteria, investor reviews, evidence selection, and review progress.
- **PostgreSQL** stores documents, reviews, criteria, and evidence relationships.
- **Plugin Host and Gen-TS** connect the Review Plugin UI to the backend RPC contracts.
- **React Query** handles server state and cache invalidation in the UI.
- **Playwright** verifies the main browser workflows.

The important part of the implementation was not only getting the screens to work, but keeping the existing boundaries intact and making the server the final authority for authorization, validation, and persistence.

---

## 2. How the application pieces connect

I kept the existing request flow from the skeleton.

### DataRoom flow

Browser  
→ DataRoom UI  
→ Plugin Host / application RPC boundary  
→ Rust DataRoom dispatcher  
→ DataRoom operation  
→ SQLx  
→ PostgreSQL

The DataRoom is responsible for document-related operations such as listing, searching, reading, and registering materials.

### Review flow

Browser  
→ Review Plugin UI  
→ Plugin Host  
→ Review Plugin RPC  
→ Rust Review Plugin server  
→ SQLx  
→ PostgreSQL

The Review Plugin does not directly access the DataRoom implementation or its database queries.

When an investor needs to select evidence, the Review Plugin uses the existing Plugin Host boundary to request DataRoom materials:

```ts
host.call(
  "list_materials",
  { search: null },
  { target: "dataroom" }
)
```

This keeps the ownership of document data inside DataRoom while still allowing the Review Plugin to use that functionality.

---

## 3. DataRoom implementation

The DataRoom handles the document side of the assignment.

I implemented:

- material listing;
- title search;
- material detail retrieval;
- company-only material registration;
- workspace-scoped access;
- material status display.

The supported material statuses are:

- `ready`
- `processing`
- `failed`

A newly registered `.txt` or `.md` file is stored as `ready`, which matches the assignment's scope. The existing `processing` and `failed` records are retained as sample states for the UI and evidence validation.

### Registration

The registration flow starts in the browser.

The user selects a supported `.txt` or `.md` file. The browser reads the file as a UTF-8 string and sends the title, filename, and content through the existing RPC mechanism.

The backend then derives:

- the authenticated user;
- the authenticated workspace;
- the uploader;
- the initial `ready` status;
- a new material ID.

The client does not get to choose those authorization-related values.

The backend also remains responsible for enforcing the request and input rules. The frontend performs an early file-size/type check for a better user experience, but that check is not treated as the security boundary.

### Material listing

The list endpoint is workspace-scoped and supports title search.

The results are ordered by:

1. creation time descending;
2. material ID ascending when creation times are equal.

The list response uses a lightweight material projection. Full document content is fetched only when a user opens a material detail view.

This avoids loading the full body of every document just to display the list.

### React Query

The DataRoom material query includes the current search value:

```text
["dataroom", workspaceId, "materials", search]
```

This means different searches do not incorrectly share the same cached result.

After successful registration, the relevant material queries are invalidated so the new material appears without requiring a full page reload.

---

## 4. Review Plugin implementation

The Review Plugin owns the investor review workflow.

The implemented operations cover:

- retrieving the fixed review criteria;
- retrieving the current investor's progress;
- listing the current investor's reviews;
- retrieving a review and its evidence;
- creating a review;
- editing an existing review;
- saving review evidence.

The two allowed review statuses are:

```text
satisfied
needs_information
```

A `needs_information` review still counts as completed progress. This is important because progress represents whether an investor has reviewed a criterion, not whether the investor is satisfied with it.

The review status therefore remains separate from overall investment approval or company-wide consensus.

---

## 5. Review criteria

The three criteria are provided by the initial migration and are treated as fixed data:

1. Business Understanding
2. Team Composition
3. Revenue Status

The API retrieves these records from the `review_criteria` table and explicitly orders them by `display_order`.

The application does not provide functionality for creating or editing criteria because that is outside the assignment scope.

Keeping the criteria in the database also avoids creating a second source of truth by hard-coding the criterion records separately in the application.

---

## 6. Review API and Gen-TS

### Practical request path

A concrete investor review request follows this path:

```text
Rust Review DTO
→ ts-rs / Gen-TS
→ generated TypeScript type
→ Review Plugin `host.call(...)`
→ Plugin RPC dispatcher
→ Review server operation
→ SQLx
→ PostgreSQL
→ typed response
→ React Query cache
→ Review UI
```

For evidence selection, the Plugin uses the same Host boundary to call the DataRoom target. The Review Plugin therefore consumes DataRoom-owned material data without copying the DataRoom's database logic into the Plugin.


The backend defines typed Rust DTOs for the Review Plugin operations.

The general contract flow is:

Rust DTO  
→ Gen-TS  
→ generated TypeScript contract  
→ Plugin Host call  
→ React Query  
→ UI

For example, the save request contains only review information controlled by the investor:

```ts
type SaveReviewParams = {
  criterionId: string;
  status: ReviewStatus;
  opinion: string;
  evidenceMaterialIds: string[];
};
```

The request does not contain:

- authenticated user ID;
- user role;
- workspace ID.

Those values are derived from the authenticated server-side session.

This is important because accepting those values from the browser would allow the client to try to change the identity or workspace under which the operation is performed.

The generated TypeScript contracts were regenerated from the Rust API definitions. The generated files were not manually edited.

---

## 7. Authorization and workspace isolation

The assignment defines one workspace as the DataRoom access boundary.

The backend therefore derives the authenticated workspace from the server-side session and scopes business operations to that workspace.

The main permission model is:

| Action | Company | Investor |
|---|---:|---:|
| View materials | Yes | Yes |
| Register materials | Yes | No |
| View review progress | No | Yes |
| Create/edit reviews | No | Yes |
| View another investor's reviews | No | No |

The Review Plugin also follows the requirement that company users cannot access private investor reviews.

For example:

- company users receive an empty review list;
- company users cannot retrieve an investor's review;
- company users cannot create or edit reviews;
- investor-only queries are not unnecessarily triggered by the company UI.

For investors, review queries are scoped using both:

- authenticated user ID;
- authenticated workspace ID.

This means one investor's review data is not used as another investor's progress or review list.

The frontend also includes the user and workspace information in the relevant React Query keys and Plugin instance isolation. This prevents cached state from one investor from being reused when another investor enters the Plugin.

The frontend role checks are for user experience and request gating. The backend authorization remains the final security boundary.

---

## 8. Evidence selection and validation

Evidence is connected using material IDs rather than filenames.

The Review Plugin retrieves materials through the DataRoom Plugin Host boundary.

Only materials with `ready` status are selectable in the UI.

The server performs the authoritative validation again when the review is saved.

The save operation requires:

- at least one evidence material;
- no duplicate material IDs;
- every referenced material must exist;
- every referenced material must belong to the authenticated workspace;
- every referenced material must have `ready` status.

This means the following are rejected:

- duplicate evidence IDs;
- non-existent materials;
- materials from another workspace;
- processing materials;
- failed materials.

Evidence validation is done with parameterized SQL and the number of valid rows is compared with the number requested.

This prevents invalid evidence from being silently ignored.

---

## 9. Review creation and editing

Creating and editing a review use the same `save_review` operation.

### Creating a review

The investor selects an unreviewed criterion and provides:

- review status;
- opinion;
- one or more evidence materials.

The backend creates the review and its evidence relationships.

### Editing a review

When an existing review is edited:

- the review ID stays the same;
- the original `created_at` stays the same;
- the status can change;
- the opinion can change;
- the evidence set can change;
- `updated_at` changes.

This matches the requirement that an investor has one review per criterion and editing should update that review rather than create another one.

The database uniqueness constraint is:

```sql
UNIQUE (workspace_id, criterion_id, user_id)
```

This provides database-level protection for the one-review-per-investor-per-criterion rule.

---

## 10. Transaction handling

Review content and evidence relationships represent one logical update, so they are changed in one database transaction.

The save flow is effectively:

Begin transaction  
→ Validate criterion and evidence  
→ Create or update review  
→ Remove previous evidence relationships  
→ Insert the new evidence relationships  
→ Commit

If one of the database operations fails, the transaction is rolled back.

This means an unsuccessful update cannot leave the review with new content while keeping only part of the new evidence.

A rollback test was added to verify that the original review and evidence remain intact after a failed save.

---

## 11. Concurrent saves

Concurrent saves were treated as a database consistency problem rather than only a frontend problem.

The review save uses PostgreSQL:

```sql
INSERT ... ON CONFLICT DO UPDATE
```

together with the existing unique constraint:

```sql
UNIQUE (workspace_id, criterion_id, user_id)
```

This gives the create/update operation an atomic database-level path when two saves target the same investor and criterion.

A concurrency test executes two saves for the same criterion and checks that:

- duplicate reviews are not created;
- the final review is valid;
- the evidence belongs to one consistent save rather than becoming a mixed set.

I did not introduce advisory locks, an additional `SELECT ... FOR UPDATE`, or a queue for this operation. Saving a review is a short synchronous database transaction, so the database constraint and transaction were enough for the consistency requirements of this assignment.

---

## 12. Input validation

The backend validates the review fields before persistence.

The opinion must:

- contain at least one non-whitespace character;
- not exceed 2,000 characters.

The evidence list must contain at least one material and must not contain duplicates.

The criterion must exist in the fixed criteria table.

Authorization is checked before write-input validation. This follows the assignment requirement that a request without write permission should be rejected before its input is processed as a valid business request.

The frontend performs similar checks to give the investor immediate feedback, but those checks are not relied upon by the backend.

---

## 13. Frontend review experience

The Review Plugin UI provides:

- personal progress;
- the three criteria;
- review status;
- review opinion;
- review details;
- linked evidence;
- review creation;
- review editing.

The UI separates:

- not reviewed;
- `satisfied`;
- `needs_information`.

The progress view counts both saved statuses as completed, while still showing the actual status of each review.

### Save failures

If saving fails, the editor stays open and preserves:

- selected criterion;
- status;
- opinion;
- selected evidence.

This allows the investor to correct the problem and retry instead of losing their work.

### Successful saves

After a successful save:

- the review editor closes;
- the form is reset;
- scoped review queries are invalidated;
- progress and review lists can refresh from the server;
- success feedback is shown.

---

## 14. DataRoom and Review Plugin integration

One of the important parts of the implementation was keeping the Plugin boundary clear.

The Review Plugin needs DataRoom materials, but it does not take ownership of them.

The relationship is:

DataRoom  
→ owns materials  
→ exposes material access through its existing RPC boundary  
→ Plugin Host  
→ Review Plugin selects material IDs as evidence  
→ Review API validates those IDs against DataRoom data

This means the UI can reuse DataRoom functionality without creating another material API or duplicating material state inside the Review Plugin.

The server still checks the workspace and material status when the review is saved, so the UI selection is never treated as trusted authorization data.

---

## 15. Frontend state and caching

The existing React Query setup was reused rather than introducing another state-management library.

Review queries use scoped keys containing the authenticated user and workspace.

This is important for a multi-user workflow because review progress is personal.

For example, the application should not reuse:

```text
investor A's review progress
```

when:

```text
investor B
```

enters the same workspace.

The Plugin lifecycle also provides user/workspace-specific isolation when the Plugin is mounted again.

---

## 16. Error and UI states

The UI distinguishes between:

- loading;
- successful empty results;
- search with no results;
- request errors;
- successful mutations.

A failed request is not presented as an empty list or as completed review progress.

The DataRoom and Review Plugin also reuse the existing UI kit, localization system, design tokens, and responsive layout patterns from the skeleton.

The UI was checked for:

- small screens;
- form labels;
- keyboard accessibility;
- refresh/re-entry;
- role-specific rendering.

---

## 17. Backend structure

During implementation, the backend was also refactored to keep the feature code readable without introducing unnecessary application layers.

The main boundary is:

HTTP/RPC handler  
→ Plugin dispatcher  
→ Business operation  
→ SQLx queries

The HTTP/RPC handler remains a thin adapter.

Authorization and Plugin routing stay at the Plugin/DataRoom boundary.

Meaningful SQLx row projections were moved into dedicated model structures, while API and Gen-TS contracts remain in the existing types modules.

Database-backed tests were moved into test modules so the main server files focus on the application logic.

I intentionally avoided adding generic repository/service abstractions where the existing structure was already sufficient.

---

## 18. Database persistence

The runtime data is stored in PostgreSQL.

The implementation does not use the sample JSON as a runtime database and does not use browser storage for business data.

The database stores:

- materials;
- reviews;
- review criteria;
- review-to-material evidence relationships.

The important relationships and constraints are enforced by PostgreSQL, including:

- workspace foreign keys;
- review criterion foreign keys;
- material foreign keys;
- review/evidence relationships;
- one review per investor per criterion;
- valid review status;
- valid material status;
- opinion length constraints.

SQL queries use parameter binding rather than string interpolation for user-controlled values.

---

## 19. Testing approach

The backend uses real PostgreSQL-backed tests for the business rules that depend on persistence, authorization, transactions, and database constraints.

The tests cover:

- authorization;
- input validation;
- error handling;
- persistence;
- review creation;
- review editing;
- evidence validation;
- evidence replacement;
- transaction rollback;
- concurrent saves;
- investor isolation.

The frontend uses the provided Playwright setup.

The browser tests are split by feature. `tests/dataroom.spec.ts` covers DataRoom behavior, `tests/review.spec.ts` covers Review behavior, and `tests/support/auth.ts` contains the shared login/logout logic.

The tests use the real login flow and real application APIs rather than replacing the backend with mocked business behavior.

The suite runs against both the desktop and mobile Playwright projects.

The exact commands, test data, reset strategy, results, and limitations are documented separately in [`testing.md`](./testing.md).

---

## 20. Important implementation decisions

### Alternatives I considered

For the main design choices, I considered simpler and more complex alternatives before settling on the current implementation:

- **Review save:** a separate `SELECT` followed by `INSERT`/`UPDATE` was possible, but the existing uniqueness constraint plus PostgreSQL upsert gives a safer create/update path for concurrent saves.
- **Concurrency:** advisory locks and `SELECT ... FOR UPDATE` were considered but were not necessary for this short transaction.
- **Backend structure:** service/repository layers could have been introduced, but the existing modules were small enough that a targeted split into `models.rs` and `tests.rs` was clearer.
- **Review processing:** a queue or worker would be appropriate for long-running work, but review saving is a short synchronous database operation.
- **Search:** substring search could later use PostgreSQL `pg_trgm`, but the assignment does not justify adding that complexity.
- **API style:** REST or a separate frontend API could have been added, but the repository already provides RPC and Plugin Host boundaries, so introducing another communication pattern would work against the existing architecture.

### Keep DataRoom and Review responsibilities separate

Document management remains in DataRoom while review functionality remains in the Review Plugin. This follows the boundaries already provided by the skeleton.

### Use the database for important invariants

The one-review-per-investor-per-criterion rule is protected by a PostgreSQL uniqueness constraint instead of relying only on application checks.

### Validate evidence on the server

The UI only allows `ready` materials to be selected, but the backend checks workspace ownership and status again during save.

### Keep review and evidence updates transactional

Review content and its evidence relationships form one logical update, so they are persisted together.

### Use the existing Plugin and Gen-TS flow

No separate frontend API or communication mechanism was introduced. The implementation extends the provided RPC, Plugin Host, and generated TypeScript architecture.

### Avoid unnecessary abstractions

The backend was kept explicit and close to the existing skeleton. I extracted meaningful SQLx models and test modules where they improved readability, but did not introduce generic repository or service layers without a concrete need.

### No queue for review saving

Review saving is a short synchronous database operation. A background queue would add complexity without solving a requirement in this assignment.

---

## 21.1 Other changes in understanding

There were a few smaller changes in understanding that affected implementation decisions:

- The initial Docker failure looked like a missing repository file, but investigation showed that the real problem was the host SELinux context on the bind mount.
- The pnpm installation failure looked like a dependency problem, but the actual cause was the read-only workspace mount and where pnpm needed to create `node_modules`.
- Database-backed tests initially showed interference when tests used the same seeded records in parallel. The tests were adjusted to use temporary users/records where isolation was needed instead of globally serializing the test suite.
- An applied migration was changed during development and PostgreSQL migration checksum validation caught it. The disposable development database was reset, and the later practice was to treat applied migrations as immutable and use a new migration for schema changes.

These were not just debugging notes; they changed how I approached the environment, test isolation, and database migration workflow.

---

## 21. What changed during implementation

The implementation did not remain exactly the same as the initial plan.

One example was the Review Plugin evidence request.

The first browser implementation called the DataRoom material RPC with `null` parameters. The API correctly rejected the request with:

```text
400 invalid_input
Invalid list parameters.
```

I traced this through the generated TypeScript contract and the existing DataRoom implementation. The generated contract showed that the RPC expects an object containing the optional `search` field.

The call was therefore corrected to:

```ts
host.call(
  "list_materials",
  { search: null },
  { target: "dataroom" }
)
```

No backend or API contract change was necessary.

This was a useful example of why the generated contract and existing application code should be treated as the source of truth when integrating a Plugin.

Other implementation decisions changed after reviewing concurrency and transaction behavior with AI, but those are documented separately in [`ai-usage.md`](./ai-usage.md).

---

## 22. What was not implemented

The optional AI review-draft feature was not implemented.

The assignment explicitly marks this feature as optional and states that it has no additional bonus points. I therefore prioritized completing and testing the required manual review workflow.

Other functionality outside the assignment scope was also not added, including:

- user registration;
- external OAuth;
- MFA;
- account recovery;
- workspace/DataRoom creation;
- document deletion;
- review criteria creation/editing;
- Plugin installation/management UI;
- actual file storage;
- PDF/OCR/conversion;
- real-time collaboration;
- payments;
- cloud deployment.

The provided `processing` and `failed` document states remain fixed sample states as intended by the assignment.

---

## 23. Further documentation

The repository documentation is split by purpose:

- [`README.md`](../README.md) — quick start, accounts, scope, verification, and submission information.
- [`approach.md`](../approach.md) — the development journey, decisions, debugging notes, and changes in understanding.
- [`testing.md`](./testing.md) — reproducible setup, test strategy, database reset/isolation, commands, results, and limitations.
- [`ai-usage.md`](./ai-usage.md) — concrete examples of how AI was used during design, debugging, and testing.

The initial implementation plan was created before development and remains preserved in Git history.
