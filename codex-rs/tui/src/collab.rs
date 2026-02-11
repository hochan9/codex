use crate::history_cell::PlainHistoryCell;
use crate::render::line_utils::prefix_lines;
use crate::text_formatting::truncate_text;
use codex_core::protocol::AgentStatus;
use codex_core::protocol::CollabAgentInteractionEndEvent;
use codex_core::protocol::CollabAgentSpawnEndEvent;
use codex_core::protocol::CollabCloseEndEvent;
use codex_core::protocol::CollabResumeBeginEvent;
use codex_core::protocol::CollabResumeEndEvent;
use codex_core::protocol::CollabWaitingBeginEvent;
use codex_core::protocol::CollabWaitingEndEvent;
use codex_protocol::ThreadId;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;
use std::collections::HashMap;
use std::collections::HashSet;

const COLLAB_PROMPT_PREVIEW_GRAPHEMES: usize = 160;
const COLLAB_AGENT_ERROR_PREVIEW_GRAPHEMES: usize = 160;
const COLLAB_AGENT_RESPONSE_PREVIEW_GRAPHEMES: usize = 240;

#[derive(Debug, Clone, Default)]
pub(crate) struct ProgressBoard {
    pending_spawn_calls: HashSet<String>,
    queued_workers: HashSet<ThreadId>,
    active_workers: HashSet<ThreadId>,
    done_workers: HashMap<ThreadId, AgentStatus>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ProgressSummary {
    pub(crate) active_count: usize,
    pub(crate) queued_count: usize,
    pub(crate) pending_spawn_calls: usize,
    pub(crate) done_count: usize,
    pub(crate) completed_count: usize,
    pub(crate) errored_count: usize,
    pub(crate) shutdown_count: usize,
    pub(crate) not_found_count: usize,
}

impl ProgressSummary {
    pub(crate) fn is_empty(self) -> bool {
        self.active_count == 0 && self.queued_count == 0 && self.done_count == 0
    }
}

impl ProgressBoard {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn on_spawn_begin(&mut self, call_id: String) {
        self.pending_spawn_calls.insert(call_id);
    }

    pub(crate) fn on_spawn_end(
        &mut self,
        call_id: &str,
        new_thread_id: Option<ThreadId>,
        status: &AgentStatus,
    ) {
        self.pending_spawn_calls.remove(call_id);
        if let Some(thread_id) = new_thread_id {
            self.record_status(thread_id, status.clone());
        }
    }

    pub(crate) fn on_interaction_end(
        &mut self,
        receiver_thread_id: ThreadId,
        status: &AgentStatus,
    ) {
        self.record_status(receiver_thread_id, status.clone());
    }

    pub(crate) fn on_wait_begin(&mut self, receiver_thread_ids: &[ThreadId]) {
        for receiver_thread_id in receiver_thread_ids {
            if self.done_workers.contains_key(receiver_thread_id) {
                continue;
            }
            self.active_workers.remove(receiver_thread_id);
            self.queued_workers.insert(*receiver_thread_id);
        }
    }

    pub(crate) fn on_wait_end(&mut self, statuses: &HashMap<ThreadId, AgentStatus>) {
        for (receiver_thread_id, status) in statuses {
            self.queued_workers.remove(receiver_thread_id);
            self.record_status(*receiver_thread_id, status.clone());
        }
    }

    pub(crate) fn on_close_end(&mut self, receiver_thread_id: ThreadId) {
        self.queued_workers.remove(&receiver_thread_id);
        self.active_workers.remove(&receiver_thread_id);
        self.done_workers
            .insert(receiver_thread_id, AgentStatus::Shutdown);
    }

    pub(crate) fn on_resume_begin(&mut self, receiver_thread_id: ThreadId) {
        if self.done_workers.contains_key(&receiver_thread_id) {
            return;
        }
        self.active_workers.remove(&receiver_thread_id);
        self.queued_workers.insert(receiver_thread_id);
    }

    pub(crate) fn on_resume_end(&mut self, receiver_thread_id: ThreadId, status: &AgentStatus) {
        self.queued_workers.remove(&receiver_thread_id);
        self.record_status(receiver_thread_id, status.clone());
    }

