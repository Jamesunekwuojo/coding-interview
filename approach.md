# Approach

## 1. Purpose of This Document

This document explains how I approached the DataRoom coding assignment before starting the main implementation.

The goal is not only to describe what I plan to build, but also to document how I understood the existing codebase, how I investigated the development environment, the problems I encountered, the decisions I made, and the reasoning behind those decisions.

I am treating the assignment as an existing product that needs to be completed rather than as a greenfield application. Because of that, my first step was to understand the architecture and the boundaries already established by the repository before introducing new code.

---

# 2. My Understanding of the Assignment

This repository provides a partially implemented DataRoom application.

The main workflow I need to complete is:

1. Materials are registered in the DataRoom.
2. Materials can be viewed and searched.
3. Reviews are created for investors against predefined review criteria.
4. Evidence materials are attached to reviews.
5. Reviews can be edited without creating a new review record.
6. Review progress is calculated from the completed review criteria.
7. Access is restricted according to the authenticated user, role, and workspace.

The important boundary in the application is the workspace.

For this assignment, the existing `lighthouse` workspace represents the DataRoom boundary. Users must not be able to access or modify resources belonging to another workspace.

The repository already provides much of the infrastructure needed for this work, including:

- authentication and sessions
- the React host application
- the plugin system
- the Review Plugin structure
- the API communication layer
- generated TypeScript API types
- the Axum API
- PostgreSQL and SQLx
- the existing database migration structure
- shared UI components

The missing work is therefore mainly the actual DataRoom business functionality and the Review Plugin functionality.

---

# 3. Initial Repository Reconnaissance

Before writing business logic, I inspected the repository structure and the existing implementation.

The main areas I looked at were:

- `api/`
- `web/`
- `plugins/`
- `plugin-sdk/`
- `api-client/`
- `ui-kit/`
- `scripts/`
- database migrations
- authentication/session handling
- RPC request and response structures
- plugin loading and routing
- generated TypeScript API code

This was important because I did not want to introduce another API pattern, another authentication mechanism, or another way of communicating between the host application and plugins when the repository already has established conventions.

The existing architecture can be summarized as:

```text
Browser
   |
   v
React Host Application
   |
   +--------------------+
   |                    |
   v                    v
DataRoom UI        Review Plugin UI
   |                    |
   v                    v
DataRoom RPC        Plugin RPC
   \                    /
    \                  /
     v                v
          Axum API
             |
             v
        PostgreSQL
```

The plugin system and generated API client are already part of the application architecture, so the implementation should extend those systems rather than bypass them.

---

# 4. Development Environment Investigation

Getting the project running was part of the initial investigation.

The first `make dev` attempt did not start successfully. The API container reported:

```text
error: could not find Cargo.toml in /app/api
```

At first this looked like a Docker mount or repository-path problem because the `Cargo.toml` file was present on the host.

I checked the Docker Compose configuration and confirmed that the repository's `api` directory was supposed to be mounted into `/app/api`.

The next check showed that the container could see the directory but could not access it properly.

The host machine is running Fedora with SELinux enforcing. The repository files had the host SELinux context:

```text
user_home_t
```

while the container needed a context that allowed the container to access the bind-mounted files.

I therefore applied the appropriate container SELinux label:

```bash
sudo chcon -Rt container_file_t .
```

After this change, the API container could access `Cargo.toml` and the Rust application started successfully.

The API eventually reached:

```text
Dataroom API: http://0.0.0.0:4318
```

This was an important finding because the original problem was not a missing Rust file or an incorrect Cargo configuration. It was an environment/SELinux permission issue affecting the Docker bind mount.

---

# 5. Web Container Investigation

After fixing the API container, the web container exposed another environment issue.

The web container failed during dependency installation with an error involving:

```text
/app/ui-kit/node_modules
```

The repository uses a pnpm workspace containing the main web application and the shared `ui-kit` package.

The important discovery was that the `ui-kit` directory was mounted read-only into the container.

I confirmed this directly by attempting to write into the directory from the container. The filesystem returned:

```text
Read-only file system
```

This explained why pnpm could not create the workspace's `node_modules` directory.

I initially considered adding another Docker volume for the `node_modules` directory, but Docker could not create that mount point underneath the read-only parent mount.

I then tested pnpm's hoisted node linker without changing the project's dependency declarations:

```bash
pnpm install --node-linker=hoisted
```

The installation completed successfully.

Because this solved the workspace installation problem without changing the application's dependency graph, I made the corresponding change to `scripts/web-container.sh` by adding:

```text
--node-linker=hoisted
```

to the existing pnpm install command.

This allowed the complete development environment to start successfully.

---

# 6. What I Learned From the Environment Issues

These problems changed my understanding of the repository in a useful way.

The first failure looked like an application problem, but it was actually caused by the host/container security boundary.

The second failure looked like a pnpm dependency problem, but it was actually caused by the combination of:

- pnpm workspace packages
- Docker bind mounts
- read-only package mounts
- where pnpm expected to create workspace dependencies

I am documenting these issues because they are part of the actual implementation experience and because the assignment specifically asks for the development/environment response time to be distinguished from the actual implementation time.

The initial environment troubleshooting took significantly longer than expected. The first `make dev` attempt ran for roughly 10 minutes before timing out.

I will record the final implementation time separately once the business functionality is complete.

---

# 7. Current Development Environment

The development environment is now operational.

The web application is available through the exposed web port:

```text
http://localhost:5178
```

The API listens on port `4318` inside the Docker Compose network.

The API port is not currently published directly to the host.

This means a request such as:

```bash
curl http://localhost:4318/api/health
```

from the host machine is expected to fail because port `4318` is not exposed to the host.

The web container can communicate with the API using the internal Docker address:

```text
http://api:4318
```

This is consistent with the existing Compose architecture, so I do not currently plan to expose the API port to the host unless development or testing later proves that it is necessary.

The application can now be started and accessed through the normal development flow.

---

# 8. What Already Exists

The repository already contains several important pieces that I should not rebuild.

### Authentication and Sessions

The application already has authentication/session handling.

The business implementation should use the authenticated identity supplied by the server rather than accepting identity information from the browser.

### Plugin Architecture

The repository already has a plugin loader and plugin host.

The Review Plugin should therefore remain a plugin instead of being turned into a separate application.

### RPC Communication

The API already uses RPC-style request dispatching.

The new DataRoom and Review functionality should follow the existing RPC conventions rather than introducing REST endpoints unnecessarily.

### Generated TypeScript API

The repository already has a Rust-to-TypeScript generation process.

The Rust API types should remain the source of truth, and the TypeScript client should be generated from them.

I should not manually maintain duplicate API types in the frontend.

### Database

The project already uses PostgreSQL with SQLx migrations.

The required business tables should therefore be added through the existing migration mechanism.

---

# 9. Business Model I Intend to Implement

After studying the assignment requirements and the existing architecture, I expect the required domain model to consist of three main tables.

## Materials

Materials belong to a workspace and represent the evidence that can later be attached to reviews.

Conceptually:

```text
materials
-----------
id
workspace_id
uploader_id
title
file_name
content
status
created_at
updated_at
```

The material status is expected to support:

```text
ready
processing
failed
```

The assignment requires evidence attached to reviews to be `ready`.

There is currently no separate asynchronous processing pipeline in the skeleton that would justify allowing the browser to control this state.

Therefore, when a material is registered through the current implementation, the server should determine its initial status rather than trusting a status supplied by the client.

For the current synchronous implementation, a newly registered material can start as `ready`.

The database should still support the other statuses because the assignment explicitly defines them and because they are relevant to evidence validation.

---

# 10. Reviews

A review represents an investor's assessment against one review criterion.

Conceptually:

```text
reviews
--------
id
workspace_id
criterion_id
user_id
status
opinion
created_at
updated_at
```

The review status is:

```text
satisfied
needs_information
```

The same investor should not have multiple reviews for the same criterion.

Therefore, the database should enforce uniqueness around:

```text
workspace_id + criterion_id + user_id
```

When a review is edited, the existing review record should be updated rather than creating another review.

---

# 11. Review Evidence

Evidence represents the relationship between a review and the materials supporting that review.

Conceptually:

```text
review_evidence
---------------
review_id
material_id
```

The pair:

```text
review_id + material_id
```

should be unique.

This prevents the same material from being attached to the same review more than once.

The database foreign keys can guarantee that the referenced records exist, but they cannot by themselves guarantee that the review and material belong to the same workspace.

That workspace relationship therefore needs to be validated in the application/service layer before the transaction is committed.

---

# 12. API Design

I want the API surface to remain small and focused on what the UI actually needs.

For the DataRoom, the expected operations are along the lines of:

```text
list_materials
get_material
register_material
```

The material registration request should contain the actual material information, such as:

```text
title
fileName
content
```

It should not contain:

```text
userId
workspaceId
status
```

