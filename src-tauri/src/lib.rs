use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
enum AppError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DailyTask {
    id: i64,
    scenario_id: String,
    task_date: String,
    task_type: String,
    title: String,
    prompt: String,
    scenario_tag: String,
    status: String,
    estimated_minutes: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WeaknessTag {
    tag_name: String,
    count: i64,
    last_seen_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardSummary {
    total_study_minutes: i64,
    conversation_count: i64,
    writing_count: i64,
    average_writing_score: f64,
    average_speaking_score: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HomePayload {
    tasks: Vec<DailyTask>,
    weaknesses: Vec<WeaknessTag>,
    dashboard: DashboardSummary,
}

#[derive(Debug, Clone)]
struct ScenarioSeed {
    id: &'static str,
    task_type: &'static str,
    title: &'static str,
    prompt: &'static str,
    scenario_tag: &'static str,
    estimated_minutes: i64,
}

#[derive(Debug, Clone)]
struct WordSeed {
    key: &'static str,
    word: &'static str,
    meaning_ja: &'static str,
    example: &'static str,
    category: &'static str,
    office_priority: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WritingSubmission {
    task_id: i64,
    scenario_id: Option<String>,
    prompt: String,
    draft: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationSubmission {
    task_id: i64,
    scenario_id: Option<String>,
    prompt: String,
    response_text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WritingSessionRecord {
    id: i64,
    started_at: String,
    task_kind: String,
    source_prompt: String,
    user_draft: String,
    corrected_draft: String,
    shortened_draft: String,
    feedback_summary: String,
    score_clarity: i64,
    score_conciseness: i64,
    score_tone: i64,
    score_business: i64,
    score_grammar: i64,
    evaluator: String,
    warning_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationSessionRecord {
    id: i64,
    scenario_id: String,
    started_at: String,
    scenario_type: String,
    role_type: String,
    objective: String,
    transcript: String,
    improved_transcript: String,
    feedback_summary: String,
    priority_fix: String,
    retry_prompt: String,
    score_structure: i64,
    score_speed: i64,
    score_business: i64,
    score_paraphrase: i64,
    score_intelligibility: i64,
    evaluator: String,
    warning_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScenarioProgressRecord {
    scenario_id: String,
    task_type: String,
    title: String,
    attempts: i64,
    latest_score: f64,
    average_score: f64,
    first_attempt_at: String,
    last_attempt_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WordCardRecord {
    id: i64,
    key: String,
    word: String,
    meaning_ja: String,
    example: String,
    category: String,
    office_priority: i64,
    mastery_score: f64,
    pass_count: i64,
    fail_count: i64,
    streak: i64,
    last_result: String,
    next_due_at: Option<String>,
    is_mastered: bool,
    choices: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WordTrainingPayload {
    queue: Vec<WordCardRecord>,
    library: Vec<WordCardRecord>,
    active_count: i64,
    mastered_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct VocabNoteRecord {
    id: i64,
    expression: String,
    meaning_ja: String,
    note: String,
    example: String,
    review_count: i64,
    retention_score: f64,
    last_result: String,
    created_at: String,
    last_reviewed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppConfigResponse {
    provider: String,
    has_open_ai_api_key: bool,
    openai_model: String,
    openai_api_base: String,
    ollama_model: String,
    ollama_api_base: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppConfigPayload {
    #[serde(default = "default_provider")]
    provider: String,
    #[serde(default = "default_openai_model")]
    openai_model: String,
    #[serde(default = "default_openai_api_base")]
    openai_api_base: String,
    #[serde(default = "default_ollama_model")]
    ollama_model: String,
    #[serde(default = "default_ollama_api_base")]
    ollama_api_base: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddVocabNotePayload {
    expression: String,
    meaning_ja: String,
    note: String,
    example: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReviewVocabPayload {
    note_id: i64,
    outcome: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WordAttemptPayload {
    word_id: i64,
    result: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OpenAiWritingFeedback {
    corrected_draft: String,
    shortened_draft: String,
    feedback_summary: String,
    tone_label: String,
    weaknesses: Vec<String>,
    score_clarity: i64,
    score_conciseness: i64,
    score_tone: i64,
    score_business: i64,
    score_grammar: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct OllamaWritingFeedback {
    corrected_draft: String,
    shortened_draft: String,
    feedback_summary: String,
    tone_label: String,
    weaknesses: Vec<String>,
    score_clarity: i64,
    score_conciseness: i64,
    score_tone: i64,
    score_business: i64,
    score_grammar: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ConversationFeedback {
    improved_transcript: String,
    feedback_summary: String,
    priority_fix: String,
    retry_prompt: String,
    weaknesses: Vec<String>,
    score_structure: i64,
    score_speed: i64,
    score_business: i64,
    score_paraphrase: i64,
    score_intelligibility: i64,
}

#[derive(Debug, Deserialize)]
struct ResponsesApiOutputText {
    text: String,
}

#[derive(Debug, Deserialize)]
struct ResponsesApiMessage {
    content: Vec<ResponsesApiContent>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponsesApiContent {
    #[serde(rename = "output_text")]
    OutputText(ResponsesApiOutputText),
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponsesApiOutputItem {
    #[serde(rename = "message")]
    Message(ResponsesApiMessage),
    #[serde(other)]
    Other,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OllamaModelInfo {
    name: String,
    size_bytes: Option<i64>,
    modified_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTagModel>,
}

#[derive(Debug, Deserialize)]
struct OllamaTagModel {
    name: String,
    size: Option<i64>,
    modified_at: Option<String>,
}

#[derive(Default)]
struct AppState {
    db_path: Mutex<Option<PathBuf>>,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_openai_model() -> String {
    "gpt-5-mini".to_string()
}

fn default_openai_api_base() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_ollama_model() -> String {
    "mistral:latest".to_string()
}

fn default_ollama_api_base() -> String {
    "http://127.0.0.1:11434".to_string()
}

fn scenario_catalog() -> &'static [ScenarioSeed] {
    &[
        ScenarioSeed {
            id: "conv-standup-cache",
            task_type: "conversation",
            title: "Standup update: cache rollout",
            prompt: "You are giving a standup update. Cover progress on the cache rollout, one blocker, and the next action in 45 seconds.",
            scenario_tag: "meeting",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-design-review-api",
            task_type: "conversation",
            title: "Design review: API-first proposal",
            prompt: "You are in a design review. Recommend an API-first approach, explain one trade-off, and ask for alignment in 60 seconds.",
            scenario_tag: "meeting",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-incident-update",
            task_type: "conversation",
            title: "Incident update: staging issue",
            prompt: "You are updating your team about a staging incident. Explain the current status, likely cause, and next mitigation step in 60 seconds.",
            scenario_tag: "incident",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-standup-auth-delay",
            task_type: "conversation",
            title: "Standup update: auth delay",
            prompt: "Give a standup update about an authentication feature. Mention what finished yesterday, one blocker today, and your next action in 45 seconds.",
            scenario_tag: "meeting",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-priority-negotiation",
            task_type: "conversation",
            title: "Priority negotiation with PM",
            prompt: "Talk to a PM who wants two urgent tasks done this week. Explain your current workload, one risk, and propose a priority order in 60 seconds.",
            scenario_tag: "negotiation",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-scope-tradeoff",
            task_type: "conversation",
            title: "Scope trade-off discussion",
            prompt: "Explain why reducing scope is the safest option for this sprint. Mention delivery speed, one technical risk, and the next decision needed.",
            scenario_tag: "meeting",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-release-readiness",
            task_type: "conversation",
            title: "Release readiness update",
            prompt: "Update your team on whether a release is ready. State your recommendation, one blocker, and one action before release.",
            scenario_tag: "release",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-bug-root-cause",
            task_type: "conversation",
            title: "Bug root cause explanation",
            prompt: "Explain a recently found bug to your team. Cover the symptom, likely root cause, and how you will prevent recurrence.",
            scenario_tag: "incident",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-stakeholder-update",
            task_type: "conversation",
            title: "Stakeholder update: progress and risk",
            prompt: "Give a short stakeholder update on project progress. Include current status, one major risk, and the next milestone.",
            scenario_tag: "stakeholder",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-design-choice-db",
            task_type: "conversation",
            title: "Explain a database design choice",
            prompt: "Explain why you chose one database design over another. Mention the trade-off, impact on performance, and what you want feedback on.",
            scenario_tag: "design",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-estimate-delay",
            task_type: "conversation",
            title: "Explain an estimate delay",
            prompt: "Tell your manager why your estimate changed. Keep it concise, mention the new information, and propose a revised plan.",
            scenario_tag: "alignment",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-code-review-defense",
            task_type: "conversation",
            title: "Defend a code review decision",
            prompt: "Respond to a teammate who questions your code review comment. Explain your reasoning, acknowledge one trade-off, and move toward agreement.",
            scenario_tag: "review",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-handoff-update",
            task_type: "conversation",
            title: "Cross-team handoff update",
            prompt: "Give a handoff update to another team. State what is done, what they need from you, and what you need from them.",
            scenario_tag: "handoff",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-risk-escalation",
            task_type: "conversation",
            title: "Escalate a technical risk",
            prompt: "Escalate a technical risk to your lead. Explain the risk, expected impact, and the decision you need today.",
            scenario_tag: "risk",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-retrospective-point",
            task_type: "conversation",
            title: "Retrospective improvement point",
            prompt: "In a retrospective, explain one process problem, its impact, and one concrete improvement for next sprint.",
            scenario_tag: "retrospective",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-oncall-briefing",
            task_type: "conversation",
            title: "On-call briefing",
            prompt: "Brief the on-call engineer about an issue. Cover the current symptom, what was tried, and what to monitor next.",
            scenario_tag: "incident",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-requirement-challenge",
            task_type: "conversation",
            title: "Challenge an unclear requirement",
            prompt: "Push back on an unclear requirement in a meeting. Be polite, explain the ambiguity, and propose one assumption.",
            scenario_tag: "requirements",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-demo-summary",
            task_type: "conversation",
            title: "Demo summary after feature review",
            prompt: "Summarize a feature demo. State what worked, what still needs validation, and the next action.",
            scenario_tag: "demo",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "conv-interview-project",
            task_type: "conversation",
            title: "Interview-style project explanation",
            prompt: "Explain one recent project as if you were in an interview. Cover goal, your role, one challenge, and the outcome.",
            scenario_tag: "interview",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "conv-interview-failure",
            task_type: "conversation",
            title: "Interview-style failure and learning",
            prompt: "Explain a project failure in an interview-style answer. Cover what happened, what you learned, and what you changed afterward.",
            scenario_tag: "interview",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-followup-design-review",
            task_type: "writing",
            title: "Follow-up email after design review",
            prompt: "Write a short follow-up email after a design review. Confirm the decision, mention one risk, and ask for the next action owner.",
            scenario_tag: "email",
            estimated_minutes: 15,
        },
        ScenarioSeed {
            id: "write-delay-notice",
            task_type: "writing",
            title: "Delay notice with revised plan",
            prompt: "Write a brief email explaining a delivery delay, the main reason, and a revised next step without sounding defensive.",
            scenario_tag: "email",
            estimated_minutes: 15,
        },
        ScenarioSeed {
            id: "write-clarification-request",
            task_type: "writing",
            title: "Clarification request for requirements",
            prompt: "Write a concise email asking for clarification on an unclear requirement and propose one assumption if no reply arrives today.",
            scenario_tag: "email",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-standup-slack",
            task_type: "writing",
            title: "Slack standup update",
            prompt: "Write a short Slack update with today's progress, one blocker, and the next action. Keep it under 4 lines.",
            scenario_tag: "slack",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "write-incident-summary",
            task_type: "writing",
            title: "Incident summary for the team",
            prompt: "Write a short incident summary for your team. Include current impact, suspected cause, and next mitigation step.",
            scenario_tag: "incident",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-post-meeting-actions",
            task_type: "writing",
            title: "Post-meeting action summary",
            prompt: "Write a follow-up message after a meeting. List the decision, owner, deadline, and one unresolved question.",
            scenario_tag: "email",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-polite-pushback",
            task_type: "writing",
            title: "Polite pushback on a request",
            prompt: "Write a polite email pushing back on a request that would increase risk. Suggest a safer alternative.",
            scenario_tag: "email",
            estimated_minutes: 15,
        },
        ScenarioSeed {
            id: "write-request-log-review",
            task_type: "writing",
            title: "Request help with log review",
            prompt: "Write a concise request asking another engineer to help review logs. Mention urgency, current finding, and what you need.",
            scenario_tag: "email",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "write-customer-impact-note",
            task_type: "writing",
            title: "Customer impact note",
            prompt: "Write a short internal note describing customer impact from a bug and the current mitigation plan.",
            scenario_tag: "status",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-release-go-no-go",
            task_type: "writing",
            title: "Release go/no-go recommendation",
            prompt: "Write a brief recommendation on whether to release today. State your recommendation, one risk, and one required condition.",
            scenario_tag: "release",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-followup-owner",
            task_type: "writing",
            title: "Follow up on unclear ownership",
            prompt: "Write a short follow-up asking who owns the next action item after a meeting. Keep the tone neutral and practical.",
            scenario_tag: "email",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "write-weekly-status",
            task_type: "writing",
            title: "Weekly status update",
            prompt: "Write a weekly status update with three parts: progress, blockers, and next week's focus. Keep it concise.",
            scenario_tag: "status",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-risk-flag",
            task_type: "writing",
            title: "Raise a risk early",
            prompt: "Write a short message raising a delivery risk early. Explain the risk, likely impact, and one proposed response.",
            scenario_tag: "risk",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-test-results",
            task_type: "writing",
            title: "Share test results clearly",
            prompt: "Write a short update sharing test results. Mention what passed, what failed, and what still needs confirmation.",
            scenario_tag: "status",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "write-thank-you-summary",
            task_type: "writing",
            title: "Thank-you note with next steps",
            prompt: "Write a short thank-you message after a helpful review. Mention one useful point and confirm the next step.",
            scenario_tag: "email",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "write-interview-selfintro",
            task_type: "writing",
            title: "Interview self-introduction draft",
            prompt: "Draft a short self-introduction for an interview. Include current role, strengths, and one recent achievement.",
            scenario_tag: "interview",
            estimated_minutes: 15,
        },
        ScenarioSeed {
            id: "write-interview-star",
            task_type: "writing",
            title: "STAR answer draft",
            prompt: "Write a STAR-format answer about a time you solved a difficult technical problem. Keep it concise but complete.",
            scenario_tag: "interview",
            estimated_minutes: 15,
        },
        ScenarioSeed {
            id: "write-linkedin-summary",
            task_type: "writing",
            title: "LinkedIn summary rewrite",
            prompt: "Write a concise LinkedIn summary for a software engineer aiming to work in an international environment.",
            scenario_tag: "career",
            estimated_minutes: 15,
        },
        ScenarioSeed {
            id: "write-requirement-summary",
            task_type: "writing",
            title: "Requirement summary after a call",
            prompt: "Write a short requirement summary after a call. Capture the key requirement, one open question, and the next step.",
            scenario_tag: "requirements",
            estimated_minutes: 12,
        },
        ScenarioSeed {
            id: "write-pr-review-comment",
            task_type: "writing",
            title: "Pull request review comment",
            prompt: "Write a constructive pull request review comment. Explain the issue, the risk, and a suggested change briefly.",
            scenario_tag: "review",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-meeting-phrases-a",
            task_type: "srs",
            title: "Review 15 meeting phrases",
            prompt: "Review 15 meeting phrases for alignment, trade-offs, and next actions.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-email-phrases-a",
            task_type: "srs",
            title: "Review 15 email phrases",
            prompt: "Review 15 email phrases for requests, follow-up, and clarification.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-standup-phrases",
            task_type: "srs",
            title: "Review 15 standup phrases",
            prompt: "Review 15 standup phrases for progress, blockers, and next actions.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-incident-phrases",
            task_type: "srs",
            title: "Review 15 incident phrases",
            prompt: "Review 15 incident response phrases for impact, mitigation, and updates.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-interview-phrases",
            task_type: "srs",
            title: "Review 15 interview phrases",
            prompt: "Review 15 interview phrases for achievements, challenges, and learning.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-negotiation-phrases",
            task_type: "srs",
            title: "Review 15 negotiation phrases",
            prompt: "Review 15 negotiation phrases for prioritization, compromise, and timelines.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-design-review-phrases",
            task_type: "srs",
            title: "Review 15 design review phrases",
            prompt: "Review 15 design review phrases for trade-offs, concerns, and recommendations.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-polite-pushback",
            task_type: "srs",
            title: "Review 15 polite pushback phrases",
            prompt: "Review 15 polite pushback phrases for raising concerns without sounding negative.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
        ScenarioSeed {
            id: "srs-career-phrases",
            task_type: "srs",
            title: "Review 15 career phrases",
            prompt: "Review 15 career and interview phrases for self-introduction and achievements.",
            scenario_tag: "srs",
            estimated_minutes: 10,
        },
    ]
}

fn word_catalog() -> &'static [WordSeed] {
    &[
        WordSeed { key: "deadline", word: "deadline", meaning_ja: "締め切り", example: "We need to move the deadline by two days.", category: "office", office_priority: 5 },
        WordSeed { key: "schedule", word: "schedule", meaning_ja: "予定、日程", example: "The delivery schedule changed after the review.", category: "office", office_priority: 5 },
        WordSeed { key: "priority", word: "priority", meaning_ja: "優先事項", example: "This bug is our top priority today.", category: "office", office_priority: 5 },
        WordSeed { key: "proposal", word: "proposal", meaning_ja: "提案", example: "I shared a proposal for the API design.", category: "office", office_priority: 5 },
        WordSeed { key: "approve", word: "approve", meaning_ja: "承認する", example: "The manager approved the revised plan.", category: "office", office_priority: 5 },
        WordSeed { key: "confirm", word: "confirm", meaning_ja: "確認する", example: "Please confirm the owner of the next step.", category: "office", office_priority: 5 },
        WordSeed { key: "update", word: "update", meaning_ja: "更新、進捗報告", example: "I will send an update this afternoon.", category: "office", office_priority: 5 },
        WordSeed { key: "progress", word: "progress", meaning_ja: "進捗", example: "We made good progress on the rollout.", category: "office", office_priority: 5 },
        WordSeed { key: "blocker", word: "blocker", meaning_ja: "進行を止める問題", example: "My blocker is the staging database issue.", category: "office", office_priority: 5 },
        WordSeed { key: "owner", word: "owner", meaning_ja: "担当者", example: "We still need an owner for this task.", category: "office", office_priority: 5 },
        WordSeed { key: "outcome", word: "outcome", meaning_ja: "結果", example: "The outcome was better than expected.", category: "office", office_priority: 4 },
        WordSeed { key: "followup", word: "follow-up", meaning_ja: "フォローアップ", example: "I sent a follow-up email after the meeting.", category: "office", office_priority: 4 },
        WordSeed { key: "clarify", word: "clarify", meaning_ja: "明確にする", example: "Could you clarify the requirement?", category: "office", office_priority: 5 },
        WordSeed { key: "requirement", word: "requirement", meaning_ja: "要件", example: "The requirement is still too broad.", category: "office", office_priority: 5 },
        WordSeed { key: "decision", word: "decision", meaning_ja: "決定", example: "We need a decision by Friday.", category: "office", office_priority: 5 },
        WordSeed { key: "concern", word: "concern", meaning_ja: "懸念", example: "My main concern is operational risk.", category: "office", office_priority: 5 },
        WordSeed { key: "suggest", word: "suggest", meaning_ja: "提案する", example: "I suggest reducing the scope for now.", category: "office", office_priority: 5 },
        WordSeed { key: "delay", word: "delay", meaning_ja: "遅延", example: "The delay came from test failures.", category: "office", office_priority: 5 },
        WordSeed { key: "issue", word: "issue", meaning_ja: "問題", example: "We found an issue in the authentication flow.", category: "office", office_priority: 5 },
        WordSeed { key: "resolve", word: "resolve", meaning_ja: "解決する", example: "We resolved the issue before release.", category: "office", office_priority: 5 },
        WordSeed { key: "review", word: "review", meaning_ja: "レビュー", example: "The review raised a few useful points.", category: "office", office_priority: 5 },
        WordSeed { key: "feedback", word: "feedback", meaning_ja: "フィードバック", example: "Thanks for the clear feedback.", category: "office", office_priority: 5 },
        WordSeed { key: "alignment", word: "alignment", meaning_ja: "認識合わせ", example: "We need alignment before implementation starts.", category: "office", office_priority: 5 },
        WordSeed { key: "milestone", word: "milestone", meaning_ja: "マイルストーン", example: "The next milestone is the beta release.", category: "office", office_priority: 4 },
        WordSeed { key: "resource", word: "resource", meaning_ja: "リソース", example: "We do not have enough resources this sprint.", category: "office", office_priority: 4 },
        WordSeed { key: "budget", word: "budget", meaning_ja: "予算", example: "The budget does not cover an additional vendor.", category: "office", office_priority: 4 },
        WordSeed { key: "policy", word: "policy", meaning_ja: "方針、規程", example: "This change must follow company policy.", category: "office", office_priority: 3 },
        WordSeed { key: "request", word: "request", meaning_ja: "依頼", example: "I have one request before we proceed.", category: "office", office_priority: 5 },
        WordSeed { key: "respond", word: "respond", meaning_ja: "対応する、返答する", example: "We need to respond by tomorrow.", category: "office", office_priority: 4 },
        WordSeed { key: "assign", word: "assign", meaning_ja: "割り当てる", example: "Let's assign one engineer to the fix.", category: "office", office_priority: 4 },
        WordSeed { key: "deliver", word: "deliver", meaning_ja: "納品する、届ける", example: "Can we still deliver this by Monday?", category: "office", office_priority: 4 },
        WordSeed { key: "available", word: "available", meaning_ja: "利用可能な、空いている", example: "I am available after 3 p.m.", category: "office", office_priority: 4 },
        WordSeed { key: "achieve", word: "achieve", meaning_ja: "達成する", example: "We achieved the latency target.", category: "toeic", office_priority: 3 },
        WordSeed { key: "improve", word: "improve", meaning_ja: "改善する", example: "The new cache strategy improved response time.", category: "toeic", office_priority: 4 },
        WordSeed { key: "maintain", word: "maintain", meaning_ja: "維持する", example: "We need to maintain service quality.", category: "toeic", office_priority: 3 },
        WordSeed { key: "reduce", word: "reduce", meaning_ja: "減らす", example: "This option reduces operational risk.", category: "toeic", office_priority: 4 },
        WordSeed { key: "increase", word: "increase", meaning_ja: "増やす", example: "The new load increased server costs.", category: "toeic", office_priority: 3 },
        WordSeed { key: "impact", word: "impact", meaning_ja: "影響", example: "The customer impact is currently limited.", category: "toeic", office_priority: 5 },
        WordSeed { key: "recommend", word: "recommend", meaning_ja: "推奨する", example: "I recommend delaying the release by one day.", category: "office", office_priority: 5 },
        WordSeed { key: "consider", word: "consider", meaning_ja: "検討する", example: "We should consider a rollback plan.", category: "toeic", office_priority: 4 },
        WordSeed { key: "estimate", word: "estimate", meaning_ja: "見積もる", example: "My estimate changed after testing.", category: "office", office_priority: 5 },
        WordSeed { key: "launch", word: "launch", meaning_ja: "開始する、公開する", example: "We plan to launch next month.", category: "toeic", office_priority: 3 },
        WordSeed { key: "revenue", word: "revenue", meaning_ja: "収益", example: "The feature could improve revenue next quarter.", category: "toeic", office_priority: 2 },
        WordSeed { key: "contract", word: "contract", meaning_ja: "契約", example: "The contract review is still pending.", category: "toeic", office_priority: 2 },
        WordSeed { key: "invoice", word: "invoice", meaning_ja: "請求書", example: "The invoice was sent this morning.", category: "toeic", office_priority: 2 },
        WordSeed { key: "purchase", word: "purchase", meaning_ja: "購入する", example: "We need approval before purchase.", category: "toeic", office_priority: 2 },
        WordSeed { key: "shipment", word: "shipment", meaning_ja: "出荷", example: "The shipment was delayed by weather.", category: "toeic", office_priority: 1 },
        WordSeed { key: "inventory", word: "inventory", meaning_ja: "在庫", example: "The inventory data is out of date.", category: "toeic", office_priority: 1 },
        WordSeed { key: "meeting", word: "meeting", meaning_ja: "会議", example: "The meeting starts at 10 a.m.", category: "office", office_priority: 5 },
        WordSeed { key: "agenda", word: "agenda", meaning_ja: "議題", example: "Please check the agenda before the call.", category: "office", office_priority: 4 },
        WordSeed { key: "attend", word: "attend", meaning_ja: "出席する", example: "Can you attend the review tomorrow?", category: "toeic", office_priority: 3 },
        WordSeed { key: "summary", word: "summary", meaning_ja: "要約", example: "I posted a summary after the meeting.", category: "office", office_priority: 5 },
        WordSeed { key: "brief", word: "brief", meaning_ja: "簡潔な", example: "Please keep the update brief.", category: "office", office_priority: 4 },
        WordSeed { key: "concise", word: "concise", meaning_ja: "簡潔な", example: "Your email was clear and concise.", category: "office", office_priority: 4 },
        WordSeed { key: "effective", word: "effective", meaning_ja: "効果的な", example: "We need an effective mitigation plan.", category: "toeic", office_priority: 3 },
        WordSeed { key: "efficient", word: "efficient", meaning_ja: "効率的な", example: "This is a more efficient process.", category: "toeic", office_priority: 3 },
        WordSeed { key: "vendor", word: "vendor", meaning_ja: "取引先業者", example: "The vendor asked for more details.", category: "toeic", office_priority: 2 },
        WordSeed { key: "client", word: "client", meaning_ja: "顧客", example: "The client needs an update today.", category: "office", office_priority: 3 },
        WordSeed { key: "stakeholder", word: "stakeholder", meaning_ja: "関係者", example: "We shared the risk with stakeholders.", category: "office", office_priority: 4 },
        WordSeed { key: "internal", word: "internal", meaning_ja: "社内の", example: "This is for internal use only.", category: "office", office_priority: 3 },
        WordSeed { key: "external", word: "external", meaning_ja: "社外の", example: "External communication will go out tomorrow.", category: "office", office_priority: 3 },
        WordSeed { key: "document", word: "document", meaning_ja: "文書、記録する", example: "Please document the decision.", category: "office", office_priority: 4 },
        WordSeed { key: "analysis", word: "analysis", meaning_ja: "分析", example: "We need more analysis before release.", category: "toeic", office_priority: 3 },
        WordSeed { key: "strategy", word: "strategy", meaning_ja: "戦略", example: "Our rollout strategy has changed.", category: "office", office_priority: 4 },
        WordSeed { key: "launchplan", word: "rollout", meaning_ja: "段階的展開", example: "The rollout will start with internal users.", category: "office", office_priority: 5 },
        WordSeed { key: "mitigation", word: "mitigation", meaning_ja: "緩和策", example: "We need a mitigation for the current risk.", category: "office", office_priority: 5 },
        WordSeed { key: "dependency", word: "dependency", meaning_ja: "依存関係", example: "The release depends on an external dependency.", category: "office", office_priority: 4 },
        WordSeed { key: "rollback", word: "rollback", meaning_ja: "ロールバック", example: "We prepared a rollback plan just in case.", category: "office", office_priority: 5 },
        WordSeed { key: "incident", word: "incident", meaning_ja: "障害、インシデント", example: "The incident affected only staging users.", category: "office", office_priority: 5 },
        WordSeed { key: "recovery", word: "recovery", meaning_ja: "復旧", example: "Recovery took about twenty minutes.", category: "office", office_priority: 4 },
    ]
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn local_data_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("naIEng")
}

fn preferred_db_path() -> PathBuf {
    exe_dir().join("naIEng.sqlite3")
}

fn fallback_db_path() -> PathBuf {
    local_data_root().join("naIEng.sqlite3")
}

fn primary_app_config_path() -> PathBuf {
    exe_dir().join("naIEng.config.json")
}

fn fallback_app_config_path() -> PathBuf {
    local_data_root().join("naIEng.config.json")
}

fn load_app_config_internal() -> Result<AppConfigPayload, AppError> {
    let env_model = std::env::var("OPENAI_MODEL").unwrap_or_default();
    let env_base = std::env::var("OPENAI_API_BASE").unwrap_or_default();

    let path = if primary_app_config_path().exists() {
        primary_app_config_path()
    } else if fallback_app_config_path().exists() {
        fallback_app_config_path()
    } else {
        primary_app_config_path()
    };

    let mut payload = if path.exists() {
        let text = fs::read_to_string(path)?;
        serde_json::from_str::<AppConfigPayload>(&text)?
    } else {
        AppConfigPayload {
            provider: default_provider(),
            openai_model: default_openai_model(),
            openai_api_base: default_openai_api_base(),
            ollama_model: default_ollama_model(),
            ollama_api_base: default_ollama_api_base(),
        }
    };

    if !env_model.trim().is_empty() {
        payload.openai_model = env_model;
    }
    if !env_base.trim().is_empty() {
        payload.openai_api_base = env_base;
    }

    if payload.provider.trim().is_empty() {
        payload.provider = "openai".to_string();
    }

    if payload.openai_model.trim().is_empty() {
        payload.openai_model = "gpt-5-mini".to_string();
    }
    if payload.openai_api_base.trim().is_empty() {
        payload.openai_api_base = "https://api.openai.com/v1".to_string();
    }
    if payload.ollama_model.trim().is_empty() {
        payload.ollama_model = "mistral:latest".to_string();
    }
    if payload.ollama_api_base.trim().is_empty() {
        payload.ollama_api_base = "http://127.0.0.1:11434".to_string();
    }

    Ok(payload)
}

fn save_app_config_internal(payload: AppConfigPayload) -> Result<AppConfigResponse, AppError> {
    let json = serde_json::to_string_pretty(&payload)?;

    let write_to = |path: &Path| -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &json)?;
        Ok(())
    };

    match write_to(&primary_app_config_path()) {
        Ok(()) => Ok(app_config_response(&payload)),
        Err(_) => {
            write_to(&fallback_app_config_path())?;
            Ok(app_config_response(&payload))
        }
    }
}

fn app_config_response(payload: &AppConfigPayload) -> AppConfigResponse {
    AppConfigResponse {
        provider: payload.provider.clone(),
        has_open_ai_api_key: std::env::var("OPENAI_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
        openai_model: payload.openai_model.clone(),
        openai_api_base: payload.openai_api_base.clone(),
        ollama_model: payload.ollama_model.clone(),
        ollama_api_base: payload.ollama_api_base.clone(),
    }
}

fn open_database(state: &AppState) -> Result<Connection, AppError> {
    if let Ok(guard) = state.db_path.lock() {
        if let Some(existing) = guard.as_ref() {
            return open_database_at(existing.clone());
        }
    }

    let primary = preferred_db_path();
    match open_database_at(primary.clone()) {
        Ok(conn) => {
            if let Ok(mut guard) = state.db_path.lock() {
                *guard = Some(primary);
            }
            Ok(conn)
        }
        Err(_) => {
            let fallback = fallback_db_path();
            let conn = open_database_at(fallback.clone())?;
            if let Ok(mut guard) = state.db_path.lock() {
                *guard = Some(fallback);
            }
            Ok(conn)
        }
    }
}

fn open_database_at(path: PathBuf) -> Result<Connection, AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    initialize_db(&conn)?;
    Ok(conn)
}

fn initialize_db(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS daily_task (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scenario_id TEXT NOT NULL DEFAULT '',
            task_date TEXT NOT NULL,
            task_type TEXT NOT NULL,
            title TEXT NOT NULL,
            prompt TEXT NOT NULL DEFAULT '',
            scenario_tag TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            estimated_minutes INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS scenario_catalog (
            id TEXT PRIMARY KEY,
            task_type TEXT NOT NULL,
            title TEXT NOT NULL,
            prompt TEXT NOT NULL,
            scenario_tag TEXT NOT NULL,
            estimated_minutes INTEGER NOT NULL,
            active INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS daily_refresh_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            task_date TEXT NOT NULL,
            task_type TEXT NOT NULL,
            scenario_id TEXT NOT NULL,
            shown_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS writing_session (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scenario_id TEXT NOT NULL DEFAULT '',
            started_at TEXT NOT NULL,
            task_kind TEXT NOT NULL,
            source_prompt TEXT NOT NULL,
            user_draft TEXT NOT NULL,
            corrected_draft TEXT NOT NULL,
            shortened_draft TEXT NOT NULL,
            feedback_summary TEXT NOT NULL,
            score_clarity INTEGER NOT NULL,
            score_conciseness INTEGER NOT NULL,
            score_tone INTEGER NOT NULL,
            score_business INTEGER NOT NULL,
            score_grammar INTEGER NOT NULL,
            evaluator TEXT NOT NULL DEFAULT 'local-fallback',
            warning_message TEXT
        );

        CREATE TABLE IF NOT EXISTS weakness_tag_event (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL,
            source_type TEXT NOT NULL,
            source_id INTEGER NOT NULL,
            tag_name TEXT NOT NULL,
            severity INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS conversation_session (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scenario_id TEXT NOT NULL DEFAULT '',
            started_at TEXT NOT NULL,
            scenario_type TEXT NOT NULL,
            role_type TEXT NOT NULL,
            objective TEXT NOT NULL,
            transcript TEXT NOT NULL,
            improved_transcript TEXT NOT NULL DEFAULT '',
            feedback_summary TEXT NOT NULL,
            priority_fix TEXT NOT NULL,
            retry_prompt TEXT NOT NULL,
            score_structure INTEGER NOT NULL,
            score_speed INTEGER NOT NULL,
            score_business INTEGER NOT NULL,
            score_paraphrase INTEGER NOT NULL,
            score_intelligibility INTEGER NOT NULL,
            evaluator TEXT NOT NULL DEFAULT 'local-fallback',
            warning_message TEXT
        );

        CREATE TABLE IF NOT EXISTS vocab_note (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            expression TEXT NOT NULL,
            meaning_ja TEXT NOT NULL,
            note TEXT NOT NULL DEFAULT '',
            example TEXT NOT NULL DEFAULT '',
            review_count INTEGER NOT NULL DEFAULT 0,
            retention_score REAL NOT NULL DEFAULT 0.35,
            last_result TEXT NOT NULL DEFAULT 'new',
            created_at TEXT NOT NULL,
            last_reviewed_at TEXT
        );

        CREATE TABLE IF NOT EXISTS word_card_master (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_key TEXT NOT NULL UNIQUE,
            word TEXT NOT NULL,
            meaning_ja TEXT NOT NULL,
            example TEXT NOT NULL,
            category TEXT NOT NULL,
            office_priority INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS word_card_progress (
            word_id INTEGER PRIMARY KEY,
            mastery_score REAL NOT NULL DEFAULT 0.12,
            pass_count INTEGER NOT NULL DEFAULT 0,
            fail_count INTEGER NOT NULL DEFAULT 0,
            streak INTEGER NOT NULL DEFAULT 0,
            last_result TEXT NOT NULL DEFAULT 'new',
            last_seen_at TEXT,
            next_due_at TEXT,
            is_mastered INTEGER NOT NULL DEFAULT 0
        );
        ",
    )?;

    ensure_column(
        conn,
        "daily_task",
        "scenario_id",
        "ALTER TABLE daily_task ADD COLUMN scenario_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "daily_task",
        "prompt",
        "ALTER TABLE daily_task ADD COLUMN prompt TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "writing_session",
        "scenario_id",
        "ALTER TABLE writing_session ADD COLUMN scenario_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "conversation_session",
        "scenario_id",
        "ALTER TABLE conversation_session ADD COLUMN scenario_id TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "writing_session",
        "evaluator",
        "ALTER TABLE writing_session ADD COLUMN evaluator TEXT NOT NULL DEFAULT 'local-fallback'",
    )?;
    ensure_column(
        conn,
        "writing_session",
        "warning_message",
        "ALTER TABLE writing_session ADD COLUMN warning_message TEXT",
    )?;
    ensure_column(
        conn,
        "conversation_session",
        "improved_transcript",
        "ALTER TABLE conversation_session ADD COLUMN improved_transcript TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_column(
        conn,
        "conversation_session",
        "evaluator",
        "ALTER TABLE conversation_session ADD COLUMN evaluator TEXT NOT NULL DEFAULT 'local-fallback'",
    )?;
    ensure_column(
        conn,
        "conversation_session",
        "warning_message",
        "ALTER TABLE conversation_session ADD COLUMN warning_message TEXT",
    )?;
    ensure_column(
        conn,
        "vocab_note",
        "retention_score",
        "ALTER TABLE vocab_note ADD COLUMN retention_score REAL NOT NULL DEFAULT 0.35",
    )?;
    ensure_column(
        conn,
        "vocab_note",
        "last_result",
        "ALTER TABLE vocab_note ADD COLUMN last_result TEXT NOT NULL DEFAULT 'new'",
    )?;

    seed_scenario_catalog(conn)?;
    seed_word_catalog(conn)?;
    seed_default_vocab_notes(conn)?;
    backfill_daily_tasks(conn)?;
    seed_today_if_needed(conn)?;
    backfill_daily_refresh_history(conn)?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
    alter_sql: &str,
) -> Result<(), AppError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !columns.iter().any(|existing| existing == column_name) {
        conn.execute(alter_sql, [])?;
    }
    Ok(())
}

fn seed_scenario_catalog(conn: &Connection) -> Result<(), AppError> {
    for scenario in scenario_catalog() {
        conn.execute(
            "INSERT INTO scenario_catalog (id, task_type, title, prompt, scenario_tag, estimated_minutes, active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)
             ON CONFLICT(id) DO UPDATE SET
                task_type = excluded.task_type,
                title = excluded.title,
                prompt = excluded.prompt,
                scenario_tag = excluded.scenario_tag,
                estimated_minutes = excluded.estimated_minutes,
                active = 1",
            params![
                scenario.id,
                scenario.task_type,
                scenario.title,
                scenario.prompt,
                scenario.scenario_tag,
                scenario.estimated_minutes
            ],
        )?;
    }
    Ok(())
}

fn backfill_daily_tasks(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_type, title, scenario_id, prompt
         FROM daily_task
         WHERE scenario_id = '' OR prompt = ''",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (id, task_type, title, scenario_id, prompt) in rows {
        let matched = scenario_catalog().iter().find(|scenario| {
            scenario.task_type == task_type
                && (scenario.title == title || (!scenario_id.is_empty() && scenario.id == scenario_id))
        });
        let fallback = scenario_catalog()
            .iter()
            .find(|scenario| scenario.task_type == task_type);

        if let Some(scenario) = matched.or(fallback) {
            let next_scenario_id = if scenario_id.is_empty() {
                scenario.id.to_string()
            } else {
                scenario_id.clone()
            };
            let next_prompt = if prompt.is_empty() {
                scenario.prompt.to_string()
            } else {
                prompt.clone()
            };
            conn.execute(
                "UPDATE daily_task SET scenario_id = ?1, prompt = ?2 WHERE id = ?3",
                params![next_scenario_id, next_prompt, id],
            )?;
        }
    }

    Ok(())
}

fn pick_daily_scenarios(task_type: &str, day_index: usize) -> Vec<&'static ScenarioSeed> {
    let matching: Vec<&ScenarioSeed> = scenario_catalog()
        .iter()
        .filter(|scenario| scenario.task_type == task_type)
        .collect();
    if matching.is_empty() {
        return Vec::new();
    }
    vec![matching[day_index % matching.len()]]
}

fn seed_vocab_note_if_missing(
    conn: &Connection,
    expression: &str,
    meaning_ja: &str,
    note: &str,
    example: &str,
) -> Result<(), AppError> {
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM vocab_note WHERE lower(expression) = lower(?1)",
        params![expression],
        |row| row.get(0),
    )?;

    if exists == 0 {
        conn.execute(
            "INSERT INTO vocab_note (expression, meaning_ja, note, example, review_count, retention_score, last_result, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, 0.35, 'new', ?5)",
            params![expression, meaning_ja, note, example, Utc::now().to_rfc3339()],
        )?;
    }

    Ok(())
}

fn seed_default_vocab_notes(conn: &Connection) -> Result<(), AppError> {
    seed_vocab_note_if_missing(
        conn,
        "one blocker",
        "今つまずいている課題を一つ",
        "スタンドアップで progress / one blocker / next action の型でよく使う表現です。",
        "My one blocker is that the staging result is still inconsistent.",
    )?;
    seed_vocab_note_if_missing(
        conn,
        "next action",
        "次に取る行動",
        "会議や報告の最後を締める定番表現です。",
        "The next action is to verify the logs and share the fix by 3 p.m.",
    )?;
    seed_vocab_note_if_missing(
        conn,
        "trade-off",
        "トレードオフ、何かを得る代わりに別のコストがある関係",
        "設計レビューや提案で非常によく使います。",
        "The trade-off is faster delivery but higher operational risk.",
    )?;
    Ok(())
}

fn seed_word_catalog(conn: &Connection) -> Result<(), AppError> {
    for card in word_catalog() {
        conn.execute(
            "INSERT INTO word_card_master (card_key, word, meaning_ja, example, category, office_priority)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(card_key) DO UPDATE SET
                word = excluded.word,
                meaning_ja = excluded.meaning_ja,
                example = excluded.example,
                category = excluded.category,
                office_priority = excluded.office_priority",
            params![
                card.key,
                card.word,
                card.meaning_ja,
                card.example,
                card.category,
                card.office_priority
            ],
        )?;
    }

    conn.execute(
        "INSERT INTO word_card_progress (word_id)
         SELECT id
         FROM word_card_master
         WHERE id NOT IN (SELECT word_id FROM word_card_progress)",
        [],
    )?;

    Ok(())
}

fn build_word_choices(word_id: i64, correct_word: &str) -> Vec<String> {
    let all_words: Vec<&str> = word_catalog().iter().map(|item| item.word).collect();
    let mut distractors = Vec::new();
    let start = (word_id as usize) % all_words.len();
    for offset in 0..all_words.len() {
        let candidate = all_words[(start + offset) % all_words.len()];
        if candidate != correct_word && !distractors.iter().any(|item| item == candidate) {
            distractors.push(candidate.to_string());
        }
        if distractors.len() == 3 {
            break;
        }
    }

    let mut choices = vec![correct_word.to_string()];
    choices.extend(distractors);
    let shift = (word_id as usize) % choices.len();
    choices.rotate_left(shift);
    choices
}

fn map_word_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WordCardRecord> {
    let id: i64 = row.get(0)?;
    let word: String = row.get(2)?;
    Ok(WordCardRecord {
        id,
        key: row.get(1)?,
        word: word.clone(),
        meaning_ja: row.get(3)?,
        example: row.get(4)?,
        category: row.get(5)?,
        office_priority: row.get(6)?,
        mastery_score: row.get(7)?,
        pass_count: row.get(8)?,
        fail_count: row.get(9)?,
        streak: row.get(10)?,
        last_result: row.get(11)?,
        next_due_at: row.get(12)?,
        is_mastered: row.get::<_, i64>(13)? == 1,
        choices: build_word_choices(id, &word),
    })
}

fn mastery_after_attempt(current: f64, result: &str, streak: i64) -> f64 {
    let next = match result {
        "pass" => current + 0.14 + (streak as f64 * 0.02).min(0.08),
        "fail" => current * 0.72,
        "timeout" => current * 0.58,
        _ => current,
    };
    next.clamp(0.0, 1.0)
}

fn next_due_after_attempt(now: chrono::DateTime<Utc>, mastery: f64, result: &str) -> String {
    let next = match result {
        "pass" if mastery >= 0.92 => now + ChronoDuration::days(3650),
        "pass" if mastery >= 0.78 => now + ChronoDuration::days(7),
        "pass" if mastery >= 0.62 => now + ChronoDuration::days(3),
        "pass" if mastery >= 0.45 => now + ChronoDuration::hours(18),
        "pass" if mastery >= 0.28 => now + ChronoDuration::hours(4),
        "pass" => now + ChronoDuration::minutes(25),
        "fail" => now + ChronoDuration::minutes(8),
        "timeout" => now + ChronoDuration::minutes(4),
        _ => now + ChronoDuration::minutes(15),
    };
    next.to_rfc3339()
}

fn seed_today_if_needed(conn: &Connection) -> Result<(), AppError> {
    let today = Utc::now().date_naive().to_string();
    let existing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM daily_task WHERE task_date = ?1",
        params![today],
        |row| row.get(0),
    )?;

    if existing > 0 {
        return Ok(());
    }

    let epoch_days = (Utc::now().timestamp() / 86_400).max(0) as usize;
    for task_type in ["conversation", "writing", "srs"] {
        for scenario in pick_daily_scenarios(task_type, epoch_days) {
            conn.execute(
                "INSERT INTO daily_task (scenario_id, task_date, task_type, title, prompt, scenario_tag, estimated_minutes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    scenario.id,
                    today,
                    scenario.task_type,
                    scenario.title,
                    scenario.prompt,
                    scenario.scenario_tag,
                    scenario.estimated_minutes
                ],
            )?;
            conn.execute(
                "INSERT INTO daily_refresh_history (task_date, task_type, scenario_id, shown_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![today, scenario.task_type, scenario.id, Utc::now().to_rfc3339()],
            )?;
        }
    }

    Ok(())
}

fn backfill_daily_refresh_history(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "SELECT task_date, task_type, scenario_id
         FROM daily_task
         WHERE scenario_id <> ''",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (task_date, task_type, scenario_id) in rows {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*)
             FROM daily_refresh_history
             WHERE task_date = ?1 AND task_type = ?2 AND scenario_id = ?3",
            params![task_date, task_type, scenario_id],
            |row| row.get(0),
        )?;

        if exists == 0 {
            conn.execute(
                "INSERT INTO daily_refresh_history (task_date, task_type, scenario_id, shown_at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![task_date, task_type, scenario_id, Utc::now().to_rfc3339()],
            )?;
        }
    }

    Ok(())
}

fn choose_next_scenario_for_refresh(
    conn: &Connection,
    task_type: &str,
    today: &str,
    current_scenario_id: &str,
) -> Result<&'static ScenarioSeed, AppError> {
    let matching: Vec<&ScenarioSeed> = scenario_catalog()
        .iter()
        .filter(|scenario| scenario.task_type == task_type)
        .collect();

    if matching.is_empty() {
        return Err(AppError::Message(format!(
            "No scenarios available for task type: {task_type}"
        )));
    }

    let mut stmt = conn.prepare(
        "SELECT scenario_id
         FROM daily_refresh_history
         WHERE task_date = ?1 AND task_type = ?2",
    )?;
    let seen_ids = stmt
        .query_map(params![today, task_type], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(unseen) = matching
        .iter()
        .copied()
        .find(|scenario| {
            scenario.id != current_scenario_id
                && !seen_ids.iter().any(|seen| seen == scenario.id)
        })
    {
        return Ok(unseen);
    }

    if let Some(not_current) = matching
        .iter()
        .copied()
        .find(|scenario| scenario.id != current_scenario_id)
    {
        return Ok(not_current);
    }

    Ok(matching[0])
}

fn evaluate_writing(draft: &str) -> (String, String, String, Vec<&'static str>, [i64; 5]) {
    let trimmed = draft.trim();
    let corrected = if trimmed.is_empty() {
        "Could we align on the next steps for the cache rollout by Friday?".to_string()
    } else {
        trimmed
            .replace(" i ", " I ")
            .replace("i'm", "I'm")
            .replace("dont", "don't")
            .replace("cant", "can't")
    };

    let shortened = if corrected.len() > 180 {
        format!("{}...", corrected.chars().take(177).collect::<String>().trim_end())
    } else {
        corrected.clone()
    };

    let summary =
        "Lead with the request, keep the business reason short, and end with a clear owner or next step."
            .to_string();

    let lowered = corrected.to_lowercase();
    let mut weaknesses = Vec::new();
    if corrected.len() > 220 {
        weaknesses.push("conciseness");
    }
    if !lowered.contains("could") && !lowered.contains("please") && !lowered.contains("would") {
        weaknesses.push("request-clarity");
    }
    if !lowered.contains("next") && !lowered.contains("by") {
        weaknesses.push("action-clarity");
    }
    if weaknesses.is_empty() {
        weaknesses.push("business-tone");
    }

    let scores = [
        if corrected.len() >= 40 { 4 } else { 3 },
        if corrected.len() <= 220 { 4 } else { 2 },
        4,
        if lowered.contains("next") || lowered.contains("decision") { 4 } else { 3 },
        4,
    ];

    (corrected, shortened, summary, weaknesses, scores)
}

fn evaluate_conversation_locally(response_text: &str) -> (ConversationFeedback, Vec<&'static str>) {
    let lowered = response_text.to_lowercase();
    let mut weaknesses = Vec::new();
    if !lowered.contains("next") && !lowered.contains("will") {
        weaknesses.push("next-step-clarity");
    }
    if response_text.len() > 320 {
        weaknesses.push("verbosity");
    }
    if !lowered.contains("blocker") && !lowered.contains("risk") {
        weaknesses.push("risk-clarity");
    }
    if weaknesses.is_empty() {
        weaknesses.push("conclusion-first");
    }

    (
        ConversationFeedback {
            improved_transcript:
                "Today's status is that the rollout is almost ready. My blocker is one inconsistent result in staging. Next, I will verify the logs, fix the issue, and share an update this afternoon."
                    .to_string(),
            feedback_summary:
                "Open with your status, state one blocker clearly, and end with the next action."
                    .to_string(),
            priority_fix: "Start with the conclusion before details.".to_string(),
            retry_prompt:
                "Try again in 45 seconds starting with: Today's status is...".to_string(),
            weaknesses: weaknesses.iter().map(|item| item.to_string()).collect(),
            score_structure: if response_text.len() > 40 { 4 } else { 3 },
            score_speed: 3,
            score_business: 4,
            score_paraphrase: 3,
            score_intelligibility: 3,
        },
        weaknesses,
    )
}

fn normalize_short_text(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() < 8 || trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_conversation_feedback(mut feedback: ConversationFeedback) -> ConversationFeedback {
    feedback.improved_transcript = normalize_short_text(
        &feedback.improved_transcript,
        "Today's status is on track. My blocker is one inconsistent staging result. Next, I will verify the logs and share an update this afternoon.",
    );
    feedback.feedback_summary = normalize_short_text(
        &feedback.feedback_summary,
        "Open with your status, mention one blocker, and end with the next action.",
    );
    feedback.priority_fix = normalize_short_text(
        &feedback.priority_fix,
        "Start with the conclusion before details.",
    );
    feedback.retry_prompt = normalize_short_text(
        &feedback.retry_prompt,
        "Try again in 45 seconds starting with: Today's status is...",
    );
    if feedback.weaknesses.is_empty() {
        feedback.weaknesses = vec!["conclusion-first".to_string()];
    }
    feedback
}

fn call_openai_conversation_feedback(
    config: &AppConfigPayload,
    prompt: &str,
    response_text: &str,
) -> Result<ConversationFeedback, AppError> {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.trim().is_empty() {
        return Err(AppError::Message(
            "OpenAI API key is not configured. Set OPENAI_API_KEY in the environment."
                .to_string(),
        ));
    }

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "improved_transcript": { "type": "string" },
            "feedback_summary": { "type": "string" },
            "priority_fix": { "type": "string" },
            "retry_prompt": { "type": "string" },
            "weaknesses": { "type": "array", "items": { "type": "string" } },
            "score_structure": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_speed": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_business": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_paraphrase": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_intelligibility": { "type": "integer", "minimum": 0, "maximum": 5 }
        },
        "required": [
            "improved_transcript",
            "feedback_summary",
            "priority_fix",
            "retry_prompt",
            "weaknesses",
            "score_structure",
            "score_speed",
            "score_business",
            "score_paraphrase",
            "score_intelligibility"
        ],
        "additionalProperties": false
    });

    let body = serde_json::json!({
        "model": config.openai_model,
        "reasoning": { "effort": "low" },
        "instructions": "You are an expert business English speaking coach for a Japanese software engineer. Evaluate short spoken business responses. Focus on structure, response speed, business appropriateness, paraphrasing, and intelligibility. Also provide one improved response that the learner could actually say in the same situation. Return valid JSON only.",
        "input": format!("Scenario:\n{prompt}\n\nUser response transcript:\n{response_text}"),
        "text": {
            "format": {
                "type": "json_schema",
                "name": "conversation_feedback",
                "schema": schema,
                "strict": true
            }
        }
    });

    let endpoint = format!("{}/responses", config.openai_api_base.trim_end_matches('/'));
    let client = Client::builder().timeout(Duration::from_secs(45)).build()?;
    let response = client
        .post(endpoint)
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(AppError::Message(format!(
            "OpenAI request failed with status {status}: {body}"
        )));
    }

    let value: serde_json::Value = response.json()?;
    let output = value
        .get("output")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Message("OpenAI response did not contain output.".to_string()))?;

    let mut text_chunks = Vec::new();
    for item in output {
        let parsed: ResponsesApiOutputItem = serde_json::from_value(item.clone())?;
        if let ResponsesApiOutputItem::Message(message) = parsed {
            for content in message.content {
                if let ResponsesApiContent::OutputText(text) = content {
                    text_chunks.push(text.text);
                }
            }
        }
    }

    let joined = text_chunks.join("\n").trim().to_string();
    if joined.is_empty() {
        return Err(AppError::Message(
            "OpenAI response did not contain output text.".to_string(),
        ));
    }

    Ok(normalize_conversation_feedback(
        serde_json::from_str::<ConversationFeedback>(&joined)?,
    ))
}

fn call_ollama_conversation_feedback(
    config: &AppConfigPayload,
    prompt: &str,
    response_text: &str,
) -> Result<ConversationFeedback, AppError> {
    let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
    let endpoint = format!("{}/api/generate", config.ollama_api_base.trim_end_matches('/'));
    let prompt_text = format!(
        "You are an expert business English speaking coach for a Japanese software engineer.\n\
Return JSON only with these keys: improved_transcript, feedback_summary, priority_fix, retry_prompt, weaknesses, score_structure, score_speed, score_business, score_paraphrase, score_intelligibility.\n\
Scores must be integers from 0 to 5.\n\
Scenario:\n{prompt}\n\nUser response transcript:\n{response_text}"
    );
    let body = serde_json::json!({
        "model": config.ollama_model,
        "prompt": prompt_text,
        "stream": false,
        "format": {
            "type": "object",
            "properties": {
                "improved_transcript": { "type": "string" },
                "feedback_summary": { "type": "string" },
                "priority_fix": { "type": "string" },
                "retry_prompt": { "type": "string" },
                "weaknesses": { "type": "array", "items": { "type": "string" } },
                "score_structure": { "type": "integer" },
                "score_speed": { "type": "integer" },
                "score_business": { "type": "integer" },
                "score_paraphrase": { "type": "integer" },
                "score_intelligibility": { "type": "integer" }
            },
            "required": [
                "improved_transcript",
                "feedback_summary",
                "priority_fix",
                "retry_prompt",
                "weaknesses",
                "score_structure",
                "score_speed",
                "score_business",
                "score_paraphrase",
                "score_intelligibility"
            ]
        }
    });

    let response = client.post(endpoint).json(&body).send()?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(AppError::Message(format!(
            "Ollama request failed with status {status}: {body}"
        )));
    }

    let value: serde_json::Value = response.json()?;
    let raw = value
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Message("Ollama response did not contain text.".to_string()))?;
    Ok(normalize_conversation_feedback(
        serde_json::from_str::<ConversationFeedback>(raw.trim())?,
    ))
}

fn call_openai_writing_feedback(
    config: &AppConfigPayload,
    prompt: &str,
    draft: &str,
) -> Result<OpenAiWritingFeedback, AppError> {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.trim().is_empty() {
        return Err(AppError::Message(
            "OpenAI API key is not configured. Set OPENAI_API_KEY in the environment."
                .to_string(),
        ));
    }

    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "corrected_draft": { "type": "string" },
            "shortened_draft": { "type": "string" },
            "feedback_summary": { "type": "string" },
            "tone_label": { "type": "string" },
            "weaknesses": {
                "type": "array",
                "items": { "type": "string" },
                "minItems": 1,
                "maxItems": 4
            },
            "score_clarity": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_conciseness": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_tone": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_business": { "type": "integer", "minimum": 0, "maximum": 5 },
            "score_grammar": { "type": "integer", "minimum": 0, "maximum": 5 }
        },
        "required": [
            "corrected_draft",
            "shortened_draft",
            "feedback_summary",
            "tone_label",
            "weaknesses",
            "score_clarity",
            "score_conciseness",
            "score_tone",
            "score_business",
            "score_grammar"
        ],
        "additionalProperties": false
    });

    let body = serde_json::json!({
        "model": config.openai_model,
        "reasoning": { "effort": "low" },
        "instructions": "You are an expert business English coach for a Japanese software engineer. Evaluate short business writing for clarity, conciseness, tone, grammar, and business usefulness. Prefer direct business writing over academic style. Always preserve the user's intent. Return valid JSON only.",
        "input": format!("Task:\n{prompt}\n\nUser draft:\n{draft}\n\nReturn:\n- a corrected draft\n- a shorter version\n- a one-sentence feedback summary\n- a tone label\n- 1 to 4 weakness tags\n- five integer scores from 0 to 5"),
        "text": {
            "format": {
                "type": "json_schema",
                "name": "writing_feedback",
                "schema": schema,
                "strict": true
            }
        }
    });

    let endpoint = format!("{}/responses", config.openai_api_base.trim_end_matches('/'));
    let client = Client::builder().timeout(Duration::from_secs(45)).build()?;
    let response = client
        .post(endpoint)
        .bearer_auth(api_key.trim())
        .json(&body)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(AppError::Message(format!(
            "OpenAI request failed with status {status}: {body}"
        )));
    }

    let value: serde_json::Value = response.json()?;
    let output = value
        .get("output")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Message("OpenAI response did not contain output.".to_string()))?;

    let mut text_chunks = Vec::new();
    for item in output {
        let parsed: ResponsesApiOutputItem = serde_json::from_value(item.clone())?;
        if let ResponsesApiOutputItem::Message(message) = parsed {
            for content in message.content {
                if let ResponsesApiContent::OutputText(text) = content {
                    text_chunks.push(text.text);
                }
            }
        }
    }

    let joined = text_chunks.join("\n").trim().to_string();
    if joined.is_empty() {
        return Err(AppError::Message(
            "OpenAI response did not contain output text.".to_string(),
        ));
    }

    Ok(serde_json::from_str::<OpenAiWritingFeedback>(&joined)?)
}

fn call_ollama_writing_feedback(
    config: &AppConfigPayload,
    prompt: &str,
    draft: &str,
) -> Result<OllamaWritingFeedback, AppError> {
    let client = Client::builder().timeout(Duration::from_secs(60)).build()?;
    let endpoint = format!("{}/api/generate", config.ollama_api_base.trim_end_matches('/'));
    let prompt_text = format!(
        "You are an expert business English coach for a Japanese software engineer.\n\
Return JSON only with these keys: corrected_draft, shortened_draft, feedback_summary, tone_label, weaknesses, score_clarity, score_conciseness, score_tone, score_business, score_grammar.\n\
Scores must be integers from 0 to 5.\n\
Task:\n{prompt}\n\nUser draft:\n{draft}"
    );
    let body = serde_json::json!({
        "model": config.ollama_model,
        "prompt": prompt_text,
        "stream": false,
        "format": {
            "type": "object",
            "properties": {
                "corrected_draft": { "type": "string" },
                "shortened_draft": { "type": "string" },
                "feedback_summary": { "type": "string" },
                "tone_label": { "type": "string" },
                "weaknesses": { "type": "array", "items": { "type": "string" } },
                "score_clarity": { "type": "integer" },
                "score_conciseness": { "type": "integer" },
                "score_tone": { "type": "integer" },
                "score_business": { "type": "integer" },
                "score_grammar": { "type": "integer" }
            },
            "required": [
                "corrected_draft",
                "shortened_draft",
                "feedback_summary",
                "tone_label",
                "weaknesses",
                "score_clarity",
                "score_conciseness",
                "score_tone",
                "score_business",
                "score_grammar"
            ]
        }
    });

    let response = client.post(endpoint).json(&body).send()?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(AppError::Message(format!(
            "Ollama request failed with status {status}: {body}"
        )));
    }

    let value: serde_json::Value = response.json()?;
    let raw = value
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Message("Ollama response did not contain text.".to_string()))?;
    Ok(serde_json::from_str::<OllamaWritingFeedback>(raw.trim())?)
}

fn fetch_ollama_models(config: &AppConfigPayload) -> Result<Vec<OllamaModelInfo>, AppError> {
    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;
    let endpoint = format!("{}/api/tags", config.ollama_api_base.trim_end_matches('/'));
    let response = client.get(endpoint).send()?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(AppError::Message(format!(
            "Ollama tags request failed with status {status}: {body}"
        )));
    }
    let payload: OllamaTagsResponse = response.json()?;
    Ok(payload
        .models
        .into_iter()
        .map(|model| OllamaModelInfo {
            name: model.name,
            size_bytes: model.size,
            modified_at: model.modified_at,
        })
        .collect())
}

#[tauri::command]
fn get_home_payload(state: tauri::State<AppState>) -> Result<HomePayload, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let today = Utc::now().date_naive().to_string();

