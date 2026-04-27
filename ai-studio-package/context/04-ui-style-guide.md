# Aerie UI Style Guide

Reference artifact:

`/Users/ama/Downloads/Aerie CPAP Suite (standalone).html`

## Feel

Aerie should feel like a compact clinical-engineering instrument. It should be precise, quiet, technical, and pleasant on an iPhone.

It should not feel like:

- a wellness dashboard
- a hospital EMR
- an AI SaaS landing page
- a MATLAB terminal
- a generic chart demo

## Visual Tokens

Use these as the approximate palette:

```css
:root {
  --paper: #F6F7F8;
  --paper-2: #ECEEF1;
  --card: #FFFFFF;
  --ink: #0B0D10;
  --ink-1: #1F242B;
  --ink-2: #5A6470;
  --ink-3: #8A95A2;
  --ink-4: #B6BEC8;
  --rule: rgba(11,13,16,0.10);
  --rule-2: rgba(11,13,16,0.05);
  --signal: #2D6FB0;
  --signal-2: #4D8FCB;
  --signal-3: #BBD4EC;
  --signal-4: #E5EFF8;
  --warn: #B8651A;
  --warn-2: #E8B985;
  --warn-3: #F6E3CB;
}
```

Typography:

- Use a modern readable sans font for body UI.
- Use monospace for instrument labels, values, IDs, and section markers.
- Keep letter spacing normal for body text; reserve spaced caps for small labels only.

## Interaction Model

Primary mobile flow:

1. Sign in.
2. Scope warning.
3. Upload.
4. Data quality.
5. Readout.
6. Evidence.
7. Detail views.
8. Disabled Lab.

Preserve:

- swipe hints
- page/dot progress
- compact top section labels
- numbered evidence rows
- detail drill-in pattern
- disabled future Lab affordance

## Charts

Charts should look like instruments:

- thin lines
- hard axes
- small tick labels
- threshold pins
- evidence markers
- leak co-traces where relevant
- restrained color
- no donut charts for serious metrics
- no decorative chart fills unless they encode uncertainty or range

Recommended charts:

- pressure trace
- pressure distribution histogram
- leak timeline
- SpO2/pulse strip if present
- trend sparkline
- channel availability table

## Mobile Requirements

- Design for 430 x 932 CSS px first.
- Respect `env(safe-area-inset-top)` and `env(safe-area-inset-bottom)`.
- No interactive controls inside the top unsafe area.
- Touch targets at least 44 x 44 px.
- Text must not overflow compact cards.
- Inputs must use at least 16 px font size.
- Do not rely on hover.

## Copy Tone

Use sober, skeptical language:

- "Data clean"
- "Insufficient signal"
- "Leak likely artifact"
- "Pressure support: moderate"
- "Discuss with clinician"

Avoid:

- "optimize your sleep"
- "biohack"
- "guaranteed"
- "AI doctor"
- "prescription"

