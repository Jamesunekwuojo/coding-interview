# AI Usage During the Assignment

## 1. How I used AI

I used AI as an engineering assistant during the assignment, not as a replacement for understanding the codebase or making the final decisions.

My main uses were:

- reviewing proposed API and database designs before implementation;
- checking concurrency and transaction decisions;
- helping investigate a runtime integration error;
- helping structure and debug the Playwright test suite;
- reviewing the implementation against the assignment requirements;
- helping identify places where the implementation could be simplified or made safer.

For each case, I treated the AI response as a proposal. I checked it against the assignment README, the existing skeleton, the database schema, generated TypeScript contracts, actual application behavior, and test results before accepting a recommendation.

The most useful pattern was:

**Context → AI suggestion → my decision → implementation → verification**

This allowed me to use AI for speed while still keeping the engineering decisions and validation under my control.

---

## 2. AI use case: Review Plugin API and database contract review

### Context

Before implementing the Review Plugin backend, I had already inspected the assignment requirements, the existing Plugin structure, the database migrations, and the required review workflow.

The proposed Review Plugin operations included:

- `list_criteria`
- `get_summary`
- `list_reviews`
- `get_review`
- `save_review`

The important part of `save_review` was that one operation needed to support both creating a review and editing an existing review.

The save operation also needed to:

- validate the criterion;
- validate the opinion;
- validate evidence;
- prevent evidence from another workspace;
- reject processing/failed materials;
- replace evidence when editing;
- preserve the review ID when editing;
- remain safe under concurrent saves;
- update the review and evidence atomically.

### What I asked AI to review

I asked the AI to critically review the proposed contract and persistence design rather than immediately writing implementation code.

The review focused on:

- authorization ordering;
- evidence validation;
- transaction boundaries;
- create/update races;
- evidence replacement;
- rollback behavior;
- database uniqueness;
- review ID preservation.

The AI specifically recommended using PostgreSQL `INSERT ... ON CONFLICT DO UPDATE` inside a transaction rather than implementing create and update as separate application-level flows.

### What the AI recommended

The main recommendation was to rely on the existing database uniqueness constraint:

```sql
UNIQUE (workspace_id, criterion_id, user_id)
```

and use PostgreSQL upsert behavior for the save operation.

The AI also recommended:

- rejecting duplicate evidence IDs rather than silently deduplicating them;
- validating evidence IDs in one parameterized query;
- requiring every evidence material to belong to the authenticated workspace;
- requiring every evidence material to have `ready` status;
- replacing evidence in the same transaction as the review mutation;
- preserving the existing review ID and `created_at` during edits;
- using the database uniqueness constraint as the final protection against duplicate reviews.

### Prompt/input and related code changes

The input given to the AI included the Review Plugin requirements, the proposed RPC methods, the `save_review` payload, the existing uniqueness constraint, and the requirement that review content and evidence be updated atomically. I specifically asked the AI to review authorization ordering, evidence validation, transaction boundaries, create/update races, rollback behavior, and concurrency rather than immediately writing implementation code.

The main response I used was the recommendation to use PostgreSQL `INSERT ... ON CONFLICT DO UPDATE` with the existing `(workspace_id, criterion_id, user_id)` uniqueness constraint.

The related implementation changes were:

- `save_review` uses the PostgreSQL upsert path;
- review evidence is replaced inside the same transaction;
- duplicate evidence IDs are rejected;
- evidence is validated in one parameterized query;
- the existing review ID and `created_at` are preserved during edits;
- rollback and concurrent-save integration tests were added.

### What I accepted

I accepted these recommendations because they matched the assignment requirements and the existing database model.

The resulting save flow became:

```text
Validate request
→ Validate criterion
→ Validate evidence
→ Begin transaction
→ Insert/update review with ON CONFLICT
→ Replace evidence
→ Commit
```

The database constraint remained:

```sql
UNIQUE (workspace_id, criterion_id, user_id)
```

This gave the application both application-level validation and database-level protection.

### What I changed or rejected

I did not accept every recommendation.

One important change was how criterion existence was handled.

The AI suggested that the fixed criterion IDs could be represented directly in application logic. I chose not to do that.