    let mut task_stmt = conn
        .prepare(
            "SELECT id, scenario_id, task_date, task_type, title, prompt, scenario_tag, status, estimated_minutes
             FROM daily_task
             WHERE task_date = ?1
             ORDER BY id ASC",
        )
        .map_err(|err| err.to_string())?;

    let tasks = task_stmt
        .query_map(params![today], |row| {
            Ok(DailyTask {
                id: row.get(0)?,
                scenario_id: row.get(1)?,
                task_date: row.get(2)?,
                task_type: row.get(3)?,
                title: row.get(4)?,
                prompt: row.get(5)?,
                scenario_tag: row.get(6)?,
                status: row.get(7)?,
                estimated_minutes: row.get(8)?,
            })
        })
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    let mut weakness_stmt = conn
        .prepare(
            "SELECT tag_name, COUNT(*) AS count, MAX(created_at) AS last_seen_at
             FROM weakness_tag_event
             GROUP BY tag_name
             ORDER BY count DESC, last_seen_at DESC
             LIMIT 5",
        )
        .map_err(|err| err.to_string())?;

    let weaknesses = weakness_stmt
        .query_map([], |row| {
            Ok(WeaknessTag {
                tag_name: row.get(0)?,
                count: row.get(1)?,
                last_seen_at: row.get(2)?,
            })
        })
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    let total_study_minutes: i64 = conn
        .query_row(
            "SELECT COALESCE((SELECT COUNT(*) * 15 FROM writing_session), 0) + COALESCE((SELECT COUNT(*) * 10 FROM conversation_session), 0)",
            [],
            |row| row.get(0),
        )
        .map_err(|err| err.to_string())?;