    pub(crate) fn summary(&self) -> ProgressSummary {
        let mut completed_count = 0usize;
        let mut errored_count = 0usize;
        let mut shutdown_count = 0usize;
        let mut not_found_count = 0usize;
        for status in self.done_workers.values() {
            match status {
                AgentStatus::Completed(_) => completed_count += 1,
                AgentStatus::Errored(_) => errored_count += 1,
                AgentStatus::Shutdown => shutdown_count += 1,
                AgentStatus::NotFound => not_found_count += 1,
                AgentStatus::PendingInit | AgentStatus::Running => {}
            }
        }
        ProgressSummary {
            active_count: self.active_workers.len(),
            queued_count: self.queued_workers.len() + self.pending_spawn_calls.len(),
            pending_spawn_calls: self.pending_spawn_calls.len(),
            done_count: self.done_workers.len(),
            completed_count,
            errored_count,
            shutdown_count,
            not_found_count,
        }
    }

    fn record_status(&mut self, receiver_thread_id: ThreadId, status: AgentStatus) {
        self.queued_workers.remove(&receiver_thread_id);
        self.active_workers.remove(&receiver_thread_id);
        if is_terminal_status(&status) {
            self.done_workers.insert(receiver_thread_id, status);
            return;
        }
        self.done_workers.remove(&receiver_thread_id);
        match status {
            AgentStatus::PendingInit => {
                self.queued_workers.insert(receiver_thread_id);
            }
            AgentStatus::Running => {
                self.active_workers.insert(receiver_thread_id);
            }
            AgentStatus::Completed(_)
            | AgentStatus::Errored(_)
            | AgentStatus::Shutdown
            | AgentStatus::NotFound => {}
        }
    }
}

pub(crate) fn progress_board(summary: ProgressSummary) -> Option<PlainHistoryCell> {
    if summary.is_empty() {
        return None;
    }

    let active_line = detail_line_spans(
        "active",
        vec![
            Span::from(summary.active_count.to_string()).cyan().bold(),
            Span::from(" workers").dim(),
        ],
    );
    let mut queued_spans = vec![
        Span::from(summary.queued_count.to_string())
            .magenta()
            .bold(),
        Span::from(" workers").dim(),
    ];
    if summary.pending_spawn_calls > 0 {
        queued_spans.push(Span::from(" · ").dim());
        queued_spans
            .push(Span::from(format!("{} spawn pending", summary.pending_spawn_calls)).dim());
    }
    let queued_line = detail_line_spans("queued", queued_spans);

    let mut done_spans = vec![
        Span::from(summary.done_count.to_string()).green().bold(),
        Span::from(" workers").dim(),
    ];
    push_status_count(
        &mut done_spans,
        summary.completed_count,
        "completed",
        ratatui::prelude::Stylize::green,
    );
    push_status_count(
        &mut done_spans,
        summary.errored_count,
        "errored",
        ratatui::prelude::Stylize::red,
    );
    push_status_count(
        &mut done_spans,
        summary.shutdown_count,
        "shutdown",
        ratatui::prelude::Stylize::dim,
    );
    push_status_count(
        &mut done_spans,
        summary.not_found_count,
        "not found",
        ratatui::prelude::Stylize::red,
    );
    let done_line = detail_line_spans("done", done_spans);

    Some(PlainHistoryCell::new(vec![
        vec!["Sub-agent board".cyan().bold()].into(),
        active_line,
        queued_line,
        done_line,
    ]))
}

pub(crate) fn spawn_end(ev: CollabAgentSpawnEndEvent) -> PlainHistoryCell {
    let CollabAgentSpawnEndEvent {
        call_id,
        sender_thread_id: _,
        new_thread_id,
        prompt,
        status,
    } = ev;
    let new_agent = new_thread_id
        .map(|id| Span::from(id.to_string()))
        .unwrap_or_else(|| Span::from("not created").dim());
    let mut details = vec![
        detail_line("call", call_id),
        detail_line("agent", new_agent),
        status_line(&status),
    ];
    if let Some(line) = prompt_line(&prompt) {
        details.push(line);
    }
    collab_event("Agent spawned", details)
}

pub(crate) fn interaction_end(ev: CollabAgentInteractionEndEvent) -> PlainHistoryCell {
    let CollabAgentInteractionEndEvent {
        call_id,
        sender_thread_id: _,
        receiver_thread_id,
        prompt,
        status,
    } = ev;
    let mut details = vec![
        detail_line("call", call_id),
        detail_line("receiver", receiver_thread_id.to_string()),
        status_line(&status),
    ];
    if let Some(line) = prompt_line(&prompt) {
        details.push(line);
    }
    collab_event("Input sent", details)
}

pub(crate) fn waiting_begin(ev: CollabWaitingBeginEvent) -> PlainHistoryCell {
    let CollabWaitingBeginEvent {
        call_id,
        sender_thread_id: _,
        receiver_thread_ids,
    } = ev;
    let details = vec![
        detail_line("call", call_id),
        detail_line("receivers", format_thread_ids(&receiver_thread_ids)),
    ];
    collab_event("Waiting for agents", details)
}

pub(crate) fn waiting_end(ev: CollabWaitingEndEvent) -> PlainHistoryCell {
    let CollabWaitingEndEvent {
        call_id,
        sender_thread_id: _,
        statuses,
    } = ev;
    let mut details = vec![detail_line("call", call_id)];
    details.extend(wait_complete_lines(&statuses));
    collab_event("Wait complete", details)
}

pub(crate) fn close_end(ev: CollabCloseEndEvent) -> PlainHistoryCell {
    let CollabCloseEndEvent {
        call_id,
        sender_thread_id: _,
        receiver_thread_id,
        status,
    } = ev;
    let details = vec![
        detail_line("call", call_id),
        detail_line("receiver", receiver_thread_id.to_string()),
        status_line(&status),
    ];
    collab_event("Agent closed", details)
}

pub(crate) fn resume_begin(ev: CollabResumeBeginEvent) -> PlainHistoryCell {
    let CollabResumeBeginEvent {
        call_id,
        sender_thread_id: _,
        receiver_thread_id,
    } = ev;
    let details = vec![
        detail_line("call", call_id),
        detail_line("receiver", receiver_thread_id.to_string()),
    ];
    collab_event("Resuming agent", details)
}

pub(crate) fn resume_end(ev: CollabResumeEndEvent) -> PlainHistoryCell {
    let CollabResumeEndEvent {
        call_id,
        sender_thread_id: _,
        receiver_thread_id,
        status,
    } = ev;
    let details = vec![
        detail_line("call", call_id),
        detail_line("receiver", receiver_thread_id.to_string()),
        status_line(&status),
    ];
    collab_event("Agent resumed", details)
}

fn collab_event(title: impl Into<String>, details: Vec<Line<'static>>) -> PlainHistoryCell {
    let title = title.into();
    let mut lines: Vec<Line<'static>> =
        vec![vec![Span::from("• ").dim(), Span::from(title).bold()].into()];
    if !details.is_empty() {
        lines.extend(prefix_lines(details, "  └ ".dim(), "    ".into()));
    }
    PlainHistoryCell::new(lines)
}

fn detail_line(label: &str, value: impl Into<Span<'static>>) -> Line<'static> {
    vec![Span::from(format!("{label}: ")).dim(), value.into()].into()
}

fn status_line(status: &AgentStatus) -> Line<'static> {
    detail_line("status", status_span(status))
}

fn status_span(status: &AgentStatus) -> Span<'static> {
    match status {
        AgentStatus::PendingInit => Span::from("pending init").dim(),
        AgentStatus::Running => Span::from("running").cyan().bold(),
        AgentStatus::Completed(_) => Span::from("completed").green(),
        AgentStatus::Errored(_) => Span::from("errored").red(),
        AgentStatus::Shutdown => Span::from("shutdown").dim(),
        AgentStatus::NotFound => Span::from("not found").red(),
    }
}

