use codex_protocol::openai_models::ReasoningEffort;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::io;
use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs;

const STORE_VERSION: u32 = 1;
const AGENTS_FILENAME: &str = "agents.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub prompt: String,
    pub model: Option<String>,
    pub reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentsStore {
    #[serde(default = "default_store_version")]
    pub version: u32,
    #[serde(default)]
    pub active_agent_id: Option<String>,
    #[serde(default)]
    pub agents: Vec<AgentDefinition>,
    #[serde(default)]
    pub thread_bindings: BTreeMap<String, String>,
}

impl Default for AgentsStore {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            active_agent_id: None,
            agents: built_in_agents(),
            thread_bindings: BTreeMap::new(),
        }
    }
}

impl AgentsStore {
    pub fn normalize(self) -> Self {
        let mut seen_ids = HashSet::new();
        let agents: Vec<AgentDefinition> = self
            .agents
            .into_iter()
            .filter_map(normalize_agent)
            .filter(|agent| seen_ids.insert(agent.id.clone()))
            .collect();

        let valid_ids: HashSet<String> = agents.iter().map(|agent| agent.id.clone()).collect();
        let active_agent_id = self
            .active_agent_id
            .as_deref()
            .map(str::trim)
            .and_then(|id| valid_ids.contains(id).then(|| id.to_string()));

        let thread_bindings: BTreeMap<String, String> = self
            .thread_bindings
            .into_iter()
            .filter_map(|(agent_id, thread_id)| {
                let trimmed_thread_id = thread_id.trim();
                if !valid_ids.contains(&agent_id) || trimmed_thread_id.is_empty() {
                    return None;
                }
                Some((agent_id, trimmed_thread_id.to_string()))
            })
            .collect();

        Self {
            version: STORE_VERSION,
            active_agent_id,
            agents,
            thread_bindings,
        }
    }

    pub fn get_agent(&self, id: &str) -> Option<&AgentDefinition> {
        self.agents.iter().find(|agent| agent.id == id)
    }

    pub fn active_agent(&self) -> Option<&AgentDefinition> {
        self.active_agent_id
            .as_deref()
            .and_then(|id| self.get_agent(id))
    }

    pub fn set_active_agent_id(&mut self, id: Option<String>) {
        self.active_agent_id =
            id.and_then(|candidate| self.get_agent(&candidate).is_some().then_some(candidate));
    }

    pub fn upsert_agent(&mut self, agent: AgentDefinition) {
        if let Some(existing_index) = self
            .agents
            .iter()
            .position(|existing| existing.id == agent.id)
        {
            self.agents[existing_index] = agent;
        } else {
            self.agents.push(agent);
            self.agents
                .sort_by(|left, right| left.name.cmp(&right.name));
        }
    }

    pub fn remove_agent(&mut self, id: &str) -> Option<AgentDefinition> {
        let index = self.agents.iter().position(|agent| agent.id == id)?;
        let removed = self.agents.remove(index);
        if self.active_agent_id.as_deref() == Some(id) {
            self.active_agent_id = None;
        }
        self.thread_bindings.remove(id);
        Some(removed)
    }

    pub fn thread_binding(&self, id: &str) -> Option<&str> {
        self.thread_bindings.get(id).map(String::as_str)
    }

    pub fn set_thread_binding(&mut self, id: &str, thread_id: String) {
        if self.get_agent(id).is_some() {
            self.thread_bindings.insert(id.to_string(), thread_id);
        }
    }

    pub fn find_agent_id_for_thread(&self, thread_id: &str) -> Option<&str> {
        self.thread_bindings
            .iter()
            .find_map(|(agent_id, bound_thread)| {
                (bound_thread == thread_id).then_some(agent_id.as_str())
            })
    }
}