    let conversation_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM conversation_session", [], |row| row.get(0))
        .map_err(|err| err.to_string())?;

    let writing_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM writing_session", [], |row| row.get(0))
        .map_err(|err| err.to_string())?;

    let average_writing_score: Option<f64> = conn
        .query_row(
            "SELECT AVG((score_clarity + score_conciseness + score_tone + score_business + score_grammar) / 5.0) FROM writing_session",
            [],
            |row| row.get(0),
        )
        .map_err(|err| err.to_string())?;

    let average_speaking_score: Option<f64> = conn
        .query_row(
            "SELECT AVG((score_structure + score_speed + score_business + score_paraphrase + score_intelligibility) / 5.0) FROM conversation_session",
            [],
            |row| row.get(0),
        )
        .map_err(|err| err.to_string())?;

    Ok(HomePayload {
        tasks,
        weaknesses,
        dashboard: DashboardSummary {
            total_study_minutes,
            conversation_count,
            writing_count,
            average_writing_score: average_writing_score.unwrap_or(0.0),
            average_speaking_score: average_speaking_score.unwrap_or(0.0),
        },
    })
}

#[tauri::command]
fn refresh_daily_tasks(state: tauri::State<AppState>) -> Result<HomePayload, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let today = Utc::now().date_naive().to_string();

    let mut stmt = conn
        .prepare(
            "SELECT id, task_type, scenario_id
             FROM daily_task
             WHERE task_date = ?1
             ORDER BY id ASC",
        )
        .map_err(|err| err.to_string())?;

    let pending_rows = stmt
        .query_map(params![today.clone()], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    for (task_id, task_type, current_scenario_id) in pending_rows {
        let scenario = choose_next_scenario_for_refresh(&conn, &task_type, &today, &current_scenario_id)
            .map_err(|err| err.to_string())?;

        conn.execute(
            "UPDATE daily_task
             SET scenario_id = ?1, title = ?2, prompt = ?3, scenario_tag = ?4, estimated_minutes = ?5, status = 'pending'
             WHERE id = ?6",
            params![
                scenario.id,
                scenario.title,
                scenario.prompt,
                scenario.scenario_tag,
                scenario.estimated_minutes,
                task_id
            ],
        )
        .map_err(|err| err.to_string())?;

        conn.execute(
            "INSERT INTO daily_refresh_history (task_date, task_type, scenario_id, shown_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![today, task_type, scenario.id, Utc::now().to_rfc3339()],
        )
        .map_err(|err| err.to_string())?;
    }

    get_home_payload(state)
}

