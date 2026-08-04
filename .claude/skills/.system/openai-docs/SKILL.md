---
name: openai-docs
description: Provide authoritative, current guidance from OpenAI developer docs. Use when the user asks about OpenAI products, APIs, model selection, model upgrades, prompt-upgrade guidance, or needs up-to-date official documentation with citations. Complements the claude-api skill which covers Anthropic/Claude models.
---

# OpenAI Docs (Claude Code Edition)

Provide authoritative, current guidance from OpenAI developer docs using web search and the OpenAI developer documentation site. This skill owns model selection, API model migration, and prompt-upgrade guidance for OpenAI products.

## Source Priority

1. For model-selection, "latest model", or default-model questions, fetch `https://developers.openai.com/api/docs/guides/latest-model.md` first.
2. If unavailable, use `references/latest-model.md` as fallback.
3. For API reference, schema, or parameter questions, use web search restricted to `developers.openai.com` and `platform.openai.com`.
4. For general OpenAI docs, search and fetch from official OpenAI domains.

## OpenAI Product Snapshots

1. **Chat Completions API**: Generate a model response from a list of messages comprising a conversation.
2. **Responses API**: A unified endpoint for stateful, multimodal, tool-using interactions in agentic workflows.
3. **Agents SDK**: A toolkit for building agentic apps with tools, handoffs, streaming, and tracing.
4. **Realtime API**: Low-latency, multimodal experiences including natural speech-to-speech conversations.
5. **Assistants API**: Persistent assistants with file search, code interpreter, and function calling.

## Model Selection Workflow

1. For "latest/current/default" model questions:
   - Fetch `https://developers.openai.com/api/docs/guides/latest-model.md`
   - Find the latest model ID and explicit migration or prompt-guidance links
   - Prefer explicit links from the latest-model page over derived URLs
2. For explicit named-model requests (e.g., "migrate to GPT-5.4"), preserve the requested model target. Mention newer guidance only as optional.
3. For dynamic latest/current/default upgrades, run `node scripts/resolve-latest-model-info.js`, then fetch both returned guide URLs.
4. If remote docs are unavailable, use bundled `references/latest-model.md` and disclose the fallback.

## Model Upgrade Rules

- Keep changes narrow: update active OpenAI API model defaults and directly related prompts only when safe.
- Leave historical docs, examples, eval baselines, fixtures, provider comparisons, pricing tables, and ambiguous older model usage unchanged unless the user explicitly asks.
- Keep SDK, tooling, IDE, plugin, shell, auth, and provider-environment migrations out of a model-and-prompt upgrade unless explicitly requested.
- If an upgrade needs API-surface changes, schema rewiring, or implementation work beyond model-string replacement and prompt edits, report it as blocked or confirmation-needed.

## Workflow

1. Clarify whether the request is general docs lookup, model selection, model-string upgrade, or prompt-upgrade guidance.
2. For model-selection or upgrade requests, prefer current remote docs over bundled references.
3. For general docs lookup, search with a precise query, fetch the best page, and answer with concise citations.
4. Use web search restricted to `developers.openai.com` and `platform.openai.com` for official guidance.

## Reference Map

- `https://developers.openai.com/api/docs/guides/latest-model.md` → current model-selection guidance
- `references/latest-model.md` → bundled fallback for model-selection questions
- `references/upgrade-guide.md` → bundled fallback for model upgrade planning
- `references/prompting-guide.md` → bundled fallback for prompt rewrites
- `scripts/resolve-latest-model-info.js` → resolve latest model info programmatically

## Quality Rules

- Treat OpenAI docs as the source of truth; avoid speculation.
- Keep migration changes narrow and behavior-preserving.
- Prefer prompt-only upgrades when possible.
- Avoid inventing pricing, availability, parameters, or API changes.
- Keep quotes short and within policy limits; prefer paraphrase with citations.
- If multiple pages differ, call out the difference and cite both.
- If docs do not cover the user's need, say so and offer next steps.