Those values are either derived from the authenticated request or determined by the server.

For reviews, the expected operations are:

```text
list_criteria
get_summary
list_reviews
get_review
save_review
```

The review save operation should receive the review information and evidence material IDs, for example:

```text
criterionId
status
opinion
evidenceMaterialIds
```

The authenticated user and workspace should be derived on the server.

The exact final RPC names and request/response shapes will be confirmed against the repository's existing conventions before implementation.

---

# 13. Authorization Approach

Security is one of the areas I want to get right before focusing on the UI.

The basic request flow should be:

```text
Authentication
      ↓
Workspace authorization
      ↓
Role authorization
      ↓
Resource authorization
      ↓
Business validation
      ↓
Database operation
```

The server should never trust the browser to tell it:

- which user is making the request
- which workspace the request belongs to
- which role the user has

Those values must come from the authenticated session/server-side context.

The workspace is the main tenant boundary.

For example, when attaching evidence to a review, it is not enough to check that the material ID exists.

The server must also confirm that the material:

1. belongs to the current workspace,
2. is in the required `ready` state,
3. is not duplicated in the evidence list.

Unauthorized access should be rejected before performing unnecessary domain-level validation.

---

# 14. Material Validation

Material registration will follow the assignment's requirements.

The important validation rules include:

- title is required
- filename is required
- only the required `.txt` and `.md` file types are accepted
- content is UTF-8 text
- the request must remain within the assignment's request-size limit
- the material belongs to the authenticated workspace
- the uploader is derived from the authenticated user

I do not want to introduce an arbitrary smaller content limit simply because it is convenient to implement.

The assignment specifies a request limit of `256 KiB`, so I will avoid inventing a different business limit unless the existing repository conventions require one.

I will also keep the distinction between the HTTP/request-size limit and PostgreSQL text storage clear. They are not the same thing.

---

# 15. Material Search

The material list needs to support searching.

The initial implementation can use a parameterized PostgreSQL query with `ILIKE`.

For example, the search can match against fields such as the material title and filename.

I will not claim that a normal B-tree index automatically makes a query such as:

```sql
ILIKE '%term%'
```

efficient.

If the assignment remains small, the straightforward query is sufficient.

If the project later grows enough to require optimized substring search, PostgreSQL features such as `pg_trgm` could be considered separately.

For this assignment, I prefer the simpler implementation unless the existing requirements justify additional complexity.

---

# 16. Review Validation

Saving a review should be treated as one business operation.

The validation sequence I intend to follow is approximately:

```text
1. Authenticate the request
2. Determine the current workspace
3. Confirm the required role
4. Validate the criterion
5. Validate the review status
6. Validate the opinion
7. Confirm evidence is provided
8. Check for duplicate evidence IDs
9. Load the evidence materials
10. Confirm all materials belong to the workspace
11. Confirm all materials are ready
12. Start a database transaction
13. Create or update the review
14. Replace the review's evidence relationships
15. Commit the transaction
```

The important part is that the review update and evidence replacement must happen in the same transaction.

If anything fails, the existing review and its existing evidence should remain unchanged.

This prevents a situation where the review is updated successfully but the evidence update fails halfway through.

---

# 17. Progress Calculation

The Review Plugin needs to show progress for the investor.

The assignment defines a review as completed when its status has been submitted, including:

```text
satisfied
needs_information
```

Therefore:

```text
completed = number of criteria with a saved review
remaining = total criteria - completed
```

Progress should be calculated for the authenticated investor rather than accepting an arbitrary user ID from the client.

The exact UI representation will follow the existing Review Plugin structure.

---

# 18. Architecture Decisions

At this point, my intended architecture is:

```text
                    Browser
                       |
                       v
              React Host Application
                       |
              +--------+--------+
              |                 |
              v                 v
          DataRoom         Review Plugin
              |                 |
              v                 v
        DataRoom RPC       Plugin RPC
              \                 /
               \               /
                    Axum API
                       |
                       v
                  PostgreSQL
```

The responsibilities are intentionally separated.

### DataRoom

Owns materials and material-related operations.

### Review Plugin

Owns:

- criteria
- reviews
- review evidence
- review progress

### API

Owns:

- authentication context
- authorization
- validation
- business rules
- database operations

### PostgreSQL

Owns:

- persistence
- foreign-key relationships
- uniqueness constraints
- appropriate database-level integrity rules

### Frontend

Owns:

- user interaction
- form state
- display state
- query caching
- presenting server responses

The frontend should not become the source of truth for business rules.

---

