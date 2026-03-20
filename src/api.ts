import { invoke } from "@tauri-apps/api/core";
import type {
  AddVocabNoteInput,
  AppConfig,
  AppConfigInput,
  ConversationSessionRecord,
  ConversationSubmission,
  HomePayload,
  OllamaModelInfo,
  ScenarioProgressRecord,
  VocabNoteRecord,
  VocabReviewOutcome,
  WritingEvaluation,
  WritingSessionRecord,
  WritingSubmission
} from "./types";

type InvokeFn = typeof invoke;

const hasTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const browserFallback: {
  config: AppConfigInput;
  home: HomePayload;
  conversationSessions: ConversationSessionRecord[];
  sessions: WritingSessionRecord[];
  vocabNotes: VocabNoteRecord[];
} = {
  config: {
    provider: "openai",
    openaiModel: "gpt-5-mini",
    openaiApiBase: "https://api.openai.com/v1",
    ollamaModel: "mistral:latest",
    ollamaApiBase: "http://127.0.0.1:11434"
  },
  home: {
    tasks: [
      {
        id: 1,
        scenarioId: "conv-standup-cache",
        taskDate: new Date().toISOString().slice(0, 10),
        taskType: "conversation",
        title: "Standup update: progress and blocker",
        prompt:
          "You are giving a standup update. Cover progress on the cache rollout, one blocker, and the next action in 45 seconds.",
        scenarioTag: "meeting",
        status: "pending",
        estimatedMinutes: 10
      },
      {
        id: 2,
        scenarioId: "write-followup-design-review",
        taskDate: new Date().toISOString().slice(0, 10),
        taskType: "writing",
        title: "Follow-up email after design review",
        prompt:
          "Write a short follow-up email after a design review. Confirm the decision, mention one risk, and ask for the next action owner.",
        scenarioTag: "email",
        status: "pending",
        estimatedMinutes: 15
      },
      {
        id: 3,
        scenarioId: "srs-meeting-phrases-a",
        taskDate: new Date().toISOString().slice(0, 10),
        taskType: "srs",
        title: "Review 15 meeting phrases",
        prompt: "Review 15 meeting phrases for alignment, trade-offs, and next actions.",
        scenarioTag: "srs",
        status: "pending",
        estimatedMinutes: 10
      }
    ],
    weaknesses: [
      { tagName: "conclusion-first", count: 4, lastSeenAt: new Date().toISOString() },
      { tagName: "conciseness", count: 3, lastSeenAt: new Date().toISOString() }
    ],
    dashboard: {
      totalStudyMinutes: 75,
      conversationCount: 2,
      writingCount: 3,
      averageWritingScore: 3.8,
      averageSpeakingScore: 3.2
    }
  },
  conversationSessions: [],
  sessions: [],
  vocabNotes: [
    {
      id: 1,
      expression: "one blocker",
      meaningJa: "今つまずいている課題を一つ",
      note: "スタンドアップで progress / one blocker / next action の型で使う。",
      example: "My one blocker is that the staging result is still inconsistent.",
      reviewCount: 0,
      retentionScore: 0.35,
      lastResult: "new",
      createdAt: new Date().toISOString(),
      lastReviewedAt: null
    }
  ]
};

const evalWritingFallback = (submission: WritingSubmission): WritingEvaluation => {
  const trimmed = submission.draft.trim();
  const corrected = trimmed.replace(/\bi want to\b/gi, "I would like to");
  return {
    correctedDraft: corrected || "Could we align on the next steps for the cache rollout by Friday?",
    shortenedDraft:
      corrected.length > 160
        ? `${corrected.slice(0, 157).trimEnd()}...`
        : corrected || "Could we align on next steps for the cache rollout by Friday?",
    feedbackSummary:
      "Lead with the request, keep the reason brief, and end with a clear next step.",
    toneLabel: "professional-direct",
    weaknesses: ["conciseness", "request-clarity"],
    scoreClarity: 4,
    scoreConciseness: 3,
    scoreTone: 4,
    scoreBusiness: 4,
    scoreGrammar: 4
  };
};

async function invokeOrFallback<T>(
  command: string,
  args: Record<string, unknown>,
  fallback: () => T | Promise<T>
): Promise<T> {
  if (!hasTauri()) {
    return fallback();
  }
  return (invoke as InvokeFn)(command, args) as Promise<T>;
}

