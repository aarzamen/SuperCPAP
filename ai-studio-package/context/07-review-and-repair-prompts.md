# Review And Repair Prompts For Gemini

Use these after the first app generation. Keep prompts focused. If Gemini fails three times on the same issue, start a fresh chat with the current code and the relevant prompt.

## Architecture Review

```text
DEEP review this project for architecture compliance with the Aerie prompt package.

Check:
- OAuth gate exists before upload.
- Upload warning is mandatory.
- Server-side routes handle privileged analysis/Gemini calls.
- Client code contains no real secrets.
- Deterministic analysis produces structured JSON before Gemini explanation.
- Gemini explanation uses only structured JSON.
- No fake demo constants are presented as real analysis.
- V1 Lab mode is disabled.

Return:
1. Critical blockers.
2. Important fixes.
3. Nice-to-have refinements.
4. Exact file changes required.
```

## Safety Copy Review

```text
DEEP audit all user-facing copy for medical-adjacent safety.

Remove or rewrite any phrase that implies diagnosis, prescription, treatment, certainty, or bypassing a clinician.

Allowed phrasing:
- supports considering
- compatible with
- insufficient signal
- discuss with clinician
- data quality limits this conclusion

Output complete changed files only.
```

## Parser Verification Prompt

```text
DEEP review the EDF parser and analysis engine.

Verify:
- EDF fixed header fields are parsed by byte offsets.
- Per-signal metadata arrays are parsed in EDF order.
- int16 samples are little-endian.
- digital-to-physical scaling is correct.
- record count -1 or header-only files do not crash.
- Flow.40ms, Press.40ms, MaskPress.2s, Leak.2s, SpO2.1s, Pulse.1s classify correctly.
- Missing or invalid SpO2 is reported as unavailable.

Add or repair deterministic tests. Output complete changed files only.
```

## iOS PWA Review

```text
DEEP review the PWA for iPhone 14 Pro Max and iOS Safari.

Check:
- viewport-fit=cover
- interactive-widget=resizes-content
- safe-area padding on fixed top/bottom UI
- 44px minimum touch targets
- inputs >= 16px font size
- no hover-only interactions
- install guidance for iOS
- manifest and Apple touch icon references
- service worker update strategy does not blindly skipWaiting under open tabs

Output complete changed files only.
```

## UI Polish Prompt

```text
Refine the UI to better match the Aerie reference style.

Preserve:
- off-white paper
- near-black ink
- cool gray rules
- one calm signal blue
- one sober amber warning
- monospace instrument labels
- thin chart lines
- numbered evidence rows
- disabled future Lab affordance

Avoid:
- AI gradients
- decorative blobs
- nested cards
- wellness copy
- donut charts for serious metrics

Output complete changed files only.
```

## Debugging Prompt

```text
The app has this error:

[paste exact error]

DEEP debug it systematically.

Do not rewrite unrelated files.
Identify root cause, then output complete changed files only.
Keep deterministic analysis boundaries intact.
```

