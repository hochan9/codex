## `sub-codex` local guide (KR/EN)

### 한국어

#### 목적
- 전역 설치(`npm -g`, Homebrew, `cargo install`) 없이 로컬 저장소에서 `sub-codex`를 실행하는 방법을 정리합니다.
- 이번 세션에서 반영된 sub-agent 관련 핵심 변경사항을 함께 요약합니다.

#### 설치 없이 실행하기
```bash
# 저장소 루트에서 실행
cd /Users/sonhochan/IdeaProjects/codex

# 1회성 실행 (빌드 + 실행)
cargo run --manifest-path codex-rs/Cargo.toml --bin sub-codex -- "explain this repository"

# 한 번 빌드 후 바이너리 직접 실행
cargo build --manifest-path codex-rs/Cargo.toml --bin sub-codex
./codex-rs/target/debug/sub-codex "explain this repository"
```

#### 반복 사용 (전역 설치 없이 커맨드처럼 사용)
```bash
sub-codex() {
  cargo run --manifest-path /Users/sonhochan/IdeaProjects/codex/codex-rs/Cargo.toml --bin sub-codex -- "$@"
}
```

#### 심볼릭 링크 방식 (`sub-codex` 이름으로 실행)
```bash
cd /Users/sonhochan/IdeaProjects/codex/codex-rs
cargo build -p codex-cli --bin codex

mkdir -p ~/.local/bin
ln -sf /Users/sonhochan/IdeaProjects/codex/codex-rs/target/debug/codex ~/.local/bin/sub-codex

grep -qxF 'export PATH="$HOME/.local/bin:$PATH"' ~/.zshrc || \
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

which sub-codex
sub-codex --version
```

#### 수정사항이 생겼을 때 운영 방법
- 코드 수정 후 다시 빌드:
```bash
cd /Users/sonhochan/IdeaProjects/codex/codex-rs
cargo build -p codex-cli --bin codex
```
- 그대로 `sub-codex` 실행:
```bash
sub-codex --version
sub-codex
```
- 참고:
  - `ln -sf`는 보통 최초 1회면 충분합니다. 같은 `target/debug/codex` 파일이 재빌드로 갱신됩니다.
  - `--release` 빌드를 쓰면 링크 경로를 `target/release/codex`로 맞춰야 합니다.
  - 브랜치를 바꿨으면 해당 브랜치에서 다시 `cargo build`만 실행하면 됩니다.

#### 이번 세션 변경사항 요약
- 코어 오케스트레이션 강화:
  - 자동 분할/worker 예산/재시도/timeout fallback 로직이 코어 런타임에서 결정적으로 동작하도록 반영.
- TUI 보드 개선:
  - sub-agent 진행 상태를 히스토리 셀이 아닌 고정 패널(`Sub-agent board`)로 분리.
- alias 동작 정리:
  - `sub-codex`, `codex-agent` 실행 시 alias 이름 기준 usage/help/배너 정리.
  - alias 실행 시 `features.collab=true` 기본값을 저우선순위로 주입.
- 안정화:
  - TUI 렌더 오버플로우 회귀 수정 및 관련 테스트 안정화.

---

### English

#### Purpose
- This document explains how to run `sub-codex` from the local repository without global installation (`npm -g`, Homebrew, or `cargo install`).
- It also summarizes the major sub-agent changes completed in this session.

#### Run without global install
```bash
# Run from the repository root
cd /Users/sonhochan/IdeaProjects/codex

# One-off run (build + run)
cargo run --manifest-path codex-rs/Cargo.toml --bin sub-codex -- "explain this repository"

# Build once, then run binary directly
cargo build --manifest-path codex-rs/Cargo.toml --bin sub-codex
./codex-rs/target/debug/sub-codex "explain this repository"
```

#### Reuse as a command (still no global install)
```bash
sub-codex() {
  cargo run --manifest-path /Users/sonhochan/IdeaProjects/codex/codex-rs/Cargo.toml --bin sub-codex -- "$@"
}
```

#### Symlink-based setup (`sub-codex` command name)
```bash
cd /Users/sonhochan/IdeaProjects/codex/codex-rs
cargo build -p codex-cli --bin codex

mkdir -p ~/.local/bin
ln -sf /Users/sonhochan/IdeaProjects/codex/codex-rs/target/debug/codex ~/.local/bin/sub-codex

grep -qxF 'export PATH="$HOME/.local/bin:$PATH"' ~/.zshrc || \
  echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc

which sub-codex
sub-codex --version
```

#### How to work after local changes
- Rebuild after code changes:
```bash
cd /Users/sonhochan/IdeaProjects/codex/codex-rs
cargo build -p codex-cli --bin codex
```
- Continue using `sub-codex` directly:
```bash
sub-codex --version
sub-codex
```
- Notes:
  - Usually, `ln -sf` is only needed once.
  - If you switch to `--release`, update the symlink to `target/release/codex`.
  - After changing branches, rebuild once on that branch.

#### Change summary (this session)
- Core orchestration upgrades:
  - deterministic runtime controls for auto-splitting, worker budgeting, retries, and timeout fallback.
- TUI improvement:
  - sub-agent progress moved from history cells to a fixed real-time panel (`Sub-agent board`).
- Alias behavior cleanup:
  - alias-aware usage/help/banner for `sub-codex` and `codex-agent`.
  - low-precedence default `features.collab=true` when launched via alias.
- Stability:
  - fixed a TUI rendering overflow regression and stabilized related tests.