#[tauri::command]
fn get_app_config() -> Result<AppConfigResponse, String> {
    let payload = load_app_config_internal().map_err(|err| err.to_string())?;
    Ok(app_config_response(&payload))
}

#[tauri::command]
fn save_app_config(payload: AppConfigPayload) -> Result<AppConfigResponse, String> {
    let existing = load_app_config_internal().unwrap_or(AppConfigPayload {
        provider: "openai".to_string(),
        openai_model: "gpt-5-mini".to_string(),
        openai_api_base: "https://api.openai.com/v1".to_string(),
        ollama_model: "mistral:latest".to_string(),
        ollama_api_base: "http://127.0.0.1:11434".to_string(),
    });
    let normalized = AppConfigPayload {
        provider: if payload.provider.trim().is_empty() {
            existing.provider
        } else {
            payload.provider.trim().to_string()
        },
        openai_model: if payload.openai_model.trim().is_empty() {
            "gpt-5-mini".to_string()
        } else {
            payload.openai_model.trim().to_string()
        },
        openai_api_base: if payload.openai_api_base.trim().is_empty() {
            "https://api.openai.com/v1".to_string()
        } else {
            payload.openai_api_base.trim().to_string()
        },
        ollama_model: if payload.ollama_model.trim().is_empty() {
            "mistral:latest".to_string()
        } else {
            payload.ollama_model.trim().to_string()
        },
        ollama_api_base: if payload.ollama_api_base.trim().is_empty() {
            "http://127.0.0.1:11434".to_string()
        } else {
            payload.ollama_api_base.trim().to_string()
        },
    };
    save_app_config_internal(normalized).map_err(|err| err.to_string())
}

