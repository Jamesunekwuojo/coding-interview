# Testing and Reproduction

## 1. Environment

The assignment is designed to run through the provided Docker Compose setup.

The main requirements are:

- Docker Compose v2
- `make`

The application, API, and PostgreSQL database run inside the provided Docker environment.

I did most of the development and verification inside the Docker environment rather than installing the backend and database stack directly on the host.

---

## 2. Start the application

From the repository root:

```bash
make dev
```

This starts:

- PostgreSQL (`db`)
- Rust API (`api`)
- React web application (`web`)

The web application is available at:

```text
http://localhost:5178/
```

The provided login/session system is used to access the application.

The first startup can take longer because Docker may need to build images and install dependencies.

---

## 3. Test accounts

The assignment provides three accounts.

| Role | Email | Password |
|---|---|---|
| Company Representative | `company@lighthouse.test` | `dataroom` |
| Investor | `investor@lighthouse.test` | `dataroom` |
| Investor | `peer@lighthouse.test` | `dataroom` |

The accounts belong to the `lighthouse` workspace.

These seeded users are also used when verifying role-based access and investor-to-investor isolation.

---

## 4. Database reset

To return the assignment database to its initial state:

```bash
make reset-db
```

This stops the services and removes the assignment database data before recreating the provided initial data.

I used the reset command when a clean seeded database was required for verification.

The reset strategy is especially useful for browser testing because the Playwright tests operate against real application and database state.

---

## 5. Useful development commands

### View application logs

```bash
make logs
```

This follows the Docker Compose service logs.

Press `Ctrl+C` to stop viewing the logs.

### Stop the application

```bash
make stop
```

This stops the services while preserving database data.

### Generate TypeScript contracts

After changing Rust API contracts:

```bash
make gen-ts-docker
```

The generated TypeScript contracts are then used by the frontend and Plugin UI.

### Run the repository checks

```bash
make check-docker
```

This checks the generated output, linting, TypeScript types, and frontend build.

### Run browser tests

```bash
make test-e2e
```

This runs the Playwright tests using the configured desktop and mobile browser projects.

---

## 5.1 Test data and isolation

The browser and backend verification use the seeded `lighthouse` workspace and the provided users:

- `company@lighthouse.test`
- `investor@lighthouse.test`
- `peer@lighthouse.test`

The seeded materials include ready, failed, and processing states so evidence validation can be tested against each required status.

The backend tests use real PostgreSQL persistence. They do not replace database behavior with an in-memory mock.

For tests that could interfere with the shared seeded records, temporary users or unique test data are used. This allows Rust tests to run in parallel without making the whole suite depend on one global test order.

For browser verification, `make reset-db` can restore the seeded state. The Playwright suite was also run more than once without resetting the database to check that the tests were not dependent on a single accidental database state.

## 6. Backend testing

The backend tests use the real PostgreSQL database rather than replacing persistence with an in-memory implementation.

The full backend suite can be run with:

```bash
docker compose run --rm --no-deps api cargo test
```

The backend tests cover the business rules that depend on:

- authentication and authorization;
- input validation;
- database persistence;
- evidence validation;
- review creation;
- review editing;
- evidence replacement;
- transaction rollback;
- concurrent saves;
- investor isolation.

The tests use the repository's existing database test setup.

For database-backed tests that create temporary users or reviews, temporary identifiers are used where necessary so tests can run without relying on a fixed user being available to every test.

This was important because Rust tests can run in parallel and several tests may otherwise try to modify the same seeded database records at the same time.

---

## 7. Backend verification

The final backend implementation was verified with:

```bash
docker compose run --rm --no-deps api cargo test
```

Result:

```text
37 passed
0 failed
```

Rust compilation was also checked with:

```bash
docker compose run --rm --no-deps api cargo check --locked
```

Result:

```text
passed
```

The database-backed tests include the transaction rollback and concurrent-save cases required by the assignment.

---

## 8. Frontend verification

The frontend was checked with the repository's existing Docker-based verification command:

```bash
make check-docker
```

This covers the repository's generated-output consistency checks, linting, TypeScript checking, and production build.

The final verification passed for:

- TypeScript type checking;
- linting;
- production build;
- generated TypeScript consistency.

No new frontend testing framework was introduced.

The existing Playwright setup provided by the assignment was used instead.

---

## 9. Playwright test structure

The browser tests are separated by feature rather than putting the complete assignment into one large test file.

```text
tests/
├── dataroom.spec.ts
├── review.spec.ts
└── support/
    └── auth.ts
```

### `dataroom.spec.ts`

Covers the main DataRoom workflows, including:

- company document registration;
- document search;
- document detail;
- investor restriction on document registration.

### `review.spec.ts`

Covers the main Review Plugin workflows, including:

- company restriction;
- investor review creation;
- evidence selection;
- review editing;
- `needs_information` status;
- investor-to-investor isolation;
- review input validation.

The authentication helper contains only shared login behavior. The feature tests remain independent from one another.

---

## 10. Browser test strategy

The browser tests use the real login flow and interact with the actual application UI.

They do not replace the business API with mocked review or DataRoom implementations.

This means the tests exercise the complete path:

```text
Browser
→ React UI
→ Plugin Host
→ API
→ PostgreSQL
```