Instead, the implementation checks the `review_criteria` table.

The reason was simple: the migration already defines the fixed criteria and the database already provides the foreign-key relationship. Hard-coding the same IDs again in Rust would create another source of truth.

I also rejected unnecessary complexity such as:

- advisory locks;
- an additional `SELECT ... FOR UPDATE`;
- separate create/update RPC methods;
- generic repository abstractions introduced only for this feature.

The assignment needed consistency, not a larger concurrency architecture.

### How I verified the result

I did not rely on the AI recommendation by itself.

I implemented focused backend tests for:

- review creation;
- review editing;
- preservation of the review ID;
- evidence replacement;
- invalid evidence;
- rollback;
- concurrent saves.

The final backend suite passed:

```text
37 passed
0 failed
```

The concurrency test specifically verifies that concurrent saves do not create duplicate reviews or leave mixed evidence.

The rollback test verifies that a failed save leaves the previous review and evidence unchanged.

### What this demonstrated

This was the clearest example of using AI for **design review rather than code generation**.

The AI helped challenge the design, but the final implementation was based on:

- the assignment rules;
- the existing schema;
- PostgreSQL behavior;
- the existing application architecture;
- actual tests.

---

## 3. AI use case: Debugging the Review Plugin → DataRoom integration

### Context

After the Review Plugin UI was implemented, I tested the investor evidence-selection flow in the browser.

The Review Plugin needed to retrieve DataRoom materials through the existing Plugin Host boundary.

The initial implementation used:

```ts
host.call<ListMaterialsResponse>(
  "list_materials",
  null,
  { target: "dataroom" },
)
```

The browser request failed with:

```text
400 invalid_input
Invalid list parameters.
```

At this point, changing the backend would have been premature because the DataRoom endpoint was already working.

### What I investigated

The important question was:

> Is the backend contract wrong, or is the Review Plugin sending the wrong parameter shape?

I inspected:

- the generated TypeScript contract;
- the existing DataRoom UI call;
- the Rust DataRoom request type;
- the actual browser error.

The generated contract showed that `list_materials` expects an object containing the optional search field:

```ts
type ListMaterialsParams = {
  search: string | null;
};
```

The existing DataRoom UI also used:

```ts
{ search: search.trim() || null }
```

This gave enough evidence that the Review Plugin was calling the RPC with the wrong shape.

### How AI was used

AI was used to help reason through the integration problem and compare the failing call against the existing contract.

The useful part of the interaction was not simply getting a replacement line of code. The important step was tracing the request through the existing architecture:

```text
Review Plugin UI
→ Plugin Host
→ DataRoom RPC
→ generated TypeScript contract
→ Rust request type
```

### Prompt/input and related code changes

The concrete input to the investigation was the failing browser request, the current Review Plugin call, the generated `ListMaterialsParams` type, the Rust request type, and the existing DataRoom UI call.

The relevant failing code was:

```ts
host.call<ListMaterialsResponse>(
  "list_materials",
  null,
  { target: "dataroom" },
)
```

The evidence from the generated contract and existing DataRoom implementation showed that the request must contain the `search` field. The resulting code change is shown below.

### The fix

The Review Plugin call was changed to:

```ts
host.call<ListMaterialsResponse>(
  "list_materials",
  { search: null },
  { target: "dataroom" },
)
```

Only:

```text
plugins/review/ui/app.tsx
```

needed to be changed for this runtime integration fix.

No backend endpoint, database migration, generated contract, or dependency was changed.

### How I verified it

After the change:

- `pnpm typecheck` passed;
- `pnpm lint` passed;
- `pnpm build` passed;
- backend tests remained at `37/37`;
- `cargo check --locked` passed;
- browser verification confirmed that the DataRoom evidence materials loaded successfully.

This was an important example of using the generated contract as the source of truth rather than changing the backend to accommodate an incorrect client call.

### What this demonstrated

This showed the value of AI for **debugging and tracing an existing architecture**, while still requiring manual inspection of the actual generated contract and application code.

The final fix was small because the problem was not a missing feature. It was an incorrect request shape at an existing boundary.

---

## 4. AI use case: Playwright test design and debugging