#[tauri::command]
fn list_scenario_progress(state: tauri::State<AppState>) -> Result<Vec<ScenarioProgressRecord>, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let sql = "
        SELECT
            s.scenario_id,
            s.task_type,
            COALESCE(c.title, s.scenario_id) AS title,
            COUNT(*) AS attempts,
            AVG(s.score_value) AS average_score,
            MIN(s.started_at) AS first_attempt_at,
            MAX(s.started_at) AS last_attempt_at,
            (
                SELECT s2.score_value
                FROM (
                    SELECT scenario_id, 'conversation' AS task_type, started_at,
                           (score_structure + score_speed + score_business + score_paraphrase + score_intelligibility) / 5.0 AS score_value
                    FROM conversation_session
                    UNION ALL
                    SELECT scenario_id, 'writing' AS task_type, started_at,
                           (score_clarity + score_conciseness + score_tone + score_business + score_grammar) / 5.0 AS score_value
                    FROM writing_session
                ) s2
                WHERE s2.scenario_id = s.scenario_id AND s2.task_type = s.task_type
                ORDER BY s2.started_at DESC
                LIMIT 1
            ) AS latest_score
        FROM (
            SELECT scenario_id, 'conversation' AS task_type, started_at,
                   (score_structure + score_speed + score_business + score_paraphrase + score_intelligibility) / 5.0 AS score_value
            FROM conversation_session
            WHERE scenario_id <> ''
            UNION ALL
            SELECT scenario_id, 'writing' AS task_type, started_at,
                   (score_clarity + score_conciseness + score_tone + score_business + score_grammar) / 5.0 AS score_value
            FROM writing_session
            WHERE scenario_id <> ''
        ) s
        LEFT JOIN scenario_catalog c ON c.id = s.scenario_id
        GROUP BY s.scenario_id, s.task_type, c.title
        ORDER BY last_attempt_at DESC
        LIMIT 12
    ";
    let mut stmt = conn.prepare(sql).map_err(|err| err.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ScenarioProgressRecord {
                scenario_id: row.get(0)?,
                task_type: row.get(1)?,
                title: row.get(2)?,
                attempts: row.get(3)?,
                average_score: row.get(4)?,
                first_attempt_at: row.get(5)?,
                last_attempt_at: row.get(6)?,
                latest_score: row.get(7)?,
            })
        })
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;
    Ok(rows)
}