# 19. Generated TypeScript

The generated API types are an important part of the existing architecture.

The intended flow is:

```text
Rust API types
      ↓
ts-rs / generation
      ↓
generated TypeScript
      ↓
React application
```

I will therefore avoid manually duplicating Rust request/response types in TypeScript.

After changing the Rust API contracts, I will regenerate the TypeScript API client/types and make the frontend consume the generated definitions.

This should reduce the chance of the backend and frontend silently disagreeing about request or response shapes.

---

# 20. React Query

The frontend already uses React Query.

The new queries and mutations should follow that pattern instead of introducing another client-side state management mechanism.

Query keys should contain the relevant identity/tenant information where appropriate.

For example, a material query should not accidentally reuse cached data from another authenticated context.

After mutations, the relevant queries should be invalidated or updated so that the UI reflects the new server state.

---

# 21. What I Am Deliberately Not Adding

I want to keep the implementation proportional to the assignment.

At this stage I do not plan to introduce:

- REST endpoints alongside the existing RPC architecture
- GraphQL
- Redis
- S3/object storage
- background workers
- a search engine
- WebSockets
- CQRS
- event sourcing
- a separate service for reviews
- an AI review system

The assignment can be completed using the architecture already provided by the repository.

Adding infrastructure that does not solve a stated requirement would increase complexity and make the implementation harder to explain.

---

# 22. AI-Assisted Investigation

I used AI during the reconnaissance phase, but I did not treat the generated architecture proposal as automatically correct.

One of the AI-assisted tasks was an architecture audit of the existing repository.

The AI was asked to inspect the existing architecture and identify:

- what is already implemented
- what is missing
- possible database structures
- API boundaries
- plugin responsibilities
- security concerns
- testing requirements
- implementation risks

The resulting audit was then compared against my own inspection of the repository.

This comparison was important because some suggestions were useful while others needed modification.

For example:

### Accepted

The audit correctly identified the importance of:

- keeping the Review functionality inside the plugin architecture
- using generated TypeScript types
- enforcing workspace boundaries
- using database constraints for uniqueness
- using a transaction for review/evidence updates
- testing cross-workspace access
- testing duplicate evidence
- testing invalid material states

### Modified

The AI suggested an additional `200 KiB` content restriction.

I rejected that as an unnecessary invented constraint because the assignment specifies a request limit of `256 KiB`.

I also modified the suggestion around material status.

The client should not be allowed to choose whether a material is `ready`, `processing`, or `failed`. The server should determine that state.

The AI audit also suggested indexing that could be interpreted as making `%term%` searches efficient. I rejected that assumption because a normal B-tree index does not automatically solve substring searches using:

```sql
ILIKE '%term%'
```

These changes are important because they demonstrate that AI was used as an engineering assistant, not as a replacement for understanding the code or requirements.

---

# 23. Testing Strategy

The repository does not provide a complete business-level test suite for the functionality being implemented.

I therefore intend to build the tests alongside the implementation rather than leaving them until the end.

The backend tests should cover at least:

### Authentication and authorization

- unauthenticated requests
- wrong workspace
- insufficient role
- resource ownership/boundary checks

### Materials

- successful registration
- invalid title
- invalid file type
- invalid content
- listing
- searching
- retrieving a material
- workspace isolation

### Reviews

- creating a review
- updating an existing review
- duplicate review prevention
- invalid status
- empty/whitespace-only opinion
- opinion length validation

### Evidence

- valid evidence
- no evidence
- duplicate evidence IDs
- nonexistent material
- material from another workspace
- processing material
- failed material
- mixed valid/invalid evidence

### Transactions

The important failure case is:

```text
review update succeeds
        +
evidence update fails
        ↓
everything should roll back
```

The previous review/evidence state should remain intact.

### Progress

- no completed reviews
- partially completed reviews
- all criteria completed
- `needs_information` counted as completed

The frontend should also have Playwright coverage for the important user flows.

---

# 24. Implementation Order

I do not want to start by writing a large amount of code at once.

The implementation will proceed in small checkpoints.

## Step 1 — Confirm Existing Conventions

Before creating the migration, I will inspect the existing code for:

- ID types
- timestamp conventions
- database error handling
- migration naming
- workspace/user relationships
- existing SQLx patterns
- RPC naming conventions
- response/error conventions

This prevents the new code from looking foreign to the rest of the repository.

## Step 2 — Database Migration

Create the required tables, relationships, constraints, and indexes.

Then run the migration and verify the resulting schema.

