import { useEffect, useMemo, useState } from "react";
import {
  addVocabNote,
  deleteVocabNote,
  loadAppConfig,
  loadConversationHistory,
  loadHome,
  loadScenarioProgress,
  loadVocabNotes,
  loadWritingHistory,
  listOllamaModels,
  refreshDailyTasks,
  reviewVocabNote,
  saveAppConfig,
  submitConversation,
  submitWriting
} from "./api";
import type {
  AddVocabNoteInput,
  AppConfig,
  AppConfigInput,
  ConversationSessionRecord,
  DailyTask,
  HomePayload,
  OllamaModelInfo,
  ScenarioProgressRecord,
  VocabNoteRecord,
  VocabReviewOutcome,
  WritingSessionRecord
} from "./types";

type Screen = "home" | "conversation" | "writing" | "review" | "vocab";

const emptyHome: HomePayload = {
  tasks: [],
  weaknesses: [],
  dashboard: {
    totalStudyMinutes: 0,
    conversationCount: 0,
    writingCount: 0,
    averageWritingScore: 0,
    averageSpeakingScore: 0
  }
};

export function App() {
  const [screen, setScreen] = useState<Screen>("home");
  const [home, setHome] = useState<HomePayload>(emptyHome);
  const [history, setHistory] = useState<WritingSessionRecord[]>([]);
  const [conversationHistory, setConversationHistory] = useState<ConversationSessionRecord[]>([]);
  const [scenarioProgress, setScenarioProgress] = useState<ScenarioProgressRecord[]>([]);
  const [vocabNotes, setVocabNotes] = useState<VocabNoteRecord[]>([]);
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [ollamaModels, setOllamaModels] = useState<OllamaModelInfo[]>([]);
  const [configDraft, setConfigDraft] = useState<AppConfigInput>({
    provider: "openai",
    openaiModel: "gpt-5-mini",
    openaiApiBase: "https://api.openai.com/v1",
    ollamaModel: "mistral:latest",
    ollamaApiBase: "http://127.0.0.1:11434"
  });
  const [activeTask, setActiveTask] = useState<DailyTask | null>(null);
  const [conversationTask, setConversationTask] = useState<DailyTask | null>(null);
  const [prompt, setPrompt] = useState("");
  const [draft, setDraft] = useState("");
  const [conversationPrompt, setConversationPrompt] = useState("");
  const [conversationResponse, setConversationResponse] = useState("");
  const [latestResult, setLatestResult] = useState<WritingSessionRecord | null>(null);
  const [latestConversation, setLatestConversation] =
    useState<ConversationSessionRecord | null>(null);
  const [vocabDraft, setVocabDraft] = useState<AddVocabNoteInput>({
    expression: "",
    meaningJa: "",
    note: "",
    example: ""
  });
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [savingConfig, setSavingConfig] = useState(false);
  const [savingVocab, setSavingVocab] = useState(false);
  const [refreshingDaily, setRefreshingDaily] = useState(false);
  const [vocabSort, setVocabSort] = useState<"weakest" | "strongest" | "recent">("weakest");
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const [homePayload, recentHistory, recentConversation, progress, notes, appConfig, localModels] = await Promise.all([
        loadHome(),
        loadWritingHistory(),
        loadConversationHistory(),
        loadScenarioProgress(),
        loadVocabNotes(),
        loadAppConfig(),
        listOllamaModels()
      ]);
      setHome(homePayload);
      setHistory(recentHistory);
      setConversationHistory(recentConversation);
      setScenarioProgress(progress);
      setVocabNotes(notes);
      setConfig(appConfig);
      setOllamaModels(localModels);
      setConfigDraft((current) => ({
        ...current,
        provider: appConfig.provider,
        openaiModel: appConfig.openaiModel,
        openaiApiBase: appConfig.openaiApiBase,
        ollamaModel: appConfig.ollamaModel,
        ollamaApiBase: appConfig.ollamaApiBase
      }));
      const writingTask =
        homePayload.tasks.find((task) => task.taskType === "writing" && task.status === "pending") ??
        homePayload.tasks.find((task) => task.taskType === "writing") ??
        null;
      const conversation =
        homePayload.tasks.find((task) => task.taskType === "conversation" && task.status === "pending") ??
        homePayload.tasks.find((task) => task.taskType === "conversation") ??
        null;
      setActiveTask(writingTask);
      setConversationTask(conversation);
      setPrompt(writingTask?.prompt ?? "");
      setConversationPrompt(conversation?.prompt ?? "");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load app data.");
    } finally {
      setLoading(false);
    }
  }

  async function handleAddVocab() {
    if (!vocabDraft.expression.trim() || !vocabDraft.meaningJa.trim()) {
      return;
    }

    setSavingVocab(true);
    setError(null);
    try {
      await addVocabNote(vocabDraft);
      setVocabDraft({
        expression: "",
        meaningJa: "",
        note: "",
        example: ""
      });
      await refresh();
      setScreen("vocab");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save vocab.");
    } finally {
      setSavingVocab(false);
    }
  }

  function handlePickWritingSelectionToVocab(
    session: WritingSessionRecord,
    sourceLabel: "corrected" | "shorter"
  ) {
    const selectedText = window.getSelection?.()?.toString().trim() ?? "";
    if (!selectedText) {
      setError("Select a word or phrase in Corrected or Shorter version first, then click the button.");
      return;
    }
    const sourceText =
      sourceLabel === "shorter" ? session.shortenedDraft : session.correctedDraft;
    setError(null);
    setVocabDraft({
      expression: selectedText,
      meaningJa: "",
      note: `Picked from writing practice (${sourceLabel}). Prompt: ${session.sourcePrompt}`,
      example: sourceText
    });
    setScreen("vocab");
  }

  function handlePickConversationSelectionToVocab(session: ConversationSessionRecord) {
    const selectedText = window.getSelection?.()?.toString().trim() ?? "";
    if (!selectedText) {
      setError("Select a word or phrase in Improved version first, then click the button.");
      return;
    }
    setError(null);
    setVocabDraft({
      expression: selectedText,
      meaningJa: "",
      note: `Picked from conversation practice. Scenario: ${session.objective}`,
      example: session.improvedTranscript
    });
    setScreen("vocab");
  }

  async function handleVocabReview(noteId: number, outcome: VocabReviewOutcome) {
    setError(null);
    try {
      await reviewVocabNote(noteId, outcome);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update review count.");
    }
  }

  async function handleDeleteVocab(noteId: number, expression: string) {
    const approved = window.confirm(`Delete "${expression}" from Vocab?`);
    if (!approved) {
      return;
    }
    setError(null);
    try {
      await deleteVocabNote(noteId);
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete vocab note.");
    }
  }

  const sortedVocabNotes = useMemo(() => {
    const items = [...vocabNotes];
    if (vocabSort === "strongest") {
      return items.sort((a, b) => b.retentionScore - a.retentionScore);
    }
    if (vocabSort === "recent") {
      return items.sort((a, b) => {
        const aTime = new Date(a.lastReviewedAt ?? a.createdAt).getTime();
        const bTime = new Date(b.lastReviewedAt ?? b.createdAt).getTime();
        return bTime - aTime;
      });
    }
    return items.sort((a, b) => a.retentionScore - b.retentionScore);
  }, [vocabNotes, vocabSort]);

  useEffect(() => {
    void refresh();
  }, []);

  const nextTaskLabel = useMemo(() => {
    const nextTask = home.tasks.find((task) => task.status === "pending");
    return nextTask ? `Start: ${nextTask.title}` : "All tasks completed";
  }, [home.tasks]);

  async function handleSubmit() {
    if (!activeTask || !draft.trim()) {
      return;
    }

    setSubmitting(true);
    setError(null);

    try {
      const result = await submitWriting({
        taskId: activeTask.id,
        scenarioId: activeTask.scenarioId,
        prompt,
        draft
      });
      setLatestResult(result);
      setDraft("");
      setScreen("review");
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit writing.");
    } finally {
      setSubmitting(false);
    }
  }

  async function handleSaveConfig() {
    setSavingConfig(true);
    setError(null);
    try {
      const saved = await saveAppConfig(configDraft);
      setConfig(saved);
      setConfigDraft((current) => ({
        ...current,
        provider: saved.provider,
        openaiModel: saved.openaiModel,
        openaiApiBase: saved.openaiApiBase,
        ollamaModel: saved.ollamaModel,
        ollamaApiBase: saved.ollamaApiBase
      }));
      setOllamaModels(await listOllamaModels());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save app config.");
    } finally {
      setSavingConfig(false);
    }
  }

  async function handleRefreshDaily() {
    setRefreshingDaily(true);
    setError(null);
    try {
      await refreshDailyTasks();
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to refresh daily tasks.");
    } finally {
      setRefreshingDaily(false);
    }
  }

  async function handleConversationSubmit() {
    if (!conversationTask || !conversationResponse.trim()) {
      return;
    }

    setSubmitting(true);
    setError(null);
    try {
      const result = await submitConversation({
        taskId: conversationTask.id,
        scenarioId: conversationTask.scenarioId,
        prompt: conversationPrompt,
        responseText: conversationResponse
      });
      setLatestConversation(result);
      setConversationResponse("");
      setScreen("review");
      await refresh();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit conversation.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div>
          <p className="eyebrow">Personal English OS</p>
          <h1>English Flight Deck</h1>
          <p className="sidebar-copy">
            Build daily momentum for meetings, writing, and interviews with the
            fewest possible clicks.
          </p>
        </div>

        <nav className="nav">
          <button
            className={screen === "home" ? "nav-link active" : "nav-link"}
            onClick={() => setScreen("home")}
          >
            Home
          </button>
          <button
            className={screen === "conversation" ? "nav-link active" : "nav-link"}
            onClick={() => setScreen("conversation")}
          >
            Conversation
          </button>
          <button
            className={screen === "writing" ? "nav-link active" : "nav-link"}
            onClick={() => setScreen("writing")}
          >
            Writing Drill
          </button>
          <button
            className={screen === "review" ? "nav-link active" : "nav-link"}
            onClick={() => setScreen("review")}
          >
            Review
          </button>
          <button
            className={screen === "vocab" ? "nav-link active" : "nav-link"}
            onClick={() => setScreen("vocab")}
          >
            Vocab
          </button>
        </nav>

        <div className="spotlight">
          <span className="spotlight-label">Next task</span>
          <strong>{nextTaskLabel}</strong>
        </div>
      </aside>

      <main className="content">
        {loading ? <div className="panel">Loading...</div> : null}
        {error ? <div className="panel error">{error}</div> : null}

        {!loading && screen === "home" ? (
          <section className="grid">
            <article className="panel hero">
              <p className="eyebrow">Today</p>
              <h2>Three tasks, one path</h2>
              <p>
                Keep daily friction low. Finish one conversation, one writing
                drill, and one review set.
              </p>
              <div className="hero-actions">
                <button
                  className="primary"
                  onClick={() =>
                    setScreen(conversationTask ? "conversation" : activeTask ? "writing" : "review")
                  }
                >
                  {nextTaskLabel}
                </button>
                <button
                  className="secondary"
                  onClick={() => void handleRefreshDaily()}
                  disabled={refreshingDaily}
                >
                  {refreshingDaily ? "Refreshing..." : "Refresh Daily"}
                </button>
              </div>
            </article>

            <article className="panel">
              <h3>Today's queue</h3>
              <div className="stack">
                {home.tasks.map((task) => (
                  <div key={task.id} className="task-card">
                    <div>
                      <strong>{task.title}</strong>
                      <p>
                        {task.scenarioTag} | {task.estimatedMinutes} min
                      </p>
                      <p>ID: {task.scenarioId}</p>
                    </div>
                    <span
                      className={
                        task.status === "completed" ? "status done" : "status"
                      }
                    >
                      {task.status}
                    </span>
                  </div>
                ))}
              </div>
            </article>

            <article className="panel">
              <h3>AI Provider</h3>
              <p>
                Switch between OpenAI and local Ollama. OpenAI now reads the API
                key only from the `OPENAI_API_KEY` environment variable.
              </p>
              <div className="stack compact">
                <label className="field">
                  <span>Provider</span>
                  <select
                    className="text-input"
                    value={configDraft.provider}
                    onChange={(event) =>
                      setConfigDraft((current) => ({
                        ...current,
                        provider: event.target.value as "openai" | "ollama"
                      }))
                    }
                  >
                    <option value="openai">OpenAI</option>
                    <option value="ollama">Ollama</option>
                  </select>
                </label>
                <label className="field">
                  <span>OpenAI model</span>
                  <input
                    className="text-input"
                    value={configDraft.openaiModel}
                    onChange={(event) =>
                      setConfigDraft((current) => ({
                        ...current,
                        openaiModel: event.target.value
                      }))
                    }
                  />
                </label>
                <label className="field">
                  <span>OpenAI API base</span>
                  <input
                    className="text-input"
                    value={configDraft.openaiApiBase}
                    onChange={(event) =>
                      setConfigDraft((current) => ({
                        ...current,
                        openaiApiBase: event.target.value
                      }))
                    }
                  />
                </label>
                <label className="field">
                  <span>Ollama model</span>
                  <input
                    className="text-input"
                    list="ollama-models"
                    value={configDraft.ollamaModel}
                    onChange={(event) =>
                      setConfigDraft((current) => ({
                        ...current,
                        ollamaModel: event.target.value
                      }))
                    }
                  />
                  <datalist id="ollama-models">
                    {ollamaModels.map((model) => (
                      <option key={model.name} value={model.name} />
                    ))}
                  </datalist>
                </label>
                <label className="field">
                  <span>Ollama API base</span>
                  <input
                    className="text-input"
                    value={configDraft.ollamaApiBase}
                    onChange={(event) =>
                      setConfigDraft((current) => ({
                        ...current,
                        ollamaApiBase: event.target.value
                      }))
                    }
                  />
                </label>
                <div className="panel-note">
                  <strong>Current:</strong>{" "}
                  {config?.provider === "ollama"
                    ? `Ollama / ${config.ollamaModel}`
                    : `OpenAI / ${config?.openaiModel ?? "gpt-5-mini"}`}
                  <br />
                  <strong>OpenAI key:</strong>{" "}
                  {config?.hasOpenAiApiKey ? "env detected" : "missing"}
                </div>
                <button
                  className="primary"
                  onClick={() => void handleSaveConfig()}
                  disabled={savingConfig}
                >
                  {savingConfig ? "Saving..." : "Save provider settings"}
                </button>
              </div>
            </article>

            <article className="panel">
              <h3>Focus weaknesses</h3>
              <div className="tag-list">
                {home.weaknesses.map((item) => (
                  <div key={item.tagName} className="tag-chip">
                    <strong>{item.tagName}</strong>
                    <span>{item.count} recent hits</span>
                  </div>
                ))}
              </div>
            </article>

            <article className="panel stats">
              <div>
                <span>Study minutes</span>
                <strong>{home.dashboard.totalStudyMinutes}</strong>
              </div>
              <div>
                <span>Writing drills</span>
                <strong>{home.dashboard.writingCount}</strong>
              </div>
              <div>
                <span>Speaking avg</span>
                <strong>{home.dashboard.averageSpeakingScore.toFixed(1)}</strong>
              </div>
              <div>
                <span>Writing avg</span>
                <strong>{home.dashboard.averageWritingScore.toFixed(1)}</strong>
              </div>
            </article>
          </section>
        ) : null}

        {!loading && screen === "conversation" ? (
          <section className="grid writing-layout">
            <article className="panel">
              <p className="eyebrow">Conversation drill</p>
              <h2>{conversationTask?.title ?? "No conversation task for today"}</h2>
              <label className="field">
                <span>Scenario</span>
                <textarea
                  value={conversationPrompt}
                  onChange={(event) => setConversationPrompt(event.target.value)}
                  rows={4}
                />
              </label>
              <label className="field">
                <span>Your response transcript</span>
                <textarea
                  value={conversationResponse}
                  onChange={(event) => setConversationResponse(event.target.value)}
                  rows={11}
                  placeholder="Type what you said or plan to say."
                />
              </label>
              <button
                className="primary"
                disabled={!conversationTask || !conversationResponse.trim() || submitting}
                onClick={() => void handleConversationSubmit()}
              >
                {submitting ? "Evaluating..." : "Submit conversation drill"}
              </button>
            </article>

            <article className="panel">
              <p className="eyebrow">Speaking target</p>
              <h3>What to optimize</h3>
              <ul className="plain-list">
                <li>Start with the conclusion or status.</li>
                <li>Keep one blocker concrete.</li>
                <li>Finish with the next action.</li>
                <li>Prefer short business phrasing over long explanation.</li>
              </ul>
            </article>
          </section>
        ) : null}

        {!loading && screen === "writing" ? (
          <section className="grid writing-layout">
            <article className="panel">
              <p className="eyebrow">Active drill</p>
              <h2>{activeTask?.title ?? "No writing task for today"}</h2>
              <label className="field">
                <span>Prompt</span>
                <textarea
                  value={prompt}
                  onChange={(event) => setPrompt(event.target.value)}
                  rows={4}
                />
              </label>
              <label className="field">
                <span>Your draft</span>
                <textarea
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  rows={11}
                  placeholder="Write the email here."
                />
              </label>
              <button
                className="primary"
                disabled={!activeTask || !draft.trim() || submitting}
                onClick={() => void handleSubmit()}
              >
                {submitting ? "Evaluating..." : "Submit writing drill"}
              </button>
            </article>

            <article className="panel">
              <p className="eyebrow">Design rule</p>
              <h3>What good looks like</h3>
              <ul className="plain-list">
                <li>Lead with the request or decision.</li>
                <li>Keep reasons short and concrete.</li>
                <li>End with one clear next action.</li>
                <li>Prefer business usefulness over elegant wording.</li>
              </ul>
            </article>
          </section>
        ) : null}

        {!loading && screen === "review" ? (
          <section className="grid">
            <article className="panel">
              <p className="eyebrow">Latest result</p>
              {latestResult ? (
                <>
                  <h2>Writing feedback</h2>
                  <div className="result-block">
                    <h4>Your version</h4>
                    <p>{latestResult.userDraft}</p>
                  </div>
                  <div className="result-block">
                    <h4>Summary</h4>
                    <p>{latestResult.feedbackSummary}</p>
                  </div>
                  <div className="result-block">
                    <h4>Evaluator</h4>
                    <p>{latestResult.evaluator}</p>
                    {latestResult.warningMessage ? (
                      <p className="warning-text">{latestResult.warningMessage}</p>
                    ) : null}
                  </div>
                  <div className="result-block">
                    <h4>Corrected</h4>
                    <p>{latestResult.correctedDraft}</p>
                    <button
                      className="secondary inline-action"
                      onClick={() => handlePickWritingSelectionToVocab(latestResult, "corrected")}
                    >
                      Pick selected text to Vocab
                    </button>
                  </div>
                  <div className="result-block">
                    <h4>Shorter version</h4>
                    <p>{latestResult.shortenedDraft}</p>
                    <button
                      className="secondary inline-action"
                      onClick={() => handlePickWritingSelectionToVocab(latestResult, "shorter")}
                    >
                      Pick selected text to Vocab
                    </button>
                  </div>
                </>
              ) : (
                <p>No submission yet. Complete one writing drill to populate this panel.</p>
              )}
              {latestConversation ? (
                <div className="result-block">
                  <h4>Latest conversation</h4>
                  <p>
                    <strong>Your version:</strong> {latestConversation.transcript}
                  </p>
                  <p>
                    <strong>Improved version:</strong> {latestConversation.improvedTranscript}
                  </p>
                  <button
                    className="secondary inline-action"
                    onClick={() => handlePickConversationSelectionToVocab(latestConversation)}
                  >
                    Pick selected text to Vocab
                  </button>
                  <p>
                    <strong>Summary:</strong> {latestConversation.feedbackSummary}
                  </p>
                  <p>
                    <strong>Priority fix:</strong> {latestConversation.priorityFix}
                  </p>
                  <p>
                    <strong>Retry prompt:</strong> {latestConversation.retryPrompt}
                  </p>
                </div>
              ) : null}
            </article>

            <article className="panel">
              <h3>Scenario progress</h3>
              <div className="stack">
                {scenarioProgress.length === 0 ? (
                  <p>No repeated scenario data yet.</p>
                ) : (
                  scenarioProgress.map((item) => (
                    <div key={`${item.taskType}-${item.scenarioId}`} className="history-card">
                      <div>
                        <strong>{item.title}</strong>
                        <p>{item.taskType} | {item.scenarioId}</p>
                        <p>
                          Attempts: {item.attempts} | Avg: {item.averageScore.toFixed(1)} | Latest:{" "}
                          {item.latestScore.toFixed(1)}
                        </p>
                        <p>Last: {new Date(item.lastAttemptAt).toLocaleString()}</p>
                      </div>
                      <span className="score-pill">{item.latestScore.toFixed(1)}</span>
                    </div>
                  ))
                )}
              </div>
            </article>

            <article className="panel">
              <h3>Recent writing sessions</h3>
              <div className="stack">
                {history.length === 0 ? (
                  <p>No saved writing sessions yet.</p>
                ) : (
                  history.map((session) => (
                    <div key={session.id} className="history-card">
                      <div>
                        <strong>{session.sourcePrompt}</strong>
                        <p>{new Date(session.startedAt).toLocaleString()}</p>
                        <p>{session.evaluator}</p>
                        <p>
                          <strong>Your version:</strong> {session.userDraft}
                        </p>
                        <p>
                          <strong>Corrected:</strong> {session.correctedDraft}
                        </p>
                        <button
                          className="secondary inline-action"
                          onClick={() => handlePickWritingSelectionToVocab(session, "corrected")}
                        >
                          Pick selected text from Corrected
                        </button>
                        <p>
                          <strong>Shorter:</strong> {session.shortenedDraft}
                        </p>
                        <button
                          className="secondary inline-action"
                          onClick={() => handlePickWritingSelectionToVocab(session, "shorter")}
                        >
                          Pick selected text from Shorter
                        </button>
                      </div>
                      <span className="score-pill">
                        {(
                          (session.scoreClarity +
                            session.scoreConciseness +
                            session.scoreTone +
                            session.scoreBusiness +
                            session.scoreGrammar) /
                          5
                        ).toFixed(1)}
                      </span>
                    </div>
                  ))
                )}
              </div>
            </article>

            <article className="panel">
              <h3>Recent conversation sessions</h3>
              <div className="stack">
                {conversationHistory.length === 0 ? (
                  <p>No saved conversation sessions yet.</p>
                ) : (
                  conversationHistory.map((session) => (
                    <div key={session.id} className="history-card">
                      <div>
                        <strong>{session.objective}</strong>
                        <p>{new Date(session.startedAt).toLocaleString()}</p>
                        <p>{session.evaluator}</p>
                        <p>
                          <strong>Your version:</strong> {session.transcript}
                        </p>
                        <p>
                          <strong>Improved:</strong> {session.improvedTranscript}
                        </p>
                        <button
                          className="secondary inline-action"
                          onClick={() => handlePickConversationSelectionToVocab(session)}
                        >
                          Pick selected text to Vocab
                        </button>
                        <p>
                          <strong>Fix:</strong> {session.priorityFix}
                        </p>
                      </div>
                      <span className="score-pill">
                        {(
                          (session.scoreStructure +
                            session.scoreSpeed +
                            session.scoreBusiness +
                            session.scoreParaphrase +
                            session.scoreIntelligibility) /
                          5
                        ).toFixed(1)}
                      </span>
                    </div>
                  ))
                )}
              </div>
            </article>
          </section>
        ) : null}

        {!loading && screen === "vocab" ? (
          <section className="grid writing-layout">
            <article className="panel">
              <p className="eyebrow">Vocab notebook</p>
              <h2>Save unclear expressions for spaced review</h2>
              <label className="field">
                <span>Expression</span>
                <input
                  className="text-input"
                  value={vocabDraft.expression}
                  onChange={(event) =>
                    setVocabDraft((current) => ({
                      ...current,
                      expression: event.target.value
                    }))
                  }
                  placeholder="one blocker"
                />
              </label>
              <label className="field">
                <span>Japanese meaning</span>
                <input
                  className="text-input"
                  value={vocabDraft.meaningJa}
                  onChange={(event) =>
                    setVocabDraft((current) => ({
                      ...current,
                      meaningJa: event.target.value
                    }))
                  }
                  placeholder="今つまずいている課題を一つ"
                />
              </label>
              <label className="field">
                <span>Note</span>
                <textarea
                  value={vocabDraft.note}
                  onChange={(event) =>
                    setVocabDraft((current) => ({
                      ...current,
                      note: event.target.value
                    }))
                  }
                  rows={4}
                  placeholder="どんな場面で使うか、何がわかりにくかったか。"
                />
              </label>
              <label className="field">
                <span>Example</span>
                <textarea
                  value={vocabDraft.example}
                  onChange={(event) =>
                    setVocabDraft((current) => ({
                      ...current,
                      example: event.target.value
                    }))
                  }
                  rows={4}
                  placeholder="My one blocker is that the staging result is still inconsistent."
                />
              </label>
              <button
                className="primary"
                disabled={!vocabDraft.expression.trim() || !vocabDraft.meaningJa.trim() || savingVocab}
                onClick={() => void handleAddVocab()}
              >
                {savingVocab ? "Saving..." : "Save vocab note"}
              </button>
            </article>

            <article className="panel">
              <p className="eyebrow">Review loop</p>
              <h3>Saved expressions</h3>
              <label className="field compact-field">
                <span>Sort</span>
                <select
                  className="text-input"
                  value={vocabSort}
                  onChange={(event) =>
                    setVocabSort(event.target.value as "weakest" | "strongest" | "recent")
                  }
                >
                  <option value="weakest">Lowest retention first</option>
                  <option value="strongest">Highest retention first</option>
                  <option value="recent">Recently reviewed first</option>
                </select>
              </label>
              <div className="stack">
                {sortedVocabNotes.length === 0 ? (
                  <p>No vocab notes yet.</p>
                ) : (
                  sortedVocabNotes.map((note) => (
                    <div key={note.id} className="history-card vocab-card">
                      <div>
                        <strong>{note.expression}</strong>
                        <p>{note.meaningJa}</p>
                        {note.note ? <p>{note.note}</p> : null}
                        {note.example ? <p>Example: {note.example}</p> : null}
                        <p>
                          Retention: {Math.round(note.retentionScore * 100)}% | Last result:{" "}
                          {note.lastResult}
                        </p>
                        <p>
                          Reviews: {note.reviewCount} | Last:{" "}
                          {note.lastReviewedAt
                            ? new Date(note.lastReviewedAt).toLocaleString()
                            : "not reviewed yet"}
                        </p>
                      </div>
                      <div className="vocab-actions">
                        <button
                          className="secondary"
                          onClick={() => void handleVocabReview(note.id, "reviewed")}
                        >
                          Reviewed
                        </button>
                        <button
                          className="secondary danger-soft"
                          onClick={() => void handleVocabReview(note.id, "still_hard")}
                        >
                          Still hard
                        </button>
                        <button
                          className="secondary success-soft"
                          onClick={() => void handleVocabReview(note.id, "got_it")}
                        >
                          Got it
                        </button>
                        <button
                          className="secondary danger-soft"
                          onClick={() => void handleDeleteVocab(note.id, note.expression)}
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </article>
          </section>
        ) : null}
      </main>
    </div>
  );
}