#[tauri::command]
fn list_ollama_models() -> Result<Vec<OllamaModelInfo>, String> {
    let config = load_app_config_internal().map_err(|err| err.to_string())?;
    fetch_ollama_models(&config).map_err(|err| err.to_string())
}

fn clamp_retention(score: f64) -> f64 {
    score.clamp(0.0, 1.0)
}

fn retention_after_review(current: f64, outcome: &str) -> f64 {
    match outcome {
        "still_hard" => clamp_retention(current * 0.6),
        "got_it" => clamp_retention(current + 0.22),
        "reviewed" => clamp_retention(current + 0.08),
        _ => current,
    }
}

#[tauri::command]
fn add_vocab_note(
    state: tauri::State<AppState>,
    payload: AddVocabNotePayload,
) -> Result<VocabNoteRecord, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let expression = payload.expression.trim();
    let meaning_ja = payload.meaning_ja.trim();
    let note = payload.note.trim();
    let example = payload.example.trim();

    if expression.is_empty() || meaning_ja.is_empty() {
        return Err("Expression and Japanese meaning are required.".to_string());
    }

    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO vocab_note (expression, meaning_ja, note, example, review_count, retention_score, last_result, created_at)
         VALUES (?1, ?2, ?3, ?4, 0, 0.35, 'new', ?5)",
        params![expression, meaning_ja, note, example, now],
    )
    .map_err(|err| err.to_string())?;

    let id = conn.last_insert_rowid();

    Ok(VocabNoteRecord {
        id,
        expression: expression.to_string(),
        meaning_ja: meaning_ja.to_string(),
        note: note.to_string(),
        example: example.to_string(),
        review_count: 0,
        retention_score: 0.35,
        last_result: "new".to_string(),
        created_at: now,
        last_reviewed_at: None,
    })
}

