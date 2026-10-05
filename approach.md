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