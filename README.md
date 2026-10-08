# Dataroom

**기업**은 투자 검토에 필요한 자료를 제공하고, **투자자**는 그 자료를 근거로 의견을 남깁니다.
제공된 스켈레톤 코드를 바탕으로 **자료 등록 → 근거 연결 → 검토 작성·수정 → 현황 확인** 흐름을 구현해 주세요.

`samples/`에 담긴 시나리오를 바탕으로 데이터룸 한 개와 자료를 재현해 주세요. 기업 담당자 한 명, 서로 다른 투자자 두 명, 검토 기준 세 개는 DB 초기 마이그레이션에 포함되어 있습니다.

이 과제에서는 **workspace 하나가 데이터룸 하나의 접근 경계입니다.** 구현 대상은 제공된 `lighthouse` workspace이며, API와 저장 데이터의 `workspaceId`가 데이터룸의 범위를 결정합니다. 이 문서에서 “다른 데이터룸”은 `workspaceId`가 다른 데이터룸을 뜻합니다.

## 제공 범위와 구현 범위

### 기술 스택

- FE: React·TypeScript·Vite·React Query·Tailwind·Playwright
- BE: Rust Axum·SQLx
- DB: PostgreSQL

로그인·세션, 공통 UI·CSS, DataRoom 셸, Plugin SDK·로더, API 통신·Gen-TS, DB 연결과 마이그레이션 실행에 필요한 스켈레톤을 제공합니다. Review Plugin의 `health` 호출은 Rust 응답 DTO → Gen-TS → `host.call` → React Query가 이어지는 방식을 보여주는 예제입니다. 과제에서 구현할 업무 기능에는 포함되지 않습니다.

- 자료 관리는 **DataRoom**, 검토 작성·수정·현황은 **Review Plugin**에 구현합니다.
- 제공된 Plugin의 등록·번들·수명주기·호출 경계와 Gen-TS·React Query 연결 방식을 활용합니다.
- 업무 API 계약은 직접 설계합니다. 제공된 인증·검토 기준을 제외한 업무 DB 모델, 관계, 제약, 인덱싱, 마이그레이션, 시드도 직접 구성해 주세요. API 수와 화면 구성에는 제한이 없습니다.
- 프론트엔드 테스트에는 제공된 Playwright를 사용해 주세요. 테스트 코드는 제공하지 않으므로 테스트 전략·케이스·데이터를 직접 설계하고 작성해야 합니다. 백엔드 테스트 도구는 자유롭게 선택할 수 있습니다.

## 스켈레톤 코드 실행 방법

Docker Compose v2와 `make`가 필요합니다. 전달받은 압축을 푼 뒤 `Makefile`이 있는 `dataroom` 디렉터리에서 실행하세요.

```sh
cd dataroom
make dev
```

`make dev`는 PostgreSQL(`db`), Rust API(`api`), React 웹(`web`)을 Docker Compose로 실행하고, 세 서비스가 준비되면 종료됩니다. 처음 실행할 때는 이미지 빌드와 의존성 설치에 시간이 걸릴 수 있습니다. 브라우저에서 <http://localhost:5178/>을 열면 제공된 계정으로 로그인할 수 있습니다. 로그인 이후의 업무 화면은 아직 구현되어 있지 않습니다.

개발 중에는 같은 디렉터리에서 다음 명령을 사용하세요.

```sh
make logs             # 실행 로그 확인 (Ctrl+C로 로그 보기 종료)
make gen-ts-docker    # API 계약을 변경한 뒤 TypeScript 클라이언트 생성
make check-docker     # 생성 결과 일치·lint·타입·빌드 검사
make test-e2e         # tests/에 Playwright 테스트를 작성한 뒤 실행
make stop             # 서비스 종료 (DB 데이터는 보존)
make reset-db         # 서비스 종료 및 과제 DB 데이터 삭제
```