### Context

The assignment explicitly required browser tests using the provided Playwright setup.

The initial test planning covered the complete business flow:

```text
Login
→ DataRoom
→ Register/search/read material
→ Review Plugin
→ Select evidence
→ Create review
→ Edit review
→ Check progress
```

I wanted the tests to be organized by feature rather than putting the entire assignment into one large test file.

### Test structure

The final test structure was split into `tests/dataroom.spec.ts` for DataRoom behavior and `tests/review.spec.ts` for Review behavior, with the shared login/logout logic kept in `tests/support/auth.ts`.

The reasoning was:

- DataRoom tests should stay with DataRoom behavior;
- Review tests should stay with Review behavior;
- authentication is shared support code;
- the files should remain readable as the test suite grows.

I deliberately avoided adding more helper layers than necessary.

### What I asked AI to help with

The AI was given the test requirements and asked to help implement the Playwright coverage using:

- the real login flow;
- real application behavior;
- accessible selectors;
- desktop and mobile projects;
- no arbitrary sleeps;
- independent tests;
- the existing Playwright configuration.

The tests covered:

### DataRoom

- company material registration;
- search;
- detail view;
- investor registration restriction.

### Review

- company restriction;
- investor review creation;
- evidence selection;
- review editing;
- `needs_information`;
- investor-to-investor isolation;
- validation boundaries.

The tests also checked the 2,000-character opinion boundary.

### Prompt/input and related code changes

The AI was given the existing Playwright configuration, authentication flow, seeded data, browser projects, required user flows, and the restriction that production code and dependencies should not be changed. The implementation request also required accessible selectors, real login/application behavior, independent tests, and no arbitrary sleeps.

The related code changes were limited to the E2E test area:

- `tests/dataroom.spec.ts`;
- `tests/review.spec.ts`;
- `tests/support/auth.ts`.

When strict-mode failures exposed ambiguous locators, the selectors were refined to match the actual rendered DOM. The final changes used more specific selectors such as `.last()` for the intended modal control and `.divide-y > div` for the intended evidence row instead of weakening the tests with delays.

### What happened during implementation

The initial browser test implementation exposed Playwright strict-mode locator problems.

Some selectors matched more than one element in the rendered page.

AI helped identify and refine those selectors using the actual page structure instead of adding arbitrary delays.

The changes included using more specific selectors and, where appropriate, `.last()` or the relevant structural locator.

No `page.waitForTimeout()` calls were introduced.

### What I accepted

I accepted the selector changes where they matched the actual DOM and user-visible behavior.

I did not accept a solution based on sleeping for an arbitrary amount of time.

The tests should wait for actual application state, visible elements, and user interactions.

### How I verified the result

The final suite was run against both configured Playwright projects:

| Project | Result |
|---|---:|
| Desktop | 6/6 passed |
| Mobile | 6/6 passed |
| Total | 12/12 passed |

The suite was also run after resetting the database and again without a reset.

This helped check that the tests were not dependent on one accidental database state.

### What this demonstrated

AI helped speed up test development and debugging, but the actual acceptance criteria came from the assignment and the real browser results.

---

## 5. AI use beyond the three main examples

I also used AI for inspection and review during the implementation.

One example was a final assignment-compliance review.

The review checked:

- required DataRoom functionality;
- Review Plugin behavior;
- database constraints;
- Plugin/Gen-TS integration;
- frontend role behavior;
- security and isolation;
- test coverage;
- documentation requirements;
- AI usage requirements.

This was an inspection-only step. It was not used to introduce additional features.

The review helped confirm that the implementation was aligned with the required scope before final documentation and submission.

Another architectural review looked at the backend structure after the feature work was complete.

The useful recommendation was to reduce file density by:

- moving backend tests into dedicated `tests.rs` modules;
- extracting meaningful SQLx row models into `models.rs`;
- keeping API/Gen-TS contracts in the existing types modules.

I adopted those changes because they improved readability without introducing generic enterprise-style layers.

The final backend tests remained:

```text
37 passed
0 failed
```

and generated TypeScript remained unchanged by the refactor.

---

## 6.1 Requirement checklist for the three documented examples

