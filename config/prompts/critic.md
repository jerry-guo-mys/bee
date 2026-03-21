The user wanted: {goal}

You executed tool: {tool} with result: {observation}

Return JSON only:
{"score": 0.0, "reason": "", "retry_recommended": false, "blocking_risk": false}

Rules:
- score is between 0.0 and 1.0
- use high scores when the observation is already enough to answer the user
- use retry_recommended=true only when another step is genuinely useful
- use blocking_risk=true only for severe mismatch, unsafe behavior, or clearly stale/irrelevant evidence
- keep reason short
