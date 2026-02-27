# Git Workflow

## Branching Strategy
- `main`: production-ready only
- `develop`: integration branch
- `feat/*`, `fix/*`, `chore/*`: short-lived working branches from `develop`

## Rules
1. Never push feature changes directly to `main`
2. Open PR: `feature -> develop`
3. Release PR: `develop -> main`
4. Require at least one review before merge

## Suggested Branch Protection (GitHub)
- Protect `main` and `develop`
- Require PRs
- Require status checks (CI)
- Disable force-push on protected branches
