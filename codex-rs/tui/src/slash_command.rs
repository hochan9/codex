use strum::IntoEnumIterator;
use strum_macros::AsRefStr;
use strum_macros::EnumIter;
use strum_macros::EnumString;
use strum_macros::IntoStaticStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandDescriptionLanguage {
    #[default]
    English,
    Korean,
}

impl CommandDescriptionLanguage {
    pub fn display_name(self) -> &'static str {
        match self {
            CommandDescriptionLanguage::English => "English",
            CommandDescriptionLanguage::Korean => "Korean",
        }
    }
}

/// Commands that can be invoked by starting a message with a leading slash.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString, EnumIter, AsRefStr, IntoStaticStr,
)]
#[strum(serialize_all = "kebab-case")]
pub enum SlashCommand {
    // DO NOT ALPHA-SORT! Enum order is presentation order in the popup, so
    // more frequently used commands should be listed first.
    Model,
    Language,
    Approvals,
    Permissions,
    #[strum(serialize = "setup-elevated-sandbox")]
    ElevateSandbox,
    Experimental,
    Skills,
    Review,
    Rename,
    New,
    Resume,
    Fork,
    Init,
    Compact,
    Plan,
    Collab,
    Agent,
    Agents,
    // Undo,
    Diff,
    Mention,
    Status,
    DebugConfig,
    Statusline,
    Mcp,
    Apps,
    Logout,
    Quit,
    Exit,
    Feedback,
    Rollout,
    Ps,
    Clean,
    Personality,
    TestApproval,
}