## Step 3 — DataRoom API

Implement material registration, retrieval, listing, and search.

Then generate the corresponding TypeScript API definitions.

## Step 4 — Review API

Implement criteria, review creation/update, evidence handling, and progress.

The review save operation will use a transaction.

## Step 5 — DataRoom UI

Connect the existing DataRoom shell to the material API.

Implement:

- material registration
- material listing
- searching
- material details

## Step 6 — Review Plugin UI

Connect the Review Plugin to the backend.

Implement:

- criteria
- review form
- evidence selection
- review editing
- progress

## Step 7 — Tests

Add backend tests and Playwright tests around the important business flows and security boundaries.

## Step 8 — Security Review

Review every new endpoint/RPC path for:

- authentication
- workspace authorization
- role authorization
- resource ownership
- client-controlled identity
- client-controlled status
- cross-workspace evidence

## Step 9 — Documentation

Complete the required documentation, including:

- setup/login/reset instructions
- completed and incomplete scope
- implementation/environment timing
- design decisions
- changed understanding
- testing commands and results
- limitations
- AI usage examples and validation evidence

---

# 25. Current Status

At the point this document is being written:

### Completed

- Repository structure inspected
- Existing architecture inspected
- Authentication/session flow inspected
- Plugin architecture inspected
- RPC architecture inspected
- Generated TypeScript flow inspected
- Database/migration structure inspected
- Docker Compose environment investigated
- SELinux bind-mount problem identified and resolved
- Web workspace/pnpm problem identified and resolved
- Development environment brought up successfully
- Initial business/domain model defined
- Initial API boundaries defined
- Security model defined
- AI architecture audit performed
- AI recommendations independently reviewed
- Architecture decisions recorded

### Not Yet Implemented

- Materials database migration
- Materials API
- Materials UI
- Review database migration
- Review API
- Review evidence transaction
- Review UI
- Progress UI
- Automated business tests
- Playwright tests
- Final documentation

This separation is intentional.

The repository is now in a state where the implementation can begin from an informed design rather than from trial and error.

---

# 26. Timing

I will keep two different timing records because they measure different things.

### Environment / setup response time

The initial environment troubleshooting took approximately:

```text
~30 minutes 21 seconds
```

for the first `make dev` attempt before it timed out, followed by additional investigation and fixes for the SELinux and pnpm workspace issues.

### Actual implementation time

```text
Pending
```

I will record this after the required business functionality has been implemented.

The purpose of keeping these separate is to avoid presenting environment/debugging time as if it were implementation time.

---

# 27. Current Working Principle

The main principle guiding the implementation is:

> Understand the existing system first, make the smallest change that satisfies the requirement, enforce important rules on the server and database, and verify each major step before moving to the next one.

I also want every significant technical decision to be explainable without saying:

> "The AI suggested it."

AI can help me investigate, compare approaches, identify risks, and move faster.

But the final implementation should be based on my understanding of the repository, the assignment requirements, and evidence from the code and tests.

---

# 28. Next Step

Before writing the first migration, I will inspect the existing ID, error, timestamp, workspace, and database conventions in the relevant Rust and SQL code.

The purpose is to make the new schema and API feel like a natural extension of the existing project rather than a separate design placed on top of it.

After those conventions are confirmed, I can write the migration and proceed incrementally from the database layer upward.


### Database Implementation Checkpoint

The database design was implemented in migration `0003_dataroom_and_reviews.sql`.

The migration introduces three tables:

- `materials` — stores DataRoom materials belonging to a workspace.
- `reviews` — stores an investor's review for a fixed review criterion.
- `review_evidence` — connects reviews to the materials used as evidence.

The schema uses foreign keys, check constraints, unique constraints, and indexes to enforce important domain rules at the database level. In particular, a review is unique per workspace, investor, and criterion, while duplicate evidence materials cannot be associated with the same review.

The migration also seeds the four materials from the provided scenario:

- `company-overview.md` — `ready`
- `team.md` — `ready`
- `revenue.txt` — `failed`
- `customer-interviews.md` — `processing`

The separate `samples/revenue-update.md` file was intentionally not seeded because it represents a new material that can later be registered through the DataRoom flow.

During development, migration `0003` was modified after it had already been applied to the development database. The migration system detected the changed migration checksum and stopped the API from starting. Since the database contained disposable development data, the development database was reset and the migration was reapplied. Going forward, applied migrations will be treated as immutable, and schema changes will be introduced through new migration files.