Rust API·Review Plugin 서버 소스와 마이그레이션을 수정하면 `cargo watch`가 API를 자동으로 다시 실행합니다. 웹과 Plugin UI도 변경을 감지해 다시 빌드합니다. Plugin UI가 갱신되면 브라우저를 새로 고침해 주세요.
포트가 겹치면 `WEB_PORT=5188 DB_PORT=5549 make dev`처럼 바꿀 수 있습니다. 이때 웹 주소는 <http://localhost:5188/>입니다. 의존성이나 설정을 변경한 뒤에는 해당 컨테이너를 재시작해 주세요.
Playwright 설정과 빈 `tests/` 폴더가 포함되어 있습니다. 테스트를 작성한 뒤 `make test-e2e`를 실행하면 Docker 환경에서 Chromium 데스크톱·모바일 화면을 검사할 수 있습니다. `make reset-db`는 제공된 초기 데이터까지 다시 만듭니다. 테스트별 데이터 격리 방법은 직접 설계해 주세요.

## 필수 기능

### 제공 계정과 접근권한

세 계정의 비밀번호는 모두 `dataroom`입니다.

| 역할        | 이메일                     |
| ----------- | -------------------------- |
| 기업 담당자 | `company@lighthouse.test`  |
| 투자자      | `investor@lighthouse.test` |
| 투자자      | `peer@lighthouse.test`     |

로그인·세션 복원·로그아웃은 스켈레톤에 포함되어 있습니다. 업무 API는 서버가 확인한 인증 사용자 정보를 기준으로 권한을 검사해야 합니다. 클라이언트가 전달한 사용자 ID나 역할을 신뢰하면 안 됩니다. 다른 사용자로 로그인하거나 Plugin에 다시 진입했을 때 이전 사용자의 데이터·캐시·작성 중인 입력이 섞이지 않아야 합니다.

| 행동                                 | 기업 담당자 | 투자자 |
| ------------------------------------ | ----------- | ------ |
| 데이터룸·자료·검토 기준 조회         | 가능        | 가능   |
| 자료 등록                            | 가능        | 불가   |
| 자신의 검토 조회·작성·수정·현황 확인 | 불가        | 가능   |
| 다른 투자자의 검토 조회              | 불가        | 불가   |

기업 담당자가 비공개 검토 목록을 요청하면 빈 목록을 반환하고, 화면에서도 검토·현황 기능을 사용할 수 없다는 점을 알 수 있어야 합니다.
추가 입력 필드의 허용 범위를 정해 주세요. 쓰기 권한이 없는 요청은 입력값을 검증하기 전에 거부해야 합니다.

### 자료 등록·조회

- 제목과 UTF-8 `.txt` 또는 `.md` 파일을 등록할 수 있어야 합니다. 브라우저에서 파일을 UTF-8 문자열로 읽은 뒤 파일명·본문과 함께 JSON으로 전송합니다. multipart 업로드는 구현하지 않아도 됩니다. 전체 요청은 제공된 256KiB 제한을 넘지 않아야 하며, 필요하다면 더 엄격한 제약을 정할 수 있습니다.
- 자료의 상태는 `ready`(준비 완료), `processing`(처리 중), `failed`(처리 실패)를 구분합니다.
- 목록에 제목·상태를 표시하고 제목 검색을 제공합니다. 작성 시각 내림차순, 같은 시각이면 ID 오름차순으로 정렬합니다.
- 상세 제목·파일명·상태·본문을 확인할 수 있어야 합니다.

### 검토 작성·수정

- 초기 마이그레이션에 포함된 사업 이해·팀 구성·매출 현황 기준을 불러와야 합니다. 기준의 ID·이름·검토 질문은 고정되어 있습니다. 기준 조회 API와 화면 연결은 직접 구현하되, 기준 생성·수정 기능은 구현하지 않아도 됩니다.
- 기준마다 `확인함(satisfied)` 또는 `추가 확인 필요(needs_information)`와 1~2000자의 의견을 저장합니다. 공백뿐인 의견은 거부합니다.
- 같은 데이터룸의 `ready` 자료를 하나 이상 근거로 연결합니다. 파일명이 아닌 자료 ID로 연결하며, 중복·없는 자료·다른 룸·처리 중·실패 자료는 거부합니다.
- 저장한 검토를 다시 열고 수정할 수 있어야 합니다. 투자자 한 명의 기준별 검토는 한 개이며 수정해도 같은 검토로 유지됩니다.
- 검토 내용과 근거 자료는 구분해 관리합니다. 자료 등록 성공만으로 해당 기준을 충족했다고 판단하지 않습니다.

### 선택 구현: AI 검토 초안