fn prompt_line(prompt: &str) -> Option<Line<'static>> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(detail_line(
            "prompt",
            Span::from(truncate_text(trimmed, COLLAB_PROMPT_PREVIEW_GRAPHEMES)).dim(),
        ))
    }
}

fn format_thread_ids(ids: &[ThreadId]) -> Span<'static> {
    if ids.is_empty() {
        return Span::from("none").dim();
    }
    let joined = ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    Span::from(joined)
}

fn wait_complete_lines(statuses: &HashMap<ThreadId, AgentStatus>) -> Vec<Line<'static>> {
    if statuses.is_empty() {
        return vec![detail_line("agents", Span::from("none").dim())];
    }

    let mut pending_init = 0usize;
    let mut running = 0usize;
    let mut completed = 0usize;
    let mut errored = 0usize;
    let mut shutdown = 0usize;
    let mut not_found = 0usize;
    for status in statuses.values() {
        match status {
            AgentStatus::PendingInit => pending_init += 1,
            AgentStatus::Running => running += 1,
            AgentStatus::Completed(_) => completed += 1,
            AgentStatus::Errored(_) => errored += 1,
            AgentStatus::Shutdown => shutdown += 1,
            AgentStatus::NotFound => not_found += 1,
        }
    }

    let mut summary = vec![Span::from(format!("{} total", statuses.len())).dim()];
    push_status_count(
        &mut summary,
        pending_init,
        "pending init",
        ratatui::prelude::Stylize::dim,
    );
    push_status_count(&mut summary, running, "running", |span| span.cyan().bold());
    push_status_count(
        &mut summary,
        completed,
        "completed",
        ratatui::prelude::Stylize::green,
    );
    push_status_count(
        &mut summary,
        errored,
        "errored",
        ratatui::prelude::Stylize::red,
    );
    push_status_count(
        &mut summary,
        shutdown,
        "shutdown",
        ratatui::prelude::Stylize::dim,
    );
    push_status_count(
        &mut summary,
        not_found,
        "not found",
        ratatui::prelude::Stylize::red,
    );

    let mut entries: Vec<(String, &AgentStatus)> = statuses
        .iter()
        .map(|(thread_id, status)| (thread_id.to_string(), status))
        .collect();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    let mut lines = Vec::with_capacity(entries.len() + 1);
    lines.push(detail_line_spans("agents", summary));
    lines.extend(entries.into_iter().map(|(thread_id, status)| {
        let mut spans = vec![
            Span::from(thread_id).dim(),
            Span::from(" ").dim(),
            status_span(status),
        ];
        match status {
            AgentStatus::Completed(Some(message)) => {
                let message_preview = truncate_text(
                    &message.split_whitespace().collect::<Vec<_>>().join(" "),
                    COLLAB_AGENT_RESPONSE_PREVIEW_GRAPHEMES,
                );
                spans.push(Span::from(": ").dim());
                spans.push(Span::from(message_preview));
            }
            AgentStatus::Errored(error) => {
                let error_preview = truncate_text(
                    &error.split_whitespace().collect::<Vec<_>>().join(" "),
                    COLLAB_AGENT_ERROR_PREVIEW_GRAPHEMES,
                );
                spans.push(Span::from(": ").dim());
                spans.push(Span::from(error_preview).dim());
            }
            _ => {}
        }
        spans.into()
    }));
    lines
}