The database was then verified manually. The three new tables exist, the expected constraints and indexes are present, the four scenario materials are present with the expected statuses, and the review/evidence tables are initially empty as expected.


### DataRoom API Contract and DTO Implementation

#### API contract

Before implementing the DataRoom dispatcher, I finalized the RPC contract for the material workflow.

The DataRoom exposes three methods:

- `list_materials` — lists materials in the authenticated user's workspace, optionally filtered by title.
- `get_material` — retrieves one material, including its content.
- `register_material` — registers a new `.txt` or `.md` material.

The server derives `workspace_id` and `uploader_id` from the authenticated session. The client does not provide these values. The material status is also server-controlled and is initially set to `ready`.

The API uses typed request and response DTOs rather than passing arbitrary JSON structures through the domain layer.

#### Rust DTOs and TypeScript generation

Implemented the DataRoom DTOs in:

`api/src/dataroom/types.rs`

The DTOs include:

- `MaterialStatus`
- `MaterialSummary`
- `MaterialDetail`
- `ListMaterialsParams`
- `ListMaterialsResponse`
- `GetMaterialParams`
- `GetMaterialResponse`
- `RegisterMaterialParams`
- `RegisterMaterialResponse`

The existing `serde` and `ts-rs` conventions were preserved. Rust fields use snake_case while the generated API contract uses camelCase for TypeScript consumers.

Generated TypeScript files are produced by the existing Gen-TS pipeline and are not edited manually.

#### Docker TypeScript generation fix

While running `make gen-ts-docker`, TypeScript generation initially failed with an `EROFS` (Read-only filesystem) error.

The problem was caused by the generation output path conflicting with an existing read-only Docker volume mount.

I updated `scripts/gen-ts-docker.sh` so the generation output is explicitly mounted as:

`/out-client:rw,z`

This gives the code-generation process a writable output location while preserving the runtime container's existing read-only volume configuration.

After the change, `make gen-ts-docker` completed successfully and generated the expected TypeScript contracts.

#### Verification

The Gen-TS generation step was executed successfully.




### DataRoom Backend: Dispatcher and List Materials

#### DataRoom RPC architecture

The DataRoom HTTP handler remains intentionally thin. `api/src/handlers/dataroom_rpc.rs` is responsible for extracting the authenticated user, database pool, and `DataroomRpcRequest`, then forwarding the request to `dataroom::dispatch`.

The DataRoom dispatcher in `api/src/dataroom/mod.rs` is responsible for the DataRoom-specific authorization and routing:

1. Verify that the requested `workspace_id` matches the authenticated user's workspace.
2. Verify role requirements for operations that require specific permissions.
3. Match the RPC method to the appropriate DataRoom operation.
4. Deserialize operation-specific parameters.
5. Execute the domain/database operation.

I kept this separation instead of moving the dispatcher logic into the HTTP handler because it follows the architecture already established by the skeleton and keeps transport concerns separate from DataRoom business logic.

#### Workspace authorization

Workspace isolation is enforced before DataRoom operations are dispatched.

The authenticated user's workspace is treated as the authoritative workspace boundary. The client-provided `workspace_id` is not trusted as proof of access.

If the requested workspace does not match the authenticated user's workspace, the dispatcher returns `403 Forbidden` before executing the requested operation.

For `register_material`, the user's role is also checked before processing the request. Only users with the `Company` role can register materials.

This establishes the authorization boundary before domain-specific processing.

#### List materials

Implemented the `list_materials` DataRoom RPC operation.

The operation:

- Lists materials belonging only to the authenticated user's workspace.
- Supports an optional title search.
- Uses parameterized SQL for search values.
- Sorts results by `created_at DESC, id ASC`.
- Returns summary information rather than material content.
- Returns material status as the typed `MaterialStatus` enum.
- Ignores an empty or whitespace-only search value and treats it as no search filter.

The database query explicitly scopes results using the authenticated user's workspace rather than trusting a workspace identifier from the request.

The API response uses the existing DataRoom DTO boundary:

- `MaterialSummary`
- `ListMaterialsParams`
- `ListMaterialsResponse`

These DTOs remain in `api/src/dataroom/types.rs` because they represent the API contract between Rust and TypeScript. `models.rs` is reserved for database/domain models if they become necessary; no additional model abstraction was introduced for this operation because the current query is small and does not benefit from it.

#### Timestamp representation

The DataRoom DTOs currently represent timestamps as `String`. The existing API dependency configuration does not enable SQLx's `time` feature, so the list query converts `TIMESTAMPTZ` to text using PostgreSQL rather than introducing another dependency solely for this operation.