This was important for this assignment because the evaluation includes the connection between the Plugin, Gen-TS, API, database, and UI.

The Playwright configuration runs the tests against both the configured desktop and mobile projects.

No `page.waitForTimeout()` calls are used to make the tests pass.

The tests rely on application state and accessible UI selectors instead.

---

## 11. Playwright results

The final browser suite contains six tests.

The configured Playwright projects run the same suite against desktop and mobile viewports.

Final result:

| Project | Result |
|---|---:|
| Desktop | 6/6 passed |
| Mobile | 6/6 passed |
| Total | 12/12 passed |

The browser suite was also run more than once during development, including a run after resetting the database and a consecutive run without resetting it.

This was useful for checking that the tests were not only passing because of a particular initial state.

---

## 12. Database isolation and test data

The assignment starts with fixed sample data in the `lighthouse` workspace.

The seeded data includes:

- one company user;
- two investor users;
- three review criteria;
- sample materials in `ready`, `processing`, and `failed` states.

For backend tests that need to create additional records, temporary users and identifiers are used where necessary.

This avoids relying on the state left behind by another test.

For Playwright, the seeded database provides the known starting point and `make reset-db` can be used to return the environment to that state.

The tests therefore distinguish between:

- real database-backed backend tests;
- real browser tests against the running application.

No business behavior is replaced with an in-memory storage implementation.

---

## 13. What was specifically tested

### Authorization

Verified that:

- company users can register materials;
- investors cannot register materials;
- investors can create/edit their own reviews;
- company users cannot access investor review functionality;
- one investor cannot access another investor's review.

### Input validation

Verified:

- empty/whitespace-only opinions are rejected;
- opinions over 2,000 characters are rejected;
- evidence is required;
- duplicate evidence IDs are rejected;
- invalid criteria are rejected.

### Evidence validation

Verified rejection of:

- missing materials;
- materials from another workspace;
- processing materials;
- failed materials;
- duplicate material IDs.

### Persistence

Verified that:

- registered materials are stored in PostgreSQL;
- reviews are stored in PostgreSQL;
- evidence relationships are stored in PostgreSQL;
- editing a review preserves its review ID;
- editing replaces the evidence set correctly.

### Transaction rollback

A database failure is intentionally introduced during a review save.

The test verifies that the original review and its original evidence remain unchanged after the failed transaction.

### Concurrency

Two saves for the same investor and criterion are executed concurrently.

The test verifies that:

- a duplicate review is not created;
- the final review is valid;
- evidence does not become a mixture of both concurrent requests.

### Browser user flows

Playwright verifies the main end-to-end workflows for both desktop and mobile viewports.

---

## 14. Verification commands

For a full verification pass, the main commands are:

```bash
make reset-db
make dev
```

Then, in the repository environment:

```bash
make check-docker
```

and:

```bash
make test-e2e
```

For the backend:

```bash
docker compose run --rm --no-deps api cargo test
docker compose run --rm --no-deps api cargo check --locked
```

The database can be returned to the original seeded state at any time with:

```bash
make reset-db
```

---

## 15. Environment/setup issues

Most of the implementation work was performed inside the provided Docker Compose environment.

There were a few environment-related issues during development, including:

- container bind-mount permission problems on the development machine;
- pnpm/node_modules volume behavior;
- permissions when generating TypeScript output from the container.

These were handled as environment/setup issues rather than changes to the application architecture.

For example, the container bind-mount issue was related to the host's SELinux labeling. The TypeScript generation issue was resolved by making only the generated output mount writable while keeping the relevant runtime mounts read-only.

These changes were kept separate from the business functionality.

The environment fixes and their reasoning are recorded in `approach.md`.

---

## 16. Areas not covered by tests

The following were intentionally not tested because they are outside the required scope:

- external OAuth;
- user registration;
- MFA;
- account recovery;
- workspace creation;
- document deletion;
- review criteria management;
- actual file storage;
- PDF/OCR processing;
- real-time collaboration;
- cloud deployment;
- the optional AI review-draft feature.

The optional AI review-draft feature was not implemented, so there are no tests for it.

---

## 16.1 What these tests do not prove

The passing tests give confidence in the implemented assignment workflow, but they do not prove every possible production scenario.

In particular:

- there is no load/performance test;
- there is no cloud/deployment test;
- there is no test for the optional AI review-draft feature because it was not implemented;
- external authentication and account-management flows are outside the assignment;
- browser tests cover the configured Desktop Chrome and Pixel 7 projects, not every possible browser/device combination.

The backend tests are database-backed and the Playwright tests exercise the real application, but the test suite is still scoped to the assignment requirements rather than production-scale reliability testing.

## 17. Final verification summary

| Area | Result |
|---|---|
| Backend tests | 37/37 passed |
| Playwright desktop | 6/6 passed |
| Playwright mobile | 6/6 passed |
| Playwright total | 12/12 passed |
| Rust `cargo check --locked` | Passed |
| TypeScript typecheck | Passed |
| Lint | Passed |
| Production build | Passed |
| Generated TypeScript consistency | Passed |

The final verification uses real PostgreSQL-backed backend tests and real browser-level Playwright tests rather than relying only on mocks.