필수 기능을 완료한 뒤, 투자자가 **검토 기준과 등록된 자료를 바탕으로 검토 초안을 생성**하는 기능을 선택적으로 구현할 수 있습니다.

- 현재 데이터룸의 `ready` 자료만 사용해 기준별 판단, 의견과 근거 자료 ID를 제안합니다.
- 생성 결과는 검토 입력 화면의 **초안**으로 표시합니다. 투자자가 내용을 확인·수정하고 저장하기 전에는 검토 데이터와 현황에 반영하지 않습니다.
- 존재하지 않거나 다른 데이터룸에 속한 자료, `processing`·`failed` 자료를 근거로 제안하거나 저장하지 않습니다. 최종 저장 시에는 수동 작성과 같은 서버 검증을 적용합니다.
- 생성 중·실패·재시도 상태를 구분하고, AI 기능이 실패해도 수동 검토 작성 흐름은 사용할 수 있어야 합니다.
- 모델·프롬프트·검색 방식은 자유입니다. 특정 모델이나 벡터 DB 사용 자체보다 자료와 판단의 연결, 결과를 검증한 방법, 선택 이유를 확인합니다. 실행에 필요한 설정과 재현 방법을 제출 문서에 남기되 비밀 값은 포함하지 마세요.

이 기능은 선택 사항이며 별도 가산점이 없습니다. 필수 기능의 미완료를 대체하지 않습니다.

### 검토 현황

- 현재 투자자가 작성한 기준 수와 미작성 기준 수를 보여주고, 검토 내용과 연결된 근거 자료로 이동할 수 있어야 합니다.
- `추가 확인 필요`는 작성한 검토입니다. 미작성·확인함과 구분하며, 모든 기준을 작성했다고 모두 확인된 것은 아닙니다.
- 현황은 개인의 검토 진행 상태이며 회사 전체의 합의나 투자 승인 상태가 아닙니다.

## 공통 완료 조건

- **화면:** 로딩·오류·빈 결과를 구분합니다. 조회 실패를 자료 없음이나 검토 완료로 표시하지 않습니다. 등록·저장 성공 후 목록·현황·화면 이동에 반영하고, 실패하면 입력을 보존해 재시도할 수 있게 합니다.
- **사용성:** 상세 주소 새로고침과 Plugin 재진입을 처리합니다. 작은 화면, 폼 레이블, 키보드 접근을 지원합니다. 한국어 UI와 제공된 typed 문자열·스타일 토큰·UI scale을 사용하며 신규 영어 번역 완성도는 평가하지 않습니다.
- **저장:** 자료·검토·근거 연결은 PostgreSQL에 저장하고 API 재시작·DB 재연결 후에도 보존합니다. 샘플 JSON은 초기 입력이며 런타임 저장소가 아닙니다. 관계형 데이터를 JSON 덩어리 하나나 브라우저 저장소로 대체하지 않습니다.
- **일관성:** 검토 수정과 근거 교체는 한 트랜잭션으로 처리합니다. 중간 실패 시 기존 내용과 근거가 보존되고, 동시 저장에서도 검토 중복이나 서로 섞인 근거 목록이 생기지 않아야 합니다.
- **오류·데이터 보호:** 다른 투자자·자료실의 데이터가 응답에 노출되거나 근거로 연결되지 않아야 합니다. 잘못된 입력, 미인증, 권한 없음, 없는 자료·기준, 저장 실패를 구분하고 성공이나 빈 결과로 대체하지 않습니다.
- **DB 변경:** SQL은 파라미터 바인딩을 사용합니다. 적용한 마이그레이션을 덮어쓰거나 데이터를 지워 변경을 해결하지 않고 새 마이그레이션으로 남깁니다.
- **검증:** 권한·입력·오류·영속화·롤백·동시성·사용자 흐름을 검증하는 테스트를 직접 작성합니다. 프론트엔드 브라우저 테스트는 제공된 Playwright를 사용합니다. 테스트 데이터와 DB 격리·초기화 방법, 실행 명령·결과·미검증 범위를 제출합니다. mock과 실제 API·DB 검증을 구분하고, DB 연결 실패를 메모리 저장소의 성공으로 대체하지 않습니다.

새 라이브러리가 필요하다면 선택 이유를 설명하고 사용할 수 있습니다. 코드의 가독성과 필요한 검증을 우선해 주세요.

## 제외 범위