pub fn built_in_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            id: "architect".to_string(),
            name: "Architect".to_string(),
            description: Some("System design, tradeoffs, and implementation planning.".to_string()),
            prompt: "Focus on architecture decisions. Define boundaries, interfaces, and migration-safe rollout plans before coding. Highlight tradeoffs and risks explicitly.".to_string(),
            model: None,
            reasoning_effort: None,
        },
        AgentDefinition {
            id: "code-reviewer".to_string(),
            name: "Code Reviewer".to_string(),
            description: Some("Code quality, regressions, and missing tests.".to_string()),
            prompt: "Review with a bug-first mindset. Prioritize correctness, regressions, edge cases, and test coverage gaps. Be explicit about severity and concrete fixes.".to_string(),
            model: None,
            reasoning_effort: None,
        },
        AgentDefinition {
            id: "debugger".to_string(),
            name: "Debugger".to_string(),
            description: Some("Root-cause analysis and targeted fixes.".to_string()),
            prompt: "Diagnose failures by reproducing symptoms, narrowing hypotheses, and proving root cause with concrete evidence. Prefer minimal, verifiable fixes.".to_string(),
            model: None,
            reasoning_effort: None,
        },
        AgentDefinition {
            id: "documentation".to_string(),
            name: "Documentation".to_string(),
            description: Some("User-facing docs and developer handoff materials.".to_string()),
            prompt: "Write clear, precise documentation with runnable examples, operational caveats, and maintenance notes. Keep terminology consistent with the codebase.".to_string(),
            model: None,
            reasoning_effort: None,
        },
        AgentDefinition {
            id: "test-writer".to_string(),
            name: "Test Writer".to_string(),
            description: Some("Regression tests and coverage expansion.".to_string()),
            prompt: "Design deterministic tests that validate externally visible behavior, protect regressions, and keep maintenance cost low. Prefer full-object assertions when possible.".to_string(),
            model: None,
            reasoning_effort: None,
        },
    ]
}

pub fn agents_store_path(codex_home: &Path) -> PathBuf {
    codex_home.join(AGENTS_FILENAME)
}

pub async fn load_agents_store(codex_home: &Path) -> io::Result<AgentsStore> {
    let path = agents_store_path(codex_home);
    let raw = match fs::read_to_string(&path).await {
        Ok(contents) => contents,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(AgentsStore::default()),
        Err(err) => return Err(err),
    };

    let parsed = serde_json::from_str::<AgentsStore>(&raw).map_err(|err| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("failed to parse {}: {err}", path.display()),
        )
    })?;
    Ok(parsed.normalize())
}

pub async fn save_agents_store(codex_home: &Path, state: &AgentsStore) -> io::Result<()> {
    let path = agents_store_path(codex_home);
    fs::create_dir_all(codex_home).await?;
    let normalized = state.clone().normalize();
    let serialized = serde_json::to_string_pretty(&normalized).map_err(|err| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("failed to serialize agents store: {err}"),
        )
    })?;

    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, format!("{serialized}\n")).await?;
    if let Err(rename_err) = fs::rename(&tmp_path, &path).await {
        if rename_err.kind() == ErrorKind::AlreadyExists {
            let _ = fs::remove_file(&path).await;
            fs::rename(&tmp_path, &path).await?;
        } else {
            return Err(rename_err);
        }
    }
    Ok(())
}

fn normalize_agent(agent: AgentDefinition) -> Option<AgentDefinition> {
    let name = agent.name.trim().to_string();
    let mut id = agent.id.trim().to_string();
    if id.is_empty() {
        id = slugify(&name);
    }
    if id.is_empty() {
        return None;
    }

    let prompt = agent.prompt.trim().to_string();
    if prompt.is_empty() {
        return None;
    }

    let normalized_name = if name.is_empty() { id.clone() } else { name };
    let description = agent
        .description
        .map(|description| description.trim().to_string())
        .filter(|description| !description.is_empty());
    let model = agent
        .model
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty());

    Some(AgentDefinition {
        id,
        name: normalized_name,
        description,
        prompt,
        model,
        reasoning_effort: agent.reasoning_effort,
    })
}