#[tauri::command]
fn list_vocab_notes(state: tauri::State<AppState>) -> Result<Vec<VocabNoteRecord>, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, expression, meaning_ja, note, example, review_count, retention_score, last_result, created_at, last_reviewed_at
             FROM vocab_note
             ORDER BY retention_score ASC, COALESCE(last_reviewed_at, created_at) ASC, created_at ASC
             LIMIT 100",
        )
        .map_err(|err| err.to_string())?;

    let notes = stmt
        .query_map([], |row| {
            Ok(VocabNoteRecord {
                id: row.get(0)?,
                expression: row.get(1)?,
                meaning_ja: row.get(2)?,
                note: row.get(3)?,
                example: row.get(4)?,
                review_count: row.get(5)?,
                retention_score: row.get(6)?,
                last_result: row.get(7)?,
                created_at: row.get(8)?,
                last_reviewed_at: row.get(9)?,
            })
        })
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    Ok(notes)
}

#[tauri::command]
fn get_word_training_payload(state: tauri::State<AppState>) -> Result<WordTrainingPayload, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let now = Utc::now().to_rfc3339();
    let sql = "
        SELECT
            m.id,
            m.card_key,
            m.word,
            m.meaning_ja,
            m.example,
            m.category,
            m.office_priority,
            p.mastery_score,
            p.pass_count,
            p.fail_count,
            p.streak,
            p.last_result,
            p.next_due_at,
            p.is_mastered
        FROM word_card_master m
        JOIN word_card_progress p ON p.word_id = m.id
        ORDER BY p.is_mastered ASC, p.mastery_score ASC, m.office_priority DESC, COALESCE(p.next_due_at, '') ASC, m.word ASC";

    let mut stmt = conn.prepare(sql).map_err(|err| err.to_string())?;
    let all_cards = stmt
        .query_map([], map_word_row)
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    let queue = all_cards
        .iter()
        .filter(|card| {
            !card.is_mastered
                && card
                    .next_due_at
                    .as_ref()
                    .map(|due| due <= &now)
                    .unwrap_or(true)
        })
        .take(8)
        .cloned()
        .collect::<Vec<_>>();

    let fallback_queue = if queue.is_empty() {
        all_cards
            .iter()
            .filter(|card| !card.is_mastered)
            .take(8)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        queue
    };

    let active_count = all_cards.iter().filter(|card| !card.is_mastered).count() as i64;
    let mastered_count = all_cards.iter().filter(|card| card.is_mastered).count() as i64;

    Ok(WordTrainingPayload {
        queue: fallback_queue,
        library: all_cards.into_iter().take(40).collect(),
        active_count,
        mastered_count,
    })
}

#[tauri::command]
fn submit_word_attempt(
    state: tauri::State<AppState>,
    payload: WordAttemptPayload,
) -> Result<WordTrainingPayload, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let (current_mastery, pass_count, fail_count, streak): (f64, i64, i64, i64) = conn
        .query_row(
            "SELECT mastery_score, pass_count, fail_count, streak
             FROM word_card_progress
             WHERE word_id = ?1",
            params![payload.word_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|err| err.to_string())?;

    let next_streak = if payload.result == "pass" { streak + 1 } else { 0 };
    let next_mastery = mastery_after_attempt(current_mastery, &payload.result, streak);
    let next_pass_count = if payload.result == "pass" { pass_count + 1 } else { pass_count };
    let next_fail_count = if payload.result == "pass" {
        fail_count
    } else {
        fail_count + 1
    };
    let is_mastered = next_mastery >= 0.93 && next_pass_count >= 5 && next_streak >= 3;
    let now = Utc::now();
    let next_due_at = next_due_after_attempt(now, next_mastery, &payload.result);

    conn.execute(
        "UPDATE word_card_progress
         SET mastery_score = ?1,
             pass_count = ?2,
             fail_count = ?3,
             streak = ?4,
             last_result = ?5,
             last_seen_at = ?6,
             next_due_at = ?7,
             is_mastered = ?8
         WHERE word_id = ?9",
        params![
            next_mastery,
            next_pass_count,
            next_fail_count,
            next_streak,
            payload.result,
            now.to_rfc3339(),
            next_due_at,
            if is_mastered { 1 } else { 0 },
            payload.word_id
        ],
    )
    .map_err(|err| err.to_string())?;

    get_word_training_payload(state)
}

#[tauri::command]
fn review_vocab_note(
    state: tauri::State<AppState>,
    payload: ReviewVocabPayload,
) -> Result<VocabNoteRecord, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let current_score: f64 = conn
        .query_row(
            "SELECT retention_score FROM vocab_note WHERE id = ?1",
            params![payload.note_id],
            |row| row.get(0),
        )
        .map_err(|err| err.to_string())?;
    let now = Utc::now().to_rfc3339();
    let next_score = retention_after_review(current_score, &payload.outcome);
    conn.execute(
        "UPDATE vocab_note
         SET review_count = review_count + 1,
             retention_score = ?1,
             last_result = ?2,
             last_reviewed_at = ?3
         WHERE id = ?4",
        params![next_score, payload.outcome, now, payload.note_id],
    )
    .map_err(|err| err.to_string())?;

    conn.query_row(
        "SELECT id, expression, meaning_ja, note, example, review_count, retention_score, last_result, created_at, last_reviewed_at
         FROM vocab_note
         WHERE id = ?1",
        params![payload.note_id],
        |row| {
            Ok(VocabNoteRecord {
                id: row.get(0)?,
                expression: row.get(1)?,
                meaning_ja: row.get(2)?,
                note: row.get(3)?,
                example: row.get(4)?,
                review_count: row.get(5)?,
                retention_score: row.get(6)?,
                last_result: row.get(7)?,
                created_at: row.get(8)?,
                last_reviewed_at: row.get(9)?,
            })
        },
    )
    .map_err(|err| err.to_string())
}

