# naIEng

`naIEng` is a personal English training app for `nab`, designed for business English growth as a software engineer.

It is built with:

- `Tauri`
- `Rust`
- `React`
- `TypeScript`
- `SQLite`

The app is organized around a low-friction daily loop:

- one conversation drill
- one writing drill
- one vocab review loop

## Screenshots

### Home

![Home overview](docs/screenshots/home-overview.svg)

### Conversation review

![Conversation review](docs/screenshots/conversation-review.svg)

### Writing review

![Writing review](docs/screenshots/writing-review.svg)

### Vocab notebook

![Vocab notebook](docs/screenshots/vocab-notebook.svg)

## Current Features

### Daily workflow

- Daily tasks are generated from a stock of `50 scenarios`
- The app keeps one `conversation`, one `writing`, and one `srs` task for the day
- `Refresh Daily` replaces today's tasks with unseen scenarios first
- The app tries to avoid the same `scenario_id` within the same day
- If the scenario pool is exhausted, it reuses older scenarios

### Conversation drill

- Text-based business speaking practice
- OpenAI or Ollama evaluation
- Local fallback evaluation if the provider fails
- Review shows:
  - `Your version`
  - `Improved version`
  - `Summary`
  - `Priority fix`
  - `Retry prompt`
- Conversation history is saved
- Repeated attempts on the same `scenario_id` are tracked

### Writing drill

- Business email and short business writing practice
- OpenAI or Ollama evaluation
- Local fallback evaluation if the provider fails
- Review shows:
  - `Your version`
  - `Corrected`
  - `Shorter version`
  - `Summary`
- Writing history is saved
- Repeated attempts on the same `scenario_id` are tracked

### Vocab notebook

- Save unknown words and phrases manually
- Seeded starter entries such as:
  - `one blocker`
  - `next action`
  - `trade-off`
- Each vocab card supports:
  - `Reviewed`
  - `Still hard`
  - `Got it`
  - `Delete`
- Each card has a retention score
- Vocab can be sorted by:
  - lowest retention first
  - highest retention first
  - recently reviewed first

### Pick text into Vocab

- In conversation review, select part of `Improved version` and send it to Vocab
- In writing review, select part of `Corrected` or `Shorter version` and send it to Vocab
- Selected text opens the `Vocab` screen and fills the `Expression` field
- This avoids dumping the full sentence directly into the vocab list

### Progress tracking

- Recent writing sessions
- Recent conversation sessions
- Scenario progress by `scenario_id`
- Weakness tags
- Study minutes
- Writing average score
- Speaking average score

## AI Provider Setup

The app supports:

- `OpenAI`
- `Ollama`

### OpenAI

OpenAI API keys are **not saved in the app**.

Set the key in the current PowerShell session before launch:

```powershell
$env:OPENAI_API_KEY="your_api_key"
```

### Ollama

The app can use a local Ollama server such as:

- `mistral:latest`
- `llama3:latest`
- `gemma3:4b`

Default base URL:

```text
http://127.0.0.1:11434
```

## Run

```powershell
cd S:\tools\codex\naIEng
npm.cmd run tauri:dev
```

## Build Check

```powershell
cd S:\tools\codex\naIEng\src-tauri
cargo check
```

```powershell
cd S:\tools\codex\naIEng
npm.cmd run build
```

## Notes

- Conversation input is currently transcript-based, not live audio recording
- `Refresh Daily` resets today's tasks to new scenarios and marks them pending again
- The app is optimized for local personal use, not multi-user deployment

## Next Good Improvements

- audio recording + ASR for conversation
- one-click phrase extraction from AI feedback
- better spaced repetition scheduling for vocab
- detailed scenario timeline view
- finer-grained phrase-level coaching