#### Verification

After implementing `list_materials`, the following checks were run successfully:

```bash
docker compose exec -w /app/api api cargo check --locked
```

and:

```bash
docker compose exec -w /app/api api cargo test dataroom::tests
```

The existing DataRoom authorization tests continued to pass, including:

- rejection of cross-workspace requests;
- rejection of investor attempts to register materials before input validation.

#### Current implementation status

Completed:

- DataRoom RPC dispatcher
- Workspace authorization
- Company-only authorization for material registration
- `list_materials` RPC
- Material summary response
- Focused DataRoom backend tests

Next:

- Implement `get_material`
- Add material lookup and not-found behavior
- Verify cross-workspace material access is not exposed


### DataRoom Backend: Material Operations Complete

#### Get material

Implemented the `get_material` DataRoom RPC operation.

The operation:

- Deserializes the requested material ID using the typed `GetMaterialParams` DTO.
- Retrieves the material using both the material ID and the authenticated user's workspace ID.
- Returns the full `MaterialDetail`, including content and timestamps.
- Returns `404 Not Found` when the material does not exist or does not belong to the authenticated user's workspace.
- Maps the database material status to the typed `MaterialStatus` enum.

Workspace scoping is enforced directly in the database query:

```sql
WHERE id = $1
  AND workspace_id = $2
```

This prevents a user from retrieving material content from another workspace and avoids exposing whether a material ID exists outside their accessible workspace.

#### Register material

Implemented the `register_material` DataRoom RPC operation.

The client supplies only:

- title
- file name
- content

The server derives the security-sensitive fields:

- `workspace_id` from the authenticated user's workspace;
- `uploader_id` from the authenticated user's ID;
- `status` as `ready`;
- - material ID using the same secure random ID-generation mechanism already used by the authentication system, with a `mat_` prefix.

Validation is performed before database insertion:

- title must not be empty or whitespace-only;
- file name must end in `.txt` or `.md`, case-insensitively.

I did not add a separate content-size check inside the domain operation because the requirement concerns the entire HTTP request size rather than only the content string. The existing application request-size handling remains responsible for that boundary.

#### Authorization boundary

The DataRoom dispatcher remains responsible for authorization before invoking DataRoom operations.

For `register_material`, the dispatcher verifies that the authenticated user has the `Company` role before calling the material registration operation.

This means an unauthorized investor request is rejected before material validation or database access.

The distinction is intentional:

- `dataroom::dispatch` owns RPC routing and authorization.
- `register_material` owns material-specific validation and persistence.

#### Testing and debugging

The DataRoom test suite now contains eight focused tests covering:

- cross-workspace RPC rejection;
- investor registration rejection;
- successful material retrieval;
- missing material handling;
- cross-workspace material retrieval;
- successful material registration;
- empty title validation;
- unsupported file type validation.

All eight tests pass.

During the registration test implementation, the initial test assumptions exposed several issues:

1. The repository's validation error code is `invalid_input`, not `invalid`.
2. The investor authorization test needed to exercise the dispatcher rather than calling `register_material` directly because authorization belongs to the dispatcher layer.
3. The `materials.uploader_id` foreign key requires the test user to exist in the `users` table, so the database-backed registration test uses the existing seeded `company-user`.
4. The successful registration test verifies that the server stores the authenticated user's workspace and user ID rather than accepting those values from the client.

These failures were used to align the tests with the existing architecture and database constraints rather than changing the implementation to satisfy incorrect test assumptions.

#### Verification

The following checks are passing:

```text
docker compose exec -w /app/api api cargo check --locked
docker compose exec -w /app/api api cargo test dataroom::tests
```

The focused DataRoom test suite currently passes:

```text
8 passed; 0 failed
```

#### Current implementation status

Completed:

- DataRoom RPC dispatcher
- Workspace authorization
- Company-only material registration authorization
- `list_materials`
- `get_material`
- `register_material`
- Material validation
- Material persistence
- Workspace isolation
- Focused DataRoom backend tests

Next:

- Review Plugin backend/API implementation
- Define and implement review RPC operations
- Preserve the existing Plugin/Gen-TS architecture



### AI-Assisted Review Plugin Contract Review

Before implementing the Review Plugin backend, I used an AI coding assistant to critically review the proposed RPC/API contract against the assignment requirements.

The AI was given the Review Plugin requirements, the proposed RPC methods (`list_criteria`, `get_summary`, `list_reviews`, `get_review`, and `save_review`), the proposed save payload, and the database uniqueness constraint.

