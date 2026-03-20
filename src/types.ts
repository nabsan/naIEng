export type TaskType = "conversation" | "writing" | "srs";

export type TaskStatus = "pending" | "completed";

export interface DailyTask {
  id: number;
  scenarioId: string;
  taskDate: string;
  taskType: TaskType;
  title: string;
  prompt: string;
  scenarioTag: string;
  status: TaskStatus;
  estimatedMinutes: number;
}

export interface WeaknessTag {
  tagName: string;
  count: number;
  lastSeenAt: string;
}

export interface DashboardSummary {
  totalStudyMinutes: number;
  conversationCount: number;
  writingCount: number;
  averageWritingScore: number;
  averageSpeakingScore: number;
}

export interface WritingEvaluation {
  correctedDraft: string;
  shortenedDraft: string;
  feedbackSummary: string;
  toneLabel: string;
  weaknesses: string[];
  scoreClarity: number;
  scoreConciseness: number;
  scoreTone: number;
  scoreBusiness: number;
  scoreGrammar: number;
}

export interface WritingSubmission {
  taskId: number;
  scenarioId?: string;
  prompt: string;
  draft: string;
}

export interface WritingSessionRecord {
  id: number;
  startedAt: string;
  taskKind: string;
  sourcePrompt: string;
  userDraft: string;
  correctedDraft: string;
  shortenedDraft: string;
  feedbackSummary: string;
  scoreClarity: number;
  scoreConciseness: number;
  scoreTone: number;
  scoreBusiness: number;
  scoreGrammar: number;
  evaluator: string;
  warningMessage?: string | null;
}

export interface ConversationSubmission {
  taskId: number;
  scenarioId?: string;
  prompt: string;
  responseText: string;
}

export interface ConversationSessionRecord {
  id: number;
  scenarioId: string;
  startedAt: string;
  scenarioType: string;
  roleType: string;
  objective: string;
  transcript: string;
  improvedTranscript: string;
  feedbackSummary: string;
  priorityFix: string;
  retryPrompt: string;
  scoreStructure: number;
  scoreSpeed: number;
  scoreBusiness: number;
  scoreParaphrase: number;
  scoreIntelligibility: number;
  evaluator: string;
  warningMessage?: string | null;
}

export interface ScenarioProgressRecord {
  scenarioId: string;
  taskType: string;
  title: string;
  attempts: number;
  latestScore: number;
  averageScore: number;
  firstAttemptAt: string;
  lastAttemptAt: string;
}

export interface VocabNoteRecord {
  id: number;
  expression: string;
  meaningJa: string;
  note: string;
  example: string;
  reviewCount: number;
  retentionScore: number;
  lastResult: string;
  createdAt: string;
  lastReviewedAt?: string | null;
}

export interface AddVocabNoteInput {
  expression: string;
  meaningJa: string;
  note: string;
  example: string;
}

export type VocabReviewOutcome = "reviewed" | "still_hard" | "got_it";

export interface HomePayload {
  tasks: DailyTask[];
  weaknesses: WeaknessTag[];
  dashboard: DashboardSummary;
}

export interface AppConfig {
  provider: "openai" | "ollama";
  hasOpenAiApiKey: boolean;
  openaiModel: string;
  openaiApiBase: string;
  ollamaModel: string;
  ollamaApiBase: string;
}

export interface AppConfigInput {
  provider: "openai" | "ollama";
  openaiModel: string;
  openaiApiBase: string;
  ollamaModel: string;
  ollamaApiBase: string;
}

export interface OllamaModelInfo {
  name: string;
  sizeBytes?: number | null;
  modifiedAt?: string | null;
}
