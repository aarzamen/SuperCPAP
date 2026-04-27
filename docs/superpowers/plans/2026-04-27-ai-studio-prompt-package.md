# AI Studio Prompt Package Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a paste-ready Google AI Studio prompt package for building Aerie, a mobile-first CPAP/PAP data analysis PWA with OAuth, server-side upload analysis, deterministic metrics, and evidence-bounded Gemini explanations.

**Architecture:** The package is documentation-first: a setup guide, a system-instructions file that changes Gemini's coding behavior, a master build prompt, and focused context attachments for safety, UI, analysis contracts, EDF channel mapping, and review/repair prompts. The package avoids app implementation code except for interface contracts and deterministic behavior requirements that the generated app must satisfy.

**Tech Stack:** Google AI Studio Build mode, Gemini 3.1 Pro, React/TypeScript PWA, Firebase Auth or equivalent Google OAuth, server-side Node/TypeScript analysis endpoints, EDF/EDF+ parsing, JSON evidence contract.

---

### Task 1: Create Package Directory And Overview

**Files:**
- Create: `/Users/ama/SuperCPAP/ai-studio-package/00-use-this-first.md`

- [x] **Step 1: Define setup order**

Document the exact order for using the files in AI Studio: settings first, system instructions second, attach context docs third, paste master build prompt last.

- [x] **Step 2: Include source-grounded constraints**

Include current official constraints from Google AI Studio Build mode: React default, GitHub/Cloud Run export, code visibility for shared apps, API key exposure warning, and server-side key logic.

### Task 2: Create Coding Agent System Instructions

**Files:**
- Create: `/Users/ama/SuperCPAP/ai-studio-package/01-system-instructions.md`

- [x] **Step 1: Define the agent persona briefly**

Use a compact senior full-stack PWA engineer role without excessive persona text.

- [x] **Step 2: Add behavior-changing rules**

Add rules for complete files, no placeholders, use official SDKs, server-side secrets, deterministic analysis boundaries, mobile-first iOS PWA behavior, and medical-adjacent safety language.

### Task 3: Create Master Build Prompt

**Files:**
- Create: `/Users/ama/SuperCPAP/ai-studio-package/02-master-build-prompt.md`

- [x] **Step 1: Specify product and architecture**

Describe Aerie, upload analysis, OAuth gate, server-side compute, deterministic analysis, evidence JSON, and Gemini explanation limits.

- [x] **Step 2: Specify required generated project**

Require a complete React/TypeScript PWA project with explicit file tree, full files, tests or verification harness where feasible, and no fragments.

- [x] **Step 3: Include terminal output constraints**

Place final hard constraints at the end of the prompt so Gemini is less likely to drop them.

### Task 4: Create Context Attachments

**Files:**
- Create: `/Users/ama/SuperCPAP/ai-studio-package/context/03-product-safety-brief.md`
- Create: `/Users/ama/SuperCPAP/ai-studio-package/context/04-ui-style-guide.md`
- Create: `/Users/ama/SuperCPAP/ai-studio-package/context/05-analysis-contract.md`
- Create: `/Users/ama/SuperCPAP/ai-studio-package/context/06-edf-channel-reference.md`
- Create: `/Users/ama/SuperCPAP/ai-studio-package/context/07-review-and-repair-prompts.md`

- [x] **Step 1: Product and safety**

Capture no-PII warnings, server-side processing disclosure, not-medical-advice boundaries, and acceptable titration phrasing.

- [x] **Step 2: UI style guide**

Extract visual rules from the Claude Aerie standalone prototype and translate them into implementation constraints.

- [x] **Step 3: Analysis contract**

Define deterministic JSON schemas for uploads, parsed sessions, metric summaries, evidence items, findings, and explanation inputs.

- [x] **Step 4: EDF channel reference**

Record observed ResMed-style EDF channels and how they should map into analysis semantics.

- [x] **Step 5: Review prompts**

Provide follow-up prompts for Gemini review, debugging, UI polish, safety copy review, and parser/analysis verification.

### Task 5: Self-Review

**Files:**
- Modify: all files above

- [x] **Step 1: Placeholder scan**

Search for vague placeholders such as TBD, TODO, implement later, and remove them.

- [x] **Step 2: Scope consistency**

Confirm the package consistently says server-side processing is allowed and does not preserve the prototype's local-only disclosure.

- [x] **Step 3: Handoff**

Summarize the package and identify the primary files to paste into AI Studio.