impl SlashCommand {
    /// User-visible description shown in the popup.
    pub fn description(self) -> &'static str {
        self.description_in(CommandDescriptionLanguage::English)
    }

    pub fn description_in(self, language: CommandDescriptionLanguage) -> &'static str {
        match language {
            CommandDescriptionLanguage::English => self.description_en(),
            CommandDescriptionLanguage::Korean => self.description_ko(),
        }
    }

    fn description_en(self) -> &'static str {
        match self {
            SlashCommand::Feedback => "send logs to maintainers",
            SlashCommand::New => "start a new chat during a conversation",
            SlashCommand::Init => "create an AGENTS.md file with instructions for Codex",
            SlashCommand::Compact => "summarize conversation to prevent hitting the context limit",
            SlashCommand::Review => "review my current changes and find issues",
            SlashCommand::Rename => "rename the current thread",
            SlashCommand::Resume => "resume a saved chat",
            SlashCommand::Fork => "fork the current chat",
            // SlashCommand::Undo => "ask Codex to undo a turn",
            SlashCommand::Quit | SlashCommand::Exit => "exit Codex",
            SlashCommand::Diff => "show git diff (including untracked files)",
            SlashCommand::Mention => "mention a file",
            SlashCommand::Skills => "use skills to improve how Codex performs specific tasks",
            SlashCommand::Status => "show current session configuration and token usage",
            SlashCommand::DebugConfig => "show config layers and requirement sources for debugging",
            SlashCommand::Statusline => "configure which items appear in the status line",
            SlashCommand::Ps => "list background terminals",
            SlashCommand::Clean => "stop all background terminals",
            SlashCommand::Model => "choose what model and reasoning effort to use",
            SlashCommand::Language => "choose slash-command description language",
            SlashCommand::Personality => "choose a communication style for Codex",
            SlashCommand::Plan => "switch to Plan mode",
            SlashCommand::Collab => "change collaboration mode (experimental)",
            SlashCommand::Agent => "switch the active agent thread",
            SlashCommand::Agents => "manage and switch named specialist agents",
            SlashCommand::Approvals => "choose what Codex is allowed to do",
            SlashCommand::Permissions => "choose what Codex is allowed to do",
            SlashCommand::ElevateSandbox => "set up elevated agent sandbox",
            SlashCommand::Experimental => "toggle experimental features",
            SlashCommand::Mcp => "list configured MCP tools",
            SlashCommand::Apps => "manage apps",
            SlashCommand::Logout => "log out of Codex",
            SlashCommand::Rollout => "print the rollout file path",
            SlashCommand::TestApproval => "test approval request",
        }
    }

    fn description_ko(self) -> &'static str {
        match self {
            SlashCommand::Feedback => "유지보수팀으로 로그를 전송합니다",
            SlashCommand::New => "대화 중 새 채팅을 시작합니다",
            SlashCommand::Init => "Codex용 지침이 담긴 AGENTS.md 파일을 생성합니다",
            SlashCommand::Compact => "컨텍스트 한도를 넘지 않도록 대화를 요약합니다",
            SlashCommand::Review => "현재 변경사항을 리뷰하고 이슈를 찾습니다",
            SlashCommand::Rename => "현재 스레드 이름을 변경합니다",
            SlashCommand::Resume => "저장된 채팅을 다시 엽니다",
            SlashCommand::Fork => "현재 채팅을 포크합니다",
            // SlashCommand::Undo => "ask Codex to undo a turn",
            SlashCommand::Quit | SlashCommand::Exit => "Codex를 종료합니다",
            SlashCommand::Diff => "git diff를 표시합니다(추적되지 않은 파일 포함)",
            SlashCommand::Mention => "파일을 멘션합니다",
            SlashCommand::Skills => "특정 작업 성능을 높이기 위해 skills 기능을 사용합니다",
            SlashCommand::Status => "현재 세션 설정과 토큰 사용량을 표시합니다",
            SlashCommand::DebugConfig => "디버깅을 위해 설정 계층과 요구사항 출처를 표시합니다",
            SlashCommand::Statusline => "상태줄에 표시할 항목을 설정합니다",
            SlashCommand::Ps => "백그라운드 터미널 목록을 표시합니다",
            SlashCommand::Clean => "백그라운드 터미널을 모두 중지합니다",
            SlashCommand::Model => "사용할 모델과 추론 강도를 선택합니다",
            SlashCommand::Language => "슬래시 명령 설명 언어를 선택합니다",
            SlashCommand::Personality => "Codex의 커뮤니케이션 스타일을 선택합니다",
            SlashCommand::Plan => "Plan 모드로 전환합니다",
            SlashCommand::Collab => "협업 모드를 변경합니다(실험 기능)",
            SlashCommand::Agent => "활성 에이전트 스레드를 전환합니다",
            SlashCommand::Agents => "이름 있는 전문 에이전트를 관리하고 전환합니다",
            SlashCommand::Approvals => "Codex 권한 수준을 설정합니다",
            SlashCommand::Permissions => "Codex 권한 수준을 설정합니다",
            SlashCommand::ElevateSandbox => "권한 상승 에이전트 샌드박스를 설정합니다",
            SlashCommand::Experimental => "실험 기능을 켜거나 끕니다",
            SlashCommand::Mcp => "설정된 MCP 도구를 표시합니다",
            SlashCommand::Apps => "앱을 관리합니다",
            SlashCommand::Logout => "Codex에서 로그아웃합니다",
            SlashCommand::Rollout => "롤아웃 파일 경로를 출력합니다",
            SlashCommand::TestApproval => "승인 요청 테스트를 실행합니다",
        }
    }

    /// Command string without the leading '/'. Provided for compatibility with
    /// existing code that expects a method named `command()`.
    pub fn command(self) -> &'static str {
        self.into()
    }

    /// Whether this command supports inline args (for example `/review ...`).
    pub fn supports_inline_args(self) -> bool {
        matches!(
            self,
            SlashCommand::Review | SlashCommand::Rename | SlashCommand::Plan
        )
    }

    /// Whether this command can be run while a task is in progress.
    pub fn available_during_task(self) -> bool {
        match self {
            SlashCommand::New
            | SlashCommand::Resume
            | SlashCommand::Fork
            | SlashCommand::Init
            | SlashCommand::Compact
            // | SlashCommand::Undo
            | SlashCommand::Model
            | SlashCommand::Language
            | SlashCommand::Personality
            | SlashCommand::Approvals
            | SlashCommand::Permissions
            | SlashCommand::ElevateSandbox
            | SlashCommand::Experimental
            | SlashCommand::Review
            | SlashCommand::Plan
            | SlashCommand::Logout => false,
            SlashCommand::Diff
            | SlashCommand::Rename
            | SlashCommand::Mention
            | SlashCommand::Skills
            | SlashCommand::Status
            | SlashCommand::DebugConfig
            | SlashCommand::Ps
            | SlashCommand::Clean
            | SlashCommand::Mcp
            | SlashCommand::Apps
            | SlashCommand::Feedback
            | SlashCommand::Quit
            | SlashCommand::Exit => true,
            SlashCommand::Rollout => true,
            SlashCommand::TestApproval => true,
            SlashCommand::Collab => true,
            SlashCommand::Agent => true,
            SlashCommand::Agents => true,
            SlashCommand::Statusline => false,
        }
    }

    fn is_visible(self) -> bool {
        match self {
            SlashCommand::Rollout | SlashCommand::TestApproval => cfg!(debug_assertions),
            _ => true,
        }
    }
}

/// Return all built-in commands in a Vec paired with their command string.
pub fn built_in_slash_commands() -> Vec<(&'static str, SlashCommand)> {
    SlashCommand::iter()
        .filter(|command| command.is_visible())
        .map(|c| (c.command(), c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn language_command_has_expected_slug() {
        assert_eq!(SlashCommand::Language.command(), "language");
    }

    #[test]
    fn status_description_is_localized_for_korean() {
        assert_eq!(
            SlashCommand::Status.description_in(CommandDescriptionLanguage::Korean),
            "현재 세션 설정과 토큰 사용량을 표시합니다"
        );
    }
}
