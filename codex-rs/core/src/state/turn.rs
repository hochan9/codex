//! Turn-scoped state and active turn metadata scaffolding.

use indexmap::IndexMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tokio_util::task::AbortOnDropHandle;

use codex_protocol::ThreadId;
use codex_protocol::dynamic_tools::DynamicToolResponse;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::request_user_input::RequestUserInputResponse;
use tokio::sync::oneshot;

use crate::codex::TurnContext;
use crate::protocol::ReviewDecision;
use crate::tasks::SessionTask;

/// Metadata about the currently running turn.
pub(crate) struct ActiveTurn {
    pub(crate) tasks: IndexMap<String, RunningTask>,
    pub(crate) turn_state: Arc<Mutex<TurnState>>,
}

impl Default for ActiveTurn {
    fn default() -> Self {
        Self {
            tasks: IndexMap::new(),
            turn_state: Arc::new(Mutex::new(TurnState::default())),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskKind {
    Regular,
    Review,
    Compact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CollabDelegationIntent {
    SingleAgent,
    MultiAgent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CollabDelegationPolicy {
    pub(crate) intent: CollabDelegationIntent,
    pub(crate) hard_worker_limit: usize,
    pub(crate) recommended_workers: usize,
    pub(crate) signals: Vec<&'static str>,
}

pub(crate) struct RunningTask {
    pub(crate) done: Arc<Notify>,
    pub(crate) kind: TaskKind,
    pub(crate) task: Arc<dyn SessionTask>,
    pub(crate) cancellation_token: CancellationToken,
    pub(crate) handle: Arc<AbortOnDropHandle<()>>,
    pub(crate) turn_context: Arc<TurnContext>,
    // Timer recorded when the task drops to capture the full turn duration.
    pub(crate) _timer: Option<codex_otel::Timer>,
}

impl ActiveTurn {
    pub(crate) fn add_task(&mut self, task: RunningTask) {
        let sub_id = task.turn_context.sub_id.clone();
        self.tasks.insert(sub_id, task);
    }

    pub(crate) fn remove_task(&mut self, sub_id: &str) -> bool {
        self.tasks.swap_remove(sub_id);
        self.tasks.is_empty()
    }

    pub(crate) fn drain_tasks(&mut self) -> Vec<RunningTask> {
        self.tasks.drain(..).map(|(_, task)| task).collect()
    }
}

/// Mutable state for a single turn.
#[derive(Default)]
pub(crate) struct TurnState {
    pending_approvals: HashMap<String, oneshot::Sender<ReviewDecision>>,
    pending_user_input: HashMap<String, oneshot::Sender<RequestUserInputResponse>>,
    pending_dynamic_tools: HashMap<String, oneshot::Sender<DynamicToolResponse>>,
    pending_input: Vec<ResponseInputItem>,
    collab_policy: Option<CollabDelegationPolicy>,
    collab_known_workers: HashSet<ThreadId>,
    collab_pending_spawns: HashSet<String>,
    collab_spawn_attempts: HashMap<String, usize>,
    collab_wait_timeout_rounds: usize,
}

impl TurnState {
    pub(crate) fn insert_pending_approval(
        &mut self,
        key: String,
        tx: oneshot::Sender<ReviewDecision>,
    ) -> Option<oneshot::Sender<ReviewDecision>> {
        self.pending_approvals.insert(key, tx)
    }

    pub(crate) fn remove_pending_approval(
        &mut self,
        key: &str,
    ) -> Option<oneshot::Sender<ReviewDecision>> {
        self.pending_approvals.remove(key)
    }

    pub(crate) fn clear_pending(&mut self) {
        self.pending_approvals.clear();
        self.pending_user_input.clear();
        self.pending_dynamic_tools.clear();
        self.pending_input.clear();
        self.clear_collab_orchestration();
    }

    pub(crate) fn insert_pending_user_input(
        &mut self,
        key: String,
        tx: oneshot::Sender<RequestUserInputResponse>,
    ) -> Option<oneshot::Sender<RequestUserInputResponse>> {
        self.pending_user_input.insert(key, tx)
    }

    pub(crate) fn remove_pending_user_input(
        &mut self,
        key: &str,
    ) -> Option<oneshot::Sender<RequestUserInputResponse>> {
        self.pending_user_input.remove(key)
    }

    pub(crate) fn insert_pending_dynamic_tool(
        &mut self,
        key: String,
        tx: oneshot::Sender<DynamicToolResponse>,
    ) -> Option<oneshot::Sender<DynamicToolResponse>> {
        self.pending_dynamic_tools.insert(key, tx)
    }

    pub(crate) fn remove_pending_dynamic_tool(
        &mut self,
        key: &str,
    ) -> Option<oneshot::Sender<DynamicToolResponse>> {
        self.pending_dynamic_tools.remove(key)
    }

    pub(crate) fn push_pending_input(&mut self, input: ResponseInputItem) {
        self.pending_input.push(input);
    }

    pub(crate) fn take_pending_input(&mut self) -> Vec<ResponseInputItem> {
        if self.pending_input.is_empty() {
            Vec::with_capacity(0)
        } else {
            let mut ret = Vec::new();
            std::mem::swap(&mut ret, &mut self.pending_input);
            ret
        }
    }

    pub(crate) fn has_pending_input(&self) -> bool {
        !self.pending_input.is_empty()
    }

    pub(crate) fn ensure_collab_policy(
        &mut self,
        seed_prompt: &str,
        hard_worker_limit: usize,
    ) -> CollabDelegationPolicy {
        let hard_worker_limit = hard_worker_limit.max(1);
        if let Some(policy) = self.collab_policy.clone() {
            return policy;
        }
        let policy = derive_collab_policy(seed_prompt, hard_worker_limit);
        self.collab_policy = Some(policy.clone());
        policy
    }

    pub(crate) fn collab_worker_budget_exhausted(&self) -> bool {
        let hard_limit = self
            .collab_policy
            .as_ref()
            .map(|policy| policy.hard_worker_limit)
            .unwrap_or(1);
        self.collab_known_workers.len() + self.collab_pending_spawns.len() >= hard_limit
    }

    pub(crate) fn collab_worker_budget(&self) -> (usize, usize) {
        let hard_limit = self
            .collab_policy
            .as_ref()
            .map(|policy| policy.hard_worker_limit)
            .unwrap_or(1);
        (
            hard_limit,
            self.collab_known_workers.len() + self.collab_pending_spawns.len(),
        )
    }

    pub(crate) fn mark_collab_spawn_pending(&mut self, call_id: String) {
        self.collab_pending_spawns.insert(call_id);
    }

    pub(crate) fn mark_collab_spawn_succeeded(&mut self, call_id: &str, thread_id: ThreadId) {
        self.collab_pending_spawns.remove(call_id);
        self.collab_known_workers.insert(thread_id);
        self.collab_spawn_attempts.remove(call_id);
        self.collab_wait_timeout_rounds = 0;
    }

    pub(crate) fn mark_collab_spawn_failed(&mut self, call_id: &str) {
        self.collab_pending_spawns.remove(call_id);
    }

    pub(crate) fn mark_collab_worker_closed(&mut self, thread_id: ThreadId) {
        self.collab_known_workers.remove(&thread_id);
    }

    pub(crate) fn next_collab_spawn_attempt(&mut self, call_id: &str) -> usize {
        let attempts = self
            .collab_spawn_attempts
            .entry(call_id.to_string())
            .or_insert(0);
        *attempts += 1;
        *attempts
    }

    pub(crate) fn record_collab_wait_timeout(&mut self) -> usize {
        self.collab_wait_timeout_rounds += 1;
        self.collab_wait_timeout_rounds
    }

    pub(crate) fn reset_collab_wait_timeout_rounds(&mut self) {
        self.collab_wait_timeout_rounds = 0;
    }

    fn clear_collab_orchestration(&mut self) {
        self.collab_policy = None;
        self.collab_known_workers.clear();
        self.collab_pending_spawns.clear();
        self.collab_spawn_attempts.clear();
        self.collab_wait_timeout_rounds = 0;
    }
}

const COLLAB_MIN_PROMPT_GRAPHEMES_FOR_COMPLEXITY: usize = 280;
const COLLAB_MIN_SIGNALS_FOR_MULTI_AGENT: usize = 2;

fn derive_collab_policy(seed_prompt: &str, hard_worker_limit: usize) -> CollabDelegationPolicy {
    if hard_worker_limit <= 1 {
        return CollabDelegationPolicy {
            intent: CollabDelegationIntent::SingleAgent,
            hard_worker_limit,
            recommended_workers: 1,
            signals: vec!["worker_limit_1"],
        };
    }

    let mut signals: Vec<&'static str> = Vec::new();
    let normalized = seed_prompt.to_lowercase();

    if seed_prompt.trim().chars().count() >= COLLAB_MIN_PROMPT_GRAPHEMES_FOR_COMPLEXITY {
        signals.push("long_prompt");
    }
    if normalized.contains("parallel")
        || normalized.contains("simultaneous")
        || normalized.contains("동시에")
        || normalized.contains("병렬")
    {
        signals.push("parallel_hint");
    }
    if has_multi_domain_signal(&normalized) {
        signals.push("multi_domain");
    }
    if normalized.contains("multiple files")
        || normalized.contains("many files")
        || normalized.contains("across the codebase")
        || normalized.contains("cross-cutting")
        || normalized.contains("여러 파일")
        || normalized.contains("코드베이스 전반")
    {
        signals.push("file_fanout");
    }
    if seed_prompt
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count()
        >= 4
    {
        signals.push("multi_step");
    }

    let multi_agent = signals.len() >= COLLAB_MIN_SIGNALS_FOR_MULTI_AGENT;
    if !multi_agent {
        return CollabDelegationPolicy {
            intent: CollabDelegationIntent::SingleAgent,
            hard_worker_limit: 1,
            recommended_workers: 1,
            signals,
        };
    }

    let recommended_workers = (signals.len() + 1).max(2).min(hard_worker_limit);
    CollabDelegationPolicy {
        intent: CollabDelegationIntent::MultiAgent,
        hard_worker_limit,
        recommended_workers,
        signals,
    }
}

fn has_multi_domain_signal(normalized: &str) -> bool {
    let domain_keywords: [&[&str]; 4] = [
        &["frontend", "ui", "react", "css", "ux", "web"],
        &["backend", "api", "server", "database", "db", "service"],
        &[
            "devops",
            "infra",
            "deploy",
            "docker",
            "kubernetes",
            "ci",
            "cd",
        ],
        &[
            "test",
            "qa",
            "coverage",
            "docs",
            "documentation",
            "테스트",
            "문서",
        ],
    ];
    let matched = domain_keywords
        .iter()
        .filter(|domain| domain.iter().any(|keyword| normalized.contains(keyword)))
        .count();
    matched >= 2
}

impl ActiveTurn {
    /// Clear any pending approvals and input buffered for the current turn.
    pub(crate) async fn clear_pending(&self) {
        let mut ts = self.turn_state.lock().await;
        ts.clear_pending();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn collab_policy_defaults_to_single_agent_for_small_prompt() {
        let mut turn_state = TurnState::default();
        let policy = turn_state.ensure_collab_policy("rename this variable", 6);
        assert_eq!(
            policy,
            CollabDelegationPolicy {
                intent: CollabDelegationIntent::SingleAgent,
                hard_worker_limit: 1,
                recommended_workers: 1,
                signals: vec![],
            }
        );
    }

    #[test]
    fn collab_policy_promotes_multi_agent_for_parallel_multi_domain_prompt() {
        let mut turn_state = TurnState::default();
        let policy = turn_state.ensure_collab_policy(
            "Parallelize this task: update frontend React UI, backend API service, and CI deploy docs across multiple files.\n1. list files\n2. implement\n3. test\n4. document",
            6,
        );
        assert_eq!(policy.intent, CollabDelegationIntent::MultiAgent);
        assert_eq!(policy.hard_worker_limit, 6);
        assert_eq!(policy.recommended_workers, 5);
        assert!(policy.signals.contains(&"parallel_hint"));
        assert!(policy.signals.contains(&"multi_domain"));
        assert!(policy.signals.contains(&"file_fanout"));
        assert!(policy.signals.contains(&"multi_step"));
    }

    #[test]
    fn collab_worker_budget_counts_pending_and_known_workers() {
        let mut turn_state = TurnState::default();
        let policy = turn_state
            .ensure_collab_policy("parallel frontend backend docs with multiple files", 3);
        assert_eq!(policy.hard_worker_limit, 3);
        assert!(!turn_state.collab_worker_budget_exhausted());

        turn_state.mark_collab_spawn_pending("call-1".to_string());
        turn_state.mark_collab_spawn_pending("call-2".to_string());
        assert!(!turn_state.collab_worker_budget_exhausted());

        let worker = ThreadId::new();
        turn_state.mark_collab_spawn_succeeded("call-1", worker);
        assert!(!turn_state.collab_worker_budget_exhausted());
        turn_state.mark_collab_spawn_succeeded("call-2", ThreadId::new());
        assert!(!turn_state.collab_worker_budget_exhausted());
        turn_state.mark_collab_spawn_pending("call-3".to_string());
        assert!(turn_state.collab_worker_budget_exhausted());

        turn_state.mark_collab_worker_closed(worker);
        assert!(!turn_state.collab_worker_budget_exhausted());
    }

    #[test]
    fn collab_wait_timeout_rounds_reset_after_success() {
        let mut turn_state = TurnState::default();
        assert_eq!(turn_state.record_collab_wait_timeout(), 1);
        assert_eq!(turn_state.record_collab_wait_timeout(), 2);
        turn_state.reset_collab_wait_timeout_rounds();
        assert_eq!(turn_state.record_collab_wait_timeout(), 1);
    }
}