export async function loadHome(): Promise<HomePayload> {
  return invokeOrFallback("get_home_payload", {}, () => browserFallback.home);
}

export async function refreshDailyTasks(): Promise<HomePayload> {
  return invokeOrFallback("refresh_daily_tasks", {}, () => {
    const byType = new Map(browserFallback.home.tasks.map((task) => [task.taskType, task]));
    const nextConversation = {
      ...byType.get("conversation")!,
      scenarioId: "conv-design-review-tradeoff",
      title: "Design review: explain the trade-off",
      prompt:
        "You are in a design review. Recommend one option, explain the trade-off, and ask whether the team agrees on the next step."
    };
    const nextWriting = {
      ...byType.get("writing")!,
      scenarioId: "write-delay-risk-update",
      title: "Delay update with risk and recovery plan",
      prompt:
        "Write a short update email that explains a delay, names one risk, and proposes a recovery plan with the next checkpoint."
    };
    const nextSrs = {
      ...byType.get("srs")!,
      scenarioId: "srs-standup-phrases-a",
      title: "Review standup phrases",
      prompt: "Review 15 standup phrases for status, blockers, and next actions."
    };
    browserFallback.home.tasks = browserFallback.home.tasks.map((task) => {
      if (task.status === "completed") {
        return task;
      }
      if (task.taskType === "conversation") {
        return nextConversation;
      }
      if (task.taskType === "writing") {
        return nextWriting;
      }
      if (task.taskType === "srs") {
        return nextSrs;
      }
      return task;
    });
    return browserFallback.home;
  });
}

export async function submitWriting(
  submission: WritingSubmission
): Promise<WritingSessionRecord> {
  return invokeOrFallback(
    "submit_writing_session",
    { payload: submission },
    async () => {
      const evaluation = evalWritingFallback(submission);
      const record: WritingSessionRecord = {
        id: browserFallback.sessions.length + 1,
        startedAt: new Date().toISOString(),
        taskKind: "writing",
        sourcePrompt: submission.prompt,
        userDraft: submission.draft,
        correctedDraft: evaluation.correctedDraft,
        shortenedDraft: evaluation.shortenedDraft,
        feedbackSummary: evaluation.feedbackSummary,
        scoreClarity: evaluation.scoreClarity,
        scoreConciseness: evaluation.scoreConciseness,
        scoreTone: evaluation.scoreTone,
        scoreBusiness: evaluation.scoreBusiness,
        scoreGrammar: evaluation.scoreGrammar,
        evaluator: "browser-fallback",
        warningMessage: "Running without the desktop backend."
      };
      browserFallback.sessions = [record, ...browserFallback.sessions];
      browserFallback.home.tasks = browserFallback.home.tasks.map((task) =>
        task.id === submission.taskId ? { ...task, status: "completed" } : task
      );
      browserFallback.home.dashboard.writingCount += 1;
      browserFallback.home.dashboard.totalStudyMinutes += 15;
      browserFallback.home.dashboard.averageWritingScore =
        (record.scoreClarity +
          record.scoreConciseness +
          record.scoreTone +
          record.scoreBusiness +
          record.scoreGrammar) /
        5;
      return record;
    }
  );
}

export async function submitConversation(
  submission: ConversationSubmission
): Promise<ConversationSessionRecord> {
  return invokeOrFallback("submit_conversation_session", { payload: submission }, async () => {
    const record = {
      id: Date.now(),
      scenarioId: submission.scenarioId ?? "",
      startedAt: new Date().toISOString(),
      scenarioType: "meeting",
      roleType: "engineer",
      objective: submission.prompt,
      transcript: submission.responseText,
      improvedTranscript:
        "Today's status is on track. My blocker is one inconsistent staging result. Next, I will verify the logs and share an update this afternoon.",
      feedbackSummary:
        "Start with the decision, keep the reason shorter, and end with the next action.",
      priorityFix: "Give the answer in conclusion-first order.",
      retryPrompt: "Answer again in 45 seconds starting with: The recommendation is...",
      scoreStructure: 3,
      scoreSpeed: 3,
      scoreBusiness: 4,
      scoreParaphrase: 3,
      scoreIntelligibility: 3,
      evaluator: "browser-fallback",
      warningMessage: "Running without the desktop backend."
    };
    browserFallback.conversationSessions = [record, ...browserFallback.conversationSessions];
    return record;
  });
}

