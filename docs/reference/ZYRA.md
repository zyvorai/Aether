---
hero:
  eyebrow: REFERENCE
  title: Zyra — AI Infrastructure Operating Layer
  tone: rust
---

Zyra is Aether's first-class AI assistant — not a chatbot, but an ambient intelligence layer present throughout the platform.

## Features

- **Multi-LLM** — OpenAI, Anthropic, Gemini, xAI Grok, Azure, Ollama, vLLM, and OpenAI-compatible endpoints
- **Multi-agent** — Auto, Architect, DevOps, Kubernetes, Security, SRE, Cost, Observability, AI Engineer, Database Expert
- **Intelligent routing** — Task-class based provider and agent selection
- **Approval-gated actions** — Mutations require explicit user confirmation
- **Provider management** — Settings → AI Providers (`/settings/ai-providers`)
- **Prompt library** — `GET/POST /api/zyra/prompts`
- **Agent marketplace** — `GET /api/zyra/marketplace`
- **Scoped memory** — Global, project, team, and session scopes

## API

Primary routes use `/api/zyra/*`. Legacy `/api/copilot/*` routes remain as deprecated aliases.

| Endpoint | Description |
|----------|-------------|
| `POST /api/zyra/chat` | Chat with tool calling |
| `GET /api/zyra/insights` | Ambient context bar insights |
| `GET/POST /api/zyra/providers` | Provider CRUD |
| `GET /api/zyra/agents` | Specialist agent personas |
| `GET /api/zyra/marketplace` | Installable agent catalog |

## Configuration

Providers can be configured via the dashboard or environment variables (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `AETHER_OLLAMA_URL`). API keys stored via the UI are encrypted using the secrets store.

Enable air-gapped mode in AI Providers settings to restrict Zyra to local inference backends only.