#[tauri::command]
fn delete_vocab_note(state: tauri::State<AppState>, note_id: i64) -> Result<(), String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    conn.execute("DELETE FROM vocab_note WHERE id = ?1", params![note_id])
        .map_err(|err| err.to_string())?;
    Ok(())
}

#[tauri::command]
fn submit_writing_session(
    state: tauri::State<AppState>,
    payload: WritingSubmission,
) -> Result<WritingSessionRecord, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let now = Utc::now().to_rfc3339();
    let config = load_app_config_internal().map_err(|err| err.to_string())?;
    let api_result = if config.provider == "ollama" {
        call_ollama_writing_feedback(&config, &payload.prompt, &payload.draft).map(
            |result| (
                result.corrected_draft,
                result.shortened_draft,
                result.feedback_summary,
                result.weaknesses,
                [
                    result.score_clarity,
                    result.score_conciseness,
                    result.score_tone,
                    result.score_business,
                    result.score_grammar,
                ],
                format!("ollama:{}", config.ollama_model),
            ),
        )
    } else {
        call_openai_writing_feedback(&config, &payload.prompt, &payload.draft).map(
            |result| (
                result.corrected_draft,
                result.shortened_draft,
                result.feedback_summary,
                result.weaknesses,
                [
                    result.score_clarity,
                    result.score_conciseness,
                    result.score_tone,
                    result.score_business,
                    result.score_grammar,
                ],
                format!("openai:{}", config.openai_model),
            ),
        )
    };

    let (corrected, shortened, feedback, weaknesses, scores, evaluator, warning_message) =
        match api_result {
            Ok((corrected, shortened, feedback, weaknesses, scores, evaluator)) => (
                corrected,
                shortened,
                feedback,
                weaknesses,
                scores,
                evaluator,
                None,
            ),
            Err(err) => {
                let (corrected, shortened, feedback, weaknesses, scores) =
                    evaluate_writing(&payload.draft);
                (
                    corrected,
                    shortened,
                    feedback,
                    weaknesses.into_iter().map(|item| item.to_string()).collect(),
                    scores,
                    "local-fallback".to_string(),
                    Some(err.to_string()),
                )
            }
        };

    conn.execute(
        "INSERT INTO writing_session (
            scenario_id, started_at, task_kind, source_prompt, user_draft, corrected_draft, shortened_draft,
            feedback_summary, score_clarity, score_conciseness, score_tone, score_business, score_grammar,
            evaluator, warning_message
        ) VALUES (?1, ?2, 'writing', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            payload.scenario_id.unwrap_or_default(),
            now,
            payload.prompt,
            payload.draft,
            corrected,
            shortened,
            feedback,
            scores[0],
            scores[1],
            scores[2],
            scores[3],
            scores[4],
            evaluator,
            warning_message
        ],
    )
    .map_err(|err| err.to_string())?;

    let session_id = conn.last_insert_rowid();

    conn.execute(
        "UPDATE daily_task SET status = 'completed' WHERE id = ?1",
        params![payload.task_id],
    )
    .map_err(|err| err.to_string())?;

    for weakness in weaknesses {
        conn.execute(
            "INSERT INTO weakness_tag_event (created_at, source_type, source_id, tag_name, severity)
             VALUES (?1, 'writing', ?2, ?3, 2)",
            params![Utc::now().to_rfc3339(), session_id, weakness],
        )
        .map_err(|err| err.to_string())?;
    }

    Ok(WritingSessionRecord {
        id: session_id,
        started_at: now,
        task_kind: "writing".to_string(),
        source_prompt: payload.prompt,
        user_draft: payload.draft,
        corrected_draft: corrected,
        shortened_draft: shortened,
        feedback_summary: feedback,
        score_clarity: scores[0],
        score_conciseness: scores[1],
        score_tone: scores[2],
        score_business: scores[3],
        score_grammar: scores[4],
        evaluator,
        warning_message,
    })
}

#[tauri::command]
fn submit_conversation_session(
    state: tauri::State<AppState>,
    payload: ConversationSubmission,
) -> Result<ConversationSessionRecord, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let now = Utc::now().to_rfc3339();
    let config = load_app_config_internal().map_err(|err| err.to_string())?;

    let api_result = if config.provider == "ollama" {
        call_ollama_conversation_feedback(&config, &payload.prompt, &payload.response_text)
            .map(|result| (result, format!("ollama:{}", config.ollama_model)))
    } else {
        call_openai_conversation_feedback(&config, &payload.prompt, &payload.response_text)
            .map(|result| (result, format!("openai:{}", config.openai_model)))
    };

    let (feedback, evaluator, warning_message, weakness_tags) = match api_result {
        Ok((feedback, evaluator)) => {
            let tags = feedback.weaknesses.clone();
            (feedback, evaluator, None, tags)
        }
        Err(err) => {
            let (fallback_feedback, weak) = evaluate_conversation_locally(&payload.response_text);
            (
                fallback_feedback,
                "local-fallback".to_string(),
                Some(err.to_string()),
                weak.into_iter().map(|item| item.to_string()).collect(),
            )
        }
    };
    let scenario_id = payload.scenario_id.clone().unwrap_or_default();

    conn.execute(
        "INSERT INTO conversation_session (
            scenario_id, started_at, scenario_type, role_type, objective, transcript, feedback_summary,
            improved_transcript, priority_fix, retry_prompt, score_structure, score_speed, score_business,
            score_paraphrase, score_intelligibility, evaluator, warning_message
        ) VALUES (?1, ?2, 'meeting', 'engineer', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            scenario_id,
            now,
            payload.prompt,
            payload.response_text,
            feedback.improved_transcript,
            feedback.feedback_summary,
            feedback.priority_fix,
            feedback.retry_prompt,
            feedback.score_structure,
            feedback.score_speed,
            feedback.score_business,
            feedback.score_paraphrase,
            feedback.score_intelligibility,
            evaluator,
            warning_message
        ],
    )
    .map_err(|err| err.to_string())?;

    let session_id = conn.last_insert_rowid();

    conn.execute(
        "UPDATE daily_task SET status = 'completed' WHERE id = ?1",
        params![payload.task_id],
    )
    .map_err(|err| err.to_string())?;

    for weakness in weakness_tags {
        conn.execute(
            "INSERT INTO weakness_tag_event (created_at, source_type, source_id, tag_name, severity)
             VALUES (?1, 'conversation', ?2, ?3, 2)",
            params![Utc::now().to_rfc3339(), session_id, weakness],
        )
        .map_err(|err| err.to_string())?;
    }

    Ok(ConversationSessionRecord {
        id: session_id,
        scenario_id: payload.scenario_id.unwrap_or_default(),
        started_at: now,
        scenario_type: "meeting".to_string(),
        role_type: "engineer".to_string(),
        objective: payload.prompt,
        transcript: payload.response_text,
        improved_transcript: feedback.improved_transcript,
        feedback_summary: feedback.feedback_summary,
        priority_fix: feedback.priority_fix,
        retry_prompt: feedback.retry_prompt,
        score_structure: feedback.score_structure,
        score_speed: feedback.score_speed,
        score_business: feedback.score_business,
        score_paraphrase: feedback.score_paraphrase,
        score_intelligibility: feedback.score_intelligibility,
        evaluator,
        warning_message,
    })
}

#[tauri::command]
fn list_recent_writing_sessions(
    state: tauri::State<AppState>,
) -> Result<Vec<WritingSessionRecord>, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, started_at, task_kind, source_prompt, user_draft, corrected_draft,
                    shortened_draft, feedback_summary, score_clarity, score_conciseness,
                    score_tone, score_business, score_grammar, evaluator, warning_message
             FROM writing_session
             ORDER BY started_at DESC
             LIMIT 10",
        )
        .map_err(|err| err.to_string())?;

    let sessions = stmt
        .query_map([], |row| {
        Ok(WritingSessionRecord {
            id: row.get(0)?,
            started_at: row.get(1)?,
            task_kind: row.get(2)?,
            source_prompt: row.get(3)?,
            user_draft: row.get(4)?,
            corrected_draft: row.get(5)?,
            shortened_draft: row.get(6)?,
            feedback_summary: row.get(7)?,
            score_clarity: row.get(8)?,
            score_conciseness: row.get(9)?,
            score_tone: row.get(10)?,
            score_business: row.get(11)?,
            score_grammar: row.get(12)?,
            evaluator: row.get(13)?,
            warning_message: row.get(14)?,
        })
    })
    .map_err(|err| err.to_string())?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|err| err.to_string())?;

    Ok(sessions)
}

#[tauri::command]
fn list_recent_conversation_sessions(
    state: tauri::State<AppState>,
) -> Result<Vec<ConversationSessionRecord>, String> {
    let conn = open_database(&state).map_err(|err| err.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, scenario_id, started_at, scenario_type, role_type, objective, transcript, improved_transcript, feedback_summary,
                    priority_fix, retry_prompt, score_structure, score_speed, score_business,
                    score_paraphrase, score_intelligibility, evaluator, warning_message
             FROM conversation_session
             ORDER BY started_at DESC
             LIMIT 10",
        )
        .map_err(|err| err.to_string())?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(ConversationSessionRecord {
                id: row.get(0)?,
                scenario_id: row.get(1)?,
                started_at: row.get(2)?,
                scenario_type: row.get(3)?,
                role_type: row.get(4)?,
                objective: row.get(5)?,
                transcript: row.get(6)?,
                improved_transcript: row.get(7)?,
                feedback_summary: row.get(8)?,
                priority_fix: row.get(9)?,
                retry_prompt: row.get(10)?,
                score_structure: row.get(11)?,
                score_speed: row.get(12)?,
                score_business: row.get(13)?,
                score_paraphrase: row.get(14)?,
                score_intelligibility: row.get(15)?,
                evaluator: row.get(16)?,
                warning_message: row.get(17)?,
            })
        })
        .map_err(|err| err.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| err.to_string())?;

    Ok(sessions)
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_home_payload,
            refresh_daily_tasks,
            get_app_config,
            save_app_config,
            list_ollama_models,
            list_scenario_progress,
            add_vocab_note,
            list_vocab_notes,
            get_word_training_payload,
            submit_word_attempt,
            review_vocab_note,
            delete_vocab_note,
            submit_conversation_session,
            submit_writing_session,
            list_recent_writing_sessions,
            list_recent_conversation_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
