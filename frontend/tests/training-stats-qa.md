# Training statistics QA

Data source: user-entered completed sessions, working sets, date-stamped weight measurements, and optional per-session mood and body context. No demonstration rows are inserted into user storage. The visual-training.html file contains isolated QA fixtures only.

## Chart contracts

- Muscle distribution: horizontal bars, zero baseline, completed working sets grouped by the saved primary muscle. Count, total and share are visible. Diagram marker intensity is relative to the largest count, not a physiological measurement. Unmapped custom groups remain in the list.
- Body weight: observed weight measurements, actual date spacing, focused vertical scale disclosed beside the chart. Fewer than three observations render as exact-value cards. No fabricated zeroes or estimates for missing days; hover titles contain date and weight.
- Workout volume: categorical bars for up to twelve latest completed sessions in the selected date period, zero baseline, kg = sum(weight × repetitions), excludes warmup. Zero is legitimate for bodyweight-only sets.
- Mood: labeled observation cards, self-report 1–5, no imputation or causal claims. Average denominator is the number of valid reported moods, not all workouts.
- Relative strength: estimated Epley 1RM divided by body weight saved in the same session; no retroactive application of today's weight and no automatic load recommendations.

## Checks

DOM and calculation tests pass for profile saving, missing moods, body-context snapshots, date spacing, record restoration and the existing training lifecycle.

Screenshot-based visual QA blocked: the in-app browser failed to initialize; standalone headless browser capture was unavailable/blocked by runtime policy. No claim of screenshot verification is made. Source anatomy illustration was inspected directly.