export async function loadWritingHistory(): Promise<WritingSessionRecord[]> {
  return invokeOrFallback("list_recent_writing_sessions", {}, () => browserFallback.sessions);
}

export async function loadConversationHistory(): Promise<ConversationSessionRecord[]> {
  return invokeOrFallback(
    "list_recent_conversation_sessions",
    {},
    () => browserFallback.conversationSessions
  );
}

export async function loadScenarioProgress(): Promise<ScenarioProgressRecord[]> {
  return invokeOrFallback("list_scenario_progress", {}, () => []);
}

export async function loadVocabNotes(): Promise<VocabNoteRecord[]> {
  return invokeOrFallback("list_vocab_notes", {}, () => browserFallback.vocabNotes);
}

export async function addVocabNote(input: AddVocabNoteInput): Promise<VocabNoteRecord> {
  return invokeOrFallback("add_vocab_note", { payload: input }, () => {
    const note: VocabNoteRecord = {
      id: Date.now(),
      expression: input.expression,
      meaningJa: input.meaningJa,
      note: input.note,
      example: input.example,
      reviewCount: 0,
      retentionScore: 0.35,
      lastResult: "new",
      createdAt: new Date().toISOString(),
      lastReviewedAt: null
    };
    browserFallback.vocabNotes = [note, ...browserFallback.vocabNotes];
    return note;
  });
}

function nextRetentionScore(current: number, outcome: VocabReviewOutcome): number {
  switch (outcome) {
    case "still_hard":
      return Math.max(0, current * 0.6);
    case "got_it":
      return Math.min(1, current + 0.22);
    case "reviewed":
    default:
      return Math.min(1, current + 0.08);
  }
}

export async function reviewVocabNote(
  noteId: number,
  outcome: VocabReviewOutcome
): Promise<VocabNoteRecord> {
  return invokeOrFallback("review_vocab_note", { payload: { noteId, outcome } }, () => {
    const nextReviewedAt = new Date().toISOString();
    const existing = browserFallback.vocabNotes.find((item) => item.id === noteId);
    if (!existing) {
      throw new Error("Vocab note not found.");
    }
    const updated: VocabNoteRecord = {
      ...existing,
      reviewCount: existing.reviewCount + 1,
      retentionScore: nextRetentionScore(existing.retentionScore, outcome),
      lastResult: outcome,
      lastReviewedAt: nextReviewedAt
    };
    browserFallback.vocabNotes = browserFallback.vocabNotes.map((item) =>
      item.id === noteId ? updated : item
    );
    browserFallback.vocabNotes.sort((a, b) => a.retentionScore - b.retentionScore);
    return updated;
  });
}

export async function deleteVocabNote(noteId: number): Promise<void> {
  return invokeOrFallback("delete_vocab_note", { noteId }, () => {
    browserFallback.vocabNotes = browserFallback.vocabNotes.filter((item) => item.id !== noteId);
  });
}

export async function loadAppConfig(): Promise<AppConfig> {
  return invokeOrFallback("get_app_config", {}, () => ({
    provider: browserFallback.config.provider,
    hasOpenAiApiKey: false,
    openaiModel: browserFallback.config.openaiModel,
    openaiApiBase: browserFallback.config.openaiApiBase,
    ollamaModel: browserFallback.config.ollamaModel,
    ollamaApiBase: browserFallback.config.ollamaApiBase
  }));
}

export async function saveAppConfig(input: AppConfigInput): Promise<AppConfig> {
  return invokeOrFallback(
    "save_app_config",
    { payload: input },
    () => {
      browserFallback.config = input;
      return {
        provider: input.provider,
        hasOpenAiApiKey: false,
        openaiModel: input.openaiModel,
        openaiApiBase: input.openaiApiBase,
        ollamaModel: input.ollamaModel,
        ollamaApiBase: input.ollamaApiBase
      };
    }
  );
}

export async function listOllamaModels(): Promise<OllamaModelInfo[]> {
  return invokeOrFallback("list_ollama_models", {}, () => [
    { name: browserFallback.config.ollamaModel }
  ]);
}
