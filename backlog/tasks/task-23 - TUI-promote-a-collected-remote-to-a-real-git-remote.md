---
id: TASK-23
title: 'TUI: promote a collected remote to a real git remote'
status: To Do
assignee: []
created_date: '2026-10-22T03:05:00Z'
updated_date: '2026-10-22T03:06:00Z'
labels: []
dependencies: []
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Press p on a collected row in the remote picker to run 'git remote add <name> <url>' for it, so it becomes a real remote in .git/config (and then works with plain git, not only here). The name is asked in a one-line prompt, prefilled from the entry's label with the owner part dropped or joined (decided in design); a name git rejects or one already in use shows the reason and keeps the prompt open. The collected entry stays in its file: picker_remotes already hides an entry whose repo is a real remote, so the row simply moves up into the real group. Real remotes are refused with a note. The command is a git::Cmd, so it is printed and run from one place.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 p on a collected row asks for a name, then runs git remote add and the row becomes a real remote
- [ ] #2 p on a real remote does nothing but show a note
- [ ] #3 A name that is invalid or already used shows git's reason and keeps the prompt open
- [ ] #4 The collected entry is left in its file, and removing the real remote later brings the row back
- [ ] #5 Tests cover the prompt, the refusal and the Cmd's printed form
- [ ] #6 Keys, README and docs/usage.md are updated
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision: the name prompt is prefilled with the label's parts joined by a dash (netdev/net-next -> netdev-net-next), editable before Enter.
<!-- SECTION:NOTES:END -->