The AI identified several important concerns, including deterministic criterion ordering, preserving review IDs during edits, workspace-scoped evidence validation, authorization before input validation, transactional review/evidence updates, strong review status typing, and the need for database-level uniqueness.

I adopted these recommendations where they matched the repository requirements. In particular:
- criteria will be ordered by `display_order`;
- review edits will preserve the existing review ID;
- evidence will be validated against the authenticated workspace and `ready` status;
- duplicate evidence IDs will be rejected rather than silently deduplicated;
- unauthorized write requests will be rejected before domain/input validation;
- review status will use a typed Rust enum and generated TypeScript union;
- review and evidence changes will be performed in one database transaction;
- the existing `(workspace_id, criterion_id, user_id)` uniqueness constraint will protect the one-review-per-investor-per-criterion invariant.

I did not adopt every AI recommendation. For example, the assignment does not explicitly require `get_summary` for company users to return zero progress, so I will not invent that behavior. I also treated PostgreSQL `ON CONFLICT DO UPDATE` as an implementation alternative rather than a requirement.

The recommendations were validated against the actual README, migration schema, existing Review Plugin skeleton, and the previously implemented DataRoom architecture.


### Review Plugin DTOs and Gen-TS Contract

After reviewing the Review Plugin requirements and existing skeleton, I defined the initial API DTOs for the required review operations.

The Review Plugin uses a typed `ReviewStatus` enum with the two allowed values:
- `satisfied`
- `needs_information`

The main DTOs cover:
- fixed review criteria;
- investor review progress;
- review list items;
- review detail and evidence;
- review save input;
- review save response.

The save request intentionally contains only client-controlled review data:
- criterion ID;
- review status;
- opinion;
- evidence material IDs.

The authenticated user and workspace are not accepted from the client and will be derived from the authenticated session on the server.

The DTOs use `serde(rename_all = "camelCase")` so the generated TypeScript matches the existing frontend conventions.

After implementation, `make gen-ts-docker` successfully generated the corresponding TypeScript types under `api-client/src/types/`.

I verified generated types including:
- `ReviewStatus` → `"satisfied" | "needs_information"`
- `SaveReviewParams` → `criterionId`, `status`, `opinion`, `evidenceMaterialIds`
- `ReviewCriterion` → `reviewQuestion`, `displayOrder`
- `ReviewDetail` → `criterionId`, `criterionTitle`, `reviewQuestion`, `createdAt`, `updatedAt`, and nested evidence.

No generated TypeScript files were edited manually.


### Review Plugin Dispatcher and Criteria

The Review Plugin server follows the same thin-handler/domain-dispatch boundary used by the DataRoom implementation.

The Review Plugin RPC dispatcher:
- verifies that the requested workspace matches the authenticated user's workspace;
- routes requests by the Plugin RPC method;
- keeps authorization decisions at the dispatcher boundary;
- delegates the actual operation to the corresponding Review Plugin server function.

The first implemented business operation is `list_criteria`.

The criteria are fixed in the `review_criteria` table by the initial migration. The API does not provide criterion creation or modification.

`list_criteria` reads the fixed criteria and explicitly orders them by `display_order ASC`. The resulting order is:
1. `business`
2. `team`
3. `revenue`

A real PostgreSQL-backed test was added using the repository's existing `DATABASE_URL` test convention. The test verifies both the returned criterion IDs and their display order.

Verification:
- `cargo test list_criteria`
- Result: 1 passed, 0 failed.


### Review Plugin Progress Summary

The Review Plugin `get_summary` operation is restricted to authenticated investors and calculates progress for the current user within the authenticated workspace.

The summary is based on the fixed review criteria rather than the number of existing reviews. This means a new investor starts with all three criteria remaining.

The summary reports:
- `completed` — criteria with an existing review;
- `remaining` — criteria without a review;
- `satisfied` — completed reviews marked as satisfied;
- `needs_information` — completed reviews marked as needing more information.

A `needs_information` review still counts as completed because the assignment defines progress based on whether the criterion has been reviewed, not whether the investor is satisfied with the available information.

The query is scoped by both `workspace_id` and the authenticated `user_id`, so one investor cannot affect another investor's progress.

Tests were added for:
- a new investor with no reviews;
- `needs_information` counting as completed;
- isolation from another investor's reviews.

Verification:
- `cargo test get_summary`
- Result: 3 passed, 0 failed.
- `cargo check --locked`
- Result: passed.


