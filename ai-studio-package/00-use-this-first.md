# Aerie AI Studio Package - Use This First

This folder contains the prompt package for asking Google AI Studio / Gemini 3.1 Pro to build **Aerie**, a mobile-first CPAP/PAP data analysis PWA.

Use these files in this order.

## 1. AI Studio Settings

Recommended initial build settings:

- Model: Gemini 3.1 Pro.
- Temperature: `1.0`.
- Thinking: `high` for the first full build.
- Max output tokens: at least `32768`; use `65536` if available.
- Build target: React / TypeScript web app.
- Enable URL context or grounding only when asking Gemini to verify current external docs.
- Use Build mode for the generated app, but keep this package in the prompt/context so the model has strict architecture and safety boundaries.

Why: Google's current AI Studio docs say Build mode creates a web app, React is the default, generated code can be viewed in the Code tab, projects can be pushed to GitHub or deployed to Cloud Run, and shared apps expose code to viewers. The full-stack docs say AI Studio apps can include a Node.js server-side component, use npm packages, manage secrets, and use Firebase Authentication / Google Sign-in. The docs also warn against client-side API key exposure and recommend keeping key-bearing logic server-side.

Official docs to keep handy:

- https://ai.google.dev/gemini-api/docs/aistudio-build-mode
- https://ai.google.dev/gemini-api/docs/aistudio-fullstack
- https://ai.google.dev/gemini-api/docs/gemini-3
- https://ai.google.dev/gemini-api/docs/downloads
- https://firebase.google.com/docs/auth/web/google-signin
- https://www.edfplus.info/specs/edf.html
- https://www.edfplus.info/specs/edfplus.html

## 2. Paste System Instructions

Paste the full contents of:

`01-system-instructions.md`

into AI Studio's system instructions or equivalent persistent instruction field.

These instructions intentionally change Gemini's behavior: complete files, no placeholders, server-side secrets, deterministic analysis owns the math, Gemini explains only structured outputs, iOS PWA constraints are required, and the app must not overclaim medical meaning.

## 3. Attach Context Documents

Attach or paste these as reference context:

- `context/03-product-safety-brief.md`
- `context/04-ui-style-guide.md`
- `context/05-analysis-contract.md`
- `context/06-edf-channel-reference.md`

Optional but useful for later iterations:

- `context/07-review-and-repair-prompts.md`

## 4. Paste The Master Build Prompt

Paste:

`02-master-build-prompt.md`

as the user prompt.

If AI Studio truncates or becomes lazy, start a fresh chat. Do not keep patching the same broken generation after three failed repair loops.

## 5. First Build Acceptance Criteria

The first generated project should have:

- Google OAuth or Firebase Auth gate before upload.
- A scope warning before file upload.
- Upload UI for EDF, EDF+, CRC, CSV, and TSV files.
- Server-side analysis endpoint; no Gemini API key or privileged secret in client code.
- Deterministic EDF parser and analysis engine.
- Structured JSON output from analysis.
- Gemini explanation layer that consumes structured JSON only.
- Aerie visual style: compact clinical-engineering instrument, mobile-first, iPhone safe-area aware.
- Disabled future Lab mode.
- No medical-advice language.
- No demo constants pretending to be real analysis.

## 6. Human Review Checklist

After Gemini generates the app, check these before trusting it:

- Search the client bundle for real API keys or secrets.
- Search for hardcoded fake analysis values.
- Confirm upload disclosure says server-side processing may occur.
- Confirm sign-in is required before upload.
- Confirm deterministic code computes metrics before Gemini writes summaries.
- Confirm all unsafe language is absent: "diagnose", "prescribe", "you should change", "set your CPAP to".
- Confirm the app is usable on iPhone 14 Pro Max dimensions: 430 x 932 CSS px, safe-area insets, 44 px touch targets.