회원가입·외부 OAuth·메일 인증·초대·MFA·계정 복구, workspace 및 데이터룸 생성·삭제, 자료 삭제, 검토 기준 생성·수정, Plugin 설치 관리 UI, 실제 파일 스토리지, PDF·OCR·파일 변환, 실시간 협업·결제·클라우드 배포는 요구하지 않습니다.

샘플의 `processing`·`failed` 자료는 상태 표시와 근거 검증을 위한 고정 데이터입니다. 비동기 처리, 자동 상태 전환과 재시도 기능은 구현하지 않아도 됩니다. 업로드가 성공한 UTF-8 자료는 바로 `ready`로 저장할 수 있습니다.

## 진행·제출·평가

구현을 시작하기 전에 스켈레톤 코드를 살펴보고 **이해한 과제, 확인이 필요한 점과 가정, 구현 순서**를 공유해 주세요.
과제를 진행하는 동안 AI 도구를 적극적으로 활용해도 됩니다.
진행 중 질문과 범위에 대해 협의할 수 있습니다.
최종 코드는 GitHub로 제출해 주세요. 여러 커밋으로 나누어도 됩니다.

결과물에 대한 설명도 GitHub에 함께 남겨 주세요. 별도 양식은 없습니다.

- 실행·계정·DB 초기화 방법, 완료·미완료 범위, 대략적인 작업 시간과 별도의 환경 대응 시간.
- 구현 과정에서 달라진 서비스 이해와 그 근거, 주요 설계 판단과 대안. Plugin·Gen-TS 활용과 API·DB·UI 연결을 실제 코드로 설명해 주세요.
- 직접 작성한 테스트의 실행 명령·결과·한계. AI 활용 사례 2~3개에는 제공한 맥락, 실제 입력·응답 일부 또는 관련 변경, 채택·수정 판단과 검증 근거를 함께 적어 주세요.

| 평가 항목                 | 비중 | 확인할 내용                                                                                   |
| ------------------------- | ---: | --------------------------------------------------------------------------------------------- |
| Plugin·Gen-TS 이해와 확장 |  35% | 제공된 코드 이해, API에서 생성 TS·React Query·화면까지의 연결, 본체와 Plugin의 책임·호출 경계 |
| API·DB 모델 설계          |  35% | 제품 규칙에 맞는 계약·권한·모델·제약·저장 일관성                                              |
| UI 작성과 사용자 흐름     |  30% | 직접 구현한 업무 화면, 등록→검토→현황 흐름, 상태·오류 처리·사용성                             |


# Implementation Notes and Submission Documentation

The original assignment requirements and setup instructions above are kept intact. The sections below point to the documentation added during the implementation.

## Implementation Documentation

- [`approach.md`](./approach.md) — development approach, engineering decisions, debugging notes, and changes in understanding during implementation.
- [`docs/implementation.md`](./docs/implementation.md) — overview of the final architecture and how the DataRoom, Review Plugin, API, database, Gen-TS, and UI work together.
- [`docs/testing.md`](./docs/testing.md) — setup, test strategy, commands, test data, database reset/isolation, results, and known testing limitations.
- [`docs/ai-usage.md`](./docs/ai-usage.md) — concrete examples of how AI was used during the implementation, including what was accepted, changed, and verified.

## Initial Approach

Before implementation, I documented my understanding of the assignment, assumptions, and planned implementation order.

That initial plan was committed before development started and is preserved in Git history:

`ce3f5d1` — `docs: document initial implementation approach`

The later development decisions, implementation changes, debugging notes, and changes in understanding are documented in [`approach.md`](./approach.md).

## Development Time

- **Implementation and development:** approximately 12 hours
- **Environment setup and troubleshooting:** approximately 30 minutes to 1 hour

The development time covers implementation, debugging, testing, and integration of the Dataroom workflow. The environment/setup time covers getting the provided Docker-based development environment running and resolving setup-related issues before development could proceed.

## Final Scope

The required Dataroom workflow has been implemented:

**Company registers materials → Investor views available materials → Investor selects evidence → Investor creates a review for a criterion → Investor can edit the review/evidence → Investor checks review progress**

The optional AI Review Draft feature was not implemented because it was not required and has no additional bonus points.

For the final implementation details, testing instructions, and AI usage examples, see the documentation links above.