fn slugify(input: &str) -> String {
    let mut output = String::new();
    let mut previous_was_dash = false;
    for ch in input.chars() {
        let normalized = ch.to_ascii_lowercase();
        if normalized.is_ascii_alphanumeric() {
            output.push(normalized);
            previous_was_dash = false;
        } else if !previous_was_dash {
            output.push('-');
            previous_was_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}

fn default_store_version() -> u32 {
    STORE_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[tokio::test]
    async fn missing_store_returns_default_builtins() {
        let temp = tempdir().expect("create tempdir");
        let state = load_agents_store(temp.path()).await.expect("load agents");
        let expected_ids: Vec<String> = built_in_agents()
            .into_iter()
            .map(|agent| agent.id)
            .collect();
        let actual_ids: Vec<String> = state.agents.into_iter().map(|agent| agent.id).collect();
        assert_eq!(actual_ids, expected_ids);
    }

    #[tokio::test]
    async fn save_and_load_round_trip_normalizes_data() {
        let temp = tempdir().expect("create tempdir");
        let state = AgentsStore {
            version: 9,
            active_agent_id: Some(" reviewer ".to_string()),
            agents: vec![
                AgentDefinition {
                    id: " reviewer ".to_string(),
                    name: " Reviewer ".to_string(),
                    description: Some(" ".to_string()),
                    prompt: " Keep an eye on regressions. ".to_string(),
                    model: Some(" ".to_string()),
                    reasoning_effort: Some(ReasoningEffort::Low),
                },
                AgentDefinition {
                    id: "reviewer".to_string(),
                    name: "duplicate".to_string(),
                    description: None,
                    prompt: "ignored".to_string(),
                    model: None,
                    reasoning_effort: None,
                },
            ],
            thread_bindings: BTreeMap::from([
                ("reviewer".to_string(), "  thread-123 ".to_string()),
                ("missing".to_string(), "thread-404".to_string()),
            ]),
        };
        save_agents_store(temp.path(), &state)
            .await
            .expect("save agents");

        let loaded = load_agents_store(temp.path()).await.expect("load agents");
        let expected = AgentsStore {
            version: STORE_VERSION,
            active_agent_id: Some("reviewer".to_string()),
            agents: vec![AgentDefinition {
                id: "reviewer".to_string(),
                name: "Reviewer".to_string(),
                description: None,
                prompt: "Keep an eye on regressions.".to_string(),
                model: None,
                reasoning_effort: Some(ReasoningEffort::Low),
            }],
            thread_bindings: BTreeMap::from([("reviewer".to_string(), "thread-123".to_string())]),
        };
        assert_eq!(loaded, expected);
    }

    #[test]
    fn normalize_clears_invalid_active_and_empty_prompts() {
        let state = AgentsStore {
            version: STORE_VERSION,
            active_agent_id: Some("missing".to_string()),
            agents: vec![
                AgentDefinition {
                    id: "alpha".to_string(),
                    name: "Alpha".to_string(),
                    description: None,
                    prompt: " ".to_string(),
                    model: None,
                    reasoning_effort: None,
                },
                AgentDefinition {
                    id: String::new(),
                    name: "Beta Agent".to_string(),
                    description: None,
                    prompt: "Keep it focused".to_string(),
                    model: None,
                    reasoning_effort: None,
                },
            ],
            thread_bindings: BTreeMap::new(),
        };

        let normalized = state.normalize();
        assert_eq!(normalized.active_agent_id, None);
        let ids: Vec<String> = normalized
            .agents
            .iter()
            .map(|agent| agent.id.clone())
            .collect();
        assert_eq!(ids, vec!["beta-agent".to_string()]);
    }

    #[test]
    fn find_agent_id_for_thread_returns_match() {
        let state = AgentsStore {
            version: STORE_VERSION,
            active_agent_id: None,
            agents: vec![AgentDefinition {
                id: "architect".to_string(),
                name: "Architect".to_string(),
                description: None,
                prompt: "Prompt".to_string(),
                model: None,
                reasoning_effort: None,
            }],
            thread_bindings: BTreeMap::from([("architect".to_string(), "thread-1".to_string())]),
        };
        assert_eq!(
            state.find_agent_id_for_thread("thread-1"),
            Some("architect")
        );
        assert_eq!(state.find_agent_id_for_thread("thread-2"), None);
    }
}
