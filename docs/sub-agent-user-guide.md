## Sub-agent 사용자 가이드 (한글)

### 1) 이 문서의 목적
- `codex`/`sub-codex`를 실제로 사용할 때 필요한 핵심만 빠르게 이해하도록 정리한 가이드입니다.
- 특히 이번에 반영된 sub-agent 오케스트레이션, TUI 고정 보드, alias 동작을 중심으로 설명합니다.

### 2) 빠른 시작
```bash
cd /Users/sonhochan/IdeaProjects/codex/codex-rs

# 기본 실행
cargo run --bin codex -- "프로젝트 구조 설명해줘"

# sub-agent 기본값(collab)으로 실행
cargo run --bin sub-codex -- "프론트/백엔드/테스트를 병렬로 진행해줘"
```

전역 설치 없이 로컬에서 alias처럼 쓰고 싶다면 `docs/sub-codex-local-guide.md`를 참고하세요.

### 3) `codex` vs `sub-codex`
- `codex`:
  - 기본 실행 진입점입니다.
  - 필요 시 collab을 켜서 sub-agent 흐름을 사용할 수 있습니다.
- `sub-codex`:
  - sub-agent 사용을 염두에 둔 alias 진입점입니다.
  - 실행 시 `features.collab=true` 기본값이 저우선순위로 적용됩니다.
  - 도움말/usage/배너 표기가 alias 기준으로 동작합니다.

### 4) TUI에서 볼 수 있는 것
- 대화 영역:
  - 일반 응답/툴 실행 이벤트가 표시됩니다.
- `Sub-agent board` (고정 패널):
  - `active`: 현재 실행 중 worker 수
  - `queued`: 대기 중 worker/spawn 수
  - `done`: 완료/에러/종료 상태 합계
  - 히스토리 셀과 분리된 고정 패널이라 실시간 상태 추적이 쉽습니다.

### 5) 오케스트레이션이 실제로 하는 일
- spawn 전 worker 예산 확인:
  - turn 단위 정책에서 hard worker limit을 넘으면 새 spawn을 거절합니다.
- spawn 실패 재시도:
  - retry 가능한 실패는 제한 횟수 내에서 자동 재시도합니다.
- wait timeout 처리:
  - timeout이 반복되면 fallback 정책에 따라 정체 worker를 정리(close)할 수 있습니다.
- 핵심 포인트:
  - 이 동작은 프롬프트 문구가 아니라 코어 런타임 로직에서 결정됩니다.

### 6) 자주 쓰는 명령/흐름
- `/agents`:
  - 에이전트 생성/전환/관리
- `/language`:
  - 명령 설명 언어 전환 (영어/한국어)
- `/status`:
  - 현재 세션/실행 상태 확인
- 다중 작업 요청 예시:
  - "UI 수정 + API 변경 + 테스트 보강을 병렬로 진행해줘"
  - 이런 입력이 sub-agent 정책 신호를 강하게 만들어 worker 분할 가능성이 올라갑니다.

### 7) 바로 복붙 가능한 프롬프트 예제 (KR/EN)
- 보드가 뜨는지 확인용 (한국어):
  - "다음 작업을 병렬로 나눠서 진행해줘. 1) `codex-rs/core`에서 collab timeout 정책 점검, 2) `codex-rs/tui` 보드 렌더링 점검, 3) `codex-rs/cli` alias 도움말 점검. 각 작업 결과를 요약하고 마지막에 통합 리포트로 정리해줘."
- 보드가 뜨는지 확인용 (영어):
  - "Split this into parallel workers: (1) audit collab timeout policy in `codex-rs/core`, (2) verify sub-agent board rendering in `codex-rs/tui`, (3) verify alias/help behavior in `codex-rs/cli`. Return a combined final report."
- 코드 리뷰 전용:
  - "코드 리뷰 에이전트 관점으로 `codex-rs/core/src/tools/handlers/collab.rs`의 실패/재시도/타임아웃 처리 리스크를 찾아줘."
- 디버깅 전용:
  - "`codex-exec-server` 테스트 실패 원인을 재현 가능한 단계로 분석하고, 최소 변경으로 고치는 패치를 제안해줘."
- 문서화 전용:
  - "`docs/sub-agent-user-guide.md`에 초보자용 사용 시나리오를 3개 추가해줘. 한국어/영어 예제를 같이 넣어줘."
- 언어 설정과 함께 쓰기:
  - 1) `/language ko` 후 한국어 프롬프트 실행
  - 2) `/language en` 후 영어 프롬프트 실행
  - 명령 설명과 인터랙션 언어를 맞춰서 사용할 수 있습니다.

### 8) 코드 수정 후 운영 방법
```bash
cd /Users/sonhochan/IdeaProjects/codex/codex-rs
cargo build -p codex-cli --bin codex
sub-codex --version
sub-codex
```

- 링크 방식(`~/.local/bin/sub-codex -> target/debug/codex`)을 쓰는 경우, 보통 최초 1회 링크 후에는 재빌드만 하면 됩니다.
- `--release` 빌드를 쓰면 링크 대상도 `target/release/codex`로 맞춰야 합니다.

### 9) 트러블슈팅
- `command not found: sub-codex`:
  - PATH에 alias 경로가 없는 상태입니다. `docs/sub-codex-local-guide.md`의 링크/함수 설정을 적용하세요.
- 통합 테스트에서 `No such file or directory (os error 2)`:
  - 환경 의존 도구 누락(예: dotslash)일 수 있습니다.
  - 최근 수정으로 `codex-exec-server`의 관련 테스트는 dotslash 미설치 시 스킵되도록 정리되었습니다.