fn push_status_count(
    spans: &mut Vec<Span<'static>>,
    count: usize,
    label: &'static str,
    style: impl FnOnce(Span<'static>) -> Span<'static>,
) {
    if count == 0 {
        return;
    }

    spans.push(Span::from(" · ").dim());
    spans.push(style(Span::from(format!("{count} {label}"))));
}

fn detail_line_spans(label: &str, mut value: Vec<Span<'static>>) -> Line<'static> {
    let mut spans = Vec::with_capacity(value.len() + 1);
    spans.push(Span::from(format!("{label}: ")).dim());
    spans.append(&mut value);
    spans.into()
}

fn is_terminal_status(status: &AgentStatus) -> bool {
    matches!(
        status,
        AgentStatus::Completed(_)
            | AgentStatus::Errored(_)
            | AgentStatus::Shutdown
            | AgentStatus::NotFound
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn progress_board_tracks_spawn_wait_and_completion() {
        let mut board = ProgressBoard::default();
        let worker = ThreadId::new();

        board.on_spawn_begin("spawn-1".to_string());
        assert_eq!(
            board.summary(),
            ProgressSummary {
                queued_count: 1,
                pending_spawn_calls: 1,
                ..ProgressSummary::default()
            }
        );

        board.on_spawn_end("spawn-1", Some(worker), &AgentStatus::PendingInit);
        assert_eq!(
            board.summary(),
            ProgressSummary {
                queued_count: 1,
                ..ProgressSummary::default()
            }
        );

        board.on_interaction_end(worker, &AgentStatus::Running);
        assert_eq!(
            board.summary(),
            ProgressSummary {
                active_count: 1,
                ..ProgressSummary::default()
            }
        );

        board.on_wait_begin(&[worker]);
        assert_eq!(
            board.summary(),
            ProgressSummary {
                queued_count: 1,
                ..ProgressSummary::default()
            }
        );

        let mut statuses = HashMap::new();
        statuses.insert(worker, AgentStatus::Completed(Some("ok".to_string())));
        board.on_wait_end(&statuses);
        assert_eq!(
            board.summary(),
            ProgressSummary {
                done_count: 1,
                completed_count: 1,
                ..ProgressSummary::default()
            }
        );
    }

    #[test]
    fn progress_board_handles_close_as_shutdown() {
        let mut board = ProgressBoard::default();
        let worker = ThreadId::new();

        board.on_interaction_end(worker, &AgentStatus::Running);
        board.on_close_end(worker);

        assert_eq!(
            board.summary(),
            ProgressSummary {
                done_count: 1,
                shutdown_count: 1,
                ..ProgressSummary::default()
            }
        );
    }

    #[test]
    fn progress_board_render_returns_none_when_empty() {
        assert!(progress_board(ProgressSummary::default()).is_none());
    }
}