Each of the three main examples now records the four things required by the assignment:

1. **Context/input** — what repository state, requirement, or failure was given to the AI.
2. **AI result / related change** — the recommendation and the concrete code or test changes that followed.
3. **My decision** — what I accepted, modified, or rejected.
4. **Verification** — the tests, generated contracts, database behavior, or browser behavior used to confirm the result.

The examples are intentionally concrete rather than describing AI use only in general terms.

---

## 6. How I decided whether to accept AI suggestions

I did not use a rule like "if AI suggests it, implement it."

I generally checked a suggestion against four things:

### 1. Assignment requirement

Does it actually solve a requirement from the README?

### 2. Existing architecture

Does it fit the Plugin, DataRoom, Gen-TS, SQLx, and React Query structure already provided?

### 3. Complexity

Does it solve a real problem, or does it add abstraction that the assignment does not need?

### 4. Verification

Can I prove the result through:

- tests;
- generated contracts;
- database constraints;
- browser behavior;
- or direct inspection of the implementation?

This approach led to both accepting and rejecting AI suggestions.

That distinction is important because the goal was not to maximize the amount of AI-generated code. The goal was to use AI where it could improve the engineering process.

---

## 7. Examples of recommendations I accepted and rejected

| Area | AI suggestion | Decision | Reason |
|---|---|---|---|
| Review save | PostgreSQL `ON CONFLICT DO UPDATE` | Accepted | Fits the existing uniqueness constraint and concurrent-save requirement |
| Evidence | Validate all IDs together | Accepted | Clearer and avoids silently accepting invalid evidence |
| Evidence | Reject duplicate IDs | Accepted | Matches the assignment requirement |
| Review criteria | Hard-code fixed IDs in application logic | Modified/rejected | Database already contains the source of truth |
| Concurrency | Add more locking mechanisms | Rejected | Transaction + unique constraint were sufficient |
| Architecture | Add generic repository/service layers | Rejected | Unnecessary complexity for the current codebase |
| Plugin integration | Correct `list_materials` parameter shape | Accepted | Confirmed by generated TypeScript and existing DataRoom usage |
| Playwright | Refine strict locators | Accepted | Matched actual DOM structure |
| Playwright | Use arbitrary sleeps | Rejected | Makes tests slower and less reliable |

---

## 8. AI and code ownership

AI assisted with parts of the implementation, but I remained responsible for:

- deciding what should be implemented;
- understanding the existing repository;
- choosing the API and database structure;
- reviewing generated suggestions;
- deciding which recommendations to accept;
- changing recommendations where they did not fit the repository;
- validating the implementation;
- running the tests;
- interpreting failures;
- documenting the final design.

I did not treat an AI-generated answer as proof that something was correct.

For example, the `list_materials` integration issue was only considered resolved after the generated contract, existing DataRoom implementation, and browser behavior all agreed with the fix.

Likewise, the concurrency design was only considered complete after the database-backed concurrent-save test passed.

---

## 9. What I learned from using AI during the assignment

The most useful lesson was that AI was more valuable when given enough repository context and a specific engineering question.

A broad request such as:

```text
"Build the review system."
```

would not give the same level of useful reasoning.

The better workflow was closer to:

```text
Here is the assignment requirement.
Here is the existing architecture.
Here is the proposed contract.
Here is the database constraint.
Here is the problem I am trying to solve.

Review the design and point out risks or alternatives.
Do not assume missing requirements.
```

Then I could make the final decision based on the actual repository.

I also found that AI was useful for challenging assumptions I had already made. In the save-review design, for example, the concurrency discussion made the database uniqueness constraint more central to the implementation rather than treating it as a passive schema rule.

---

## 10. Final approach to AI

I treated AI as a development tool with three main roles:

**Design reviewer**
→ challenge API, database, transaction, and concurrency decisions.

**Debugging partner**
→ help trace unexpected behavior through the existing application boundaries.

**Testing assistant**
→ help structure and refine browser tests.

The final implementation was still validated against the actual repository, database, generated contracts, and running application.

The main principle throughout the assignment was:

> **Use AI to increase the speed and quality of reasoning, but do not outsource the responsibility for the result.**
