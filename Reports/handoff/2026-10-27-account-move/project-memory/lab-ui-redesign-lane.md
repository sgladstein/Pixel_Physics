---
name: lab-ui-redesign-lane
description: Lab interface redesign thread (2026-10-22): plan doc, what shipped on claude/project-thread-vclgr3 (steps 1, 3-5 in part), crisp text measured and not shipped.
metadata:
  type: project
  modified: 2026-10-22T06:17:04.623Z
---

Thread "Lab interface redesign" (cmsg_01Mei3dwkcdzPxa5Jmuqi2bBWCYJYtgoA5EkSBwJ92thhH), started 2026-10-22 05:38Z.
- Owner (Scotttt) asked: full design review of the lab UI (long lists, dense tables -> graphs, confusing menus, overlapping windows, windows that won't close), more modern. Then: "don't pause after the mockups", explore menus + graphs overnight, put recommendations in one plan with mockups, adversarial review, THEN implement.
- Plan doc: https://claude.ai/artifact/9yZGGLQ28DAvF5ZGqhUA3p ; markdown copy /mnt/project-files/lab-ui-redesign/plan.md ; mockups in /mnt/project-files/lab-ui-redesign/mockups/, today's screenshots in .../today/.
- Mockup kit: Python + PIL drawing in the lab's own 5x7 font (parsed from src/hud.rs) at 512x320, x2 upscale. Lived in the session scratchpad (not kept).
- Five steps: (1) layout rule + close/click/scroll-through fixes, (2) crisp 2x text layer spike, ships only if frame cost <= today, (3) bar page buttons + grouped MENU/VIEW, (4) charts in place of long lists, (5) look and feel.
- Branch claude/project-thread-vclgr3; PR #799 has merged, so it now targets main.
- Confirmed bugs: Ui::covers lacked BIOSPHERE+BATTLE (clicks went through); wheel always zooms; Esc quits; R rebuilds on one press; stats "TAB CLOSE" is text; PIXELS X4->X1 warning shows by default.

- Shipped on the branch (2026-10-22 overnight): window rule (dock/fold/X, Esc closes top, two-press quit and R), overlap census via render::paint_owner in examples/labui.rs (627,757 px -> 0 over 73 screens), grouped MENU, ANTS SUMMARY/GROUPS tabs with charts, PLANTS and HISTORY charts, ui::window_frame title band. Status lives in Reports/lab-ui-redesign-2026-10-22.md.
- PR #808 MERGED 2026-10-22 11:46Z as main d5bfcd4. Follow-ups start from main. Code review fixes in: BATTLE is now ui::Window::Battle; census 0 over 74 screens; labui sets ui.notes_rested.
- Round two (2026-10-22 afternoon, owner: do the to-dos yourself, one after another): PR #815 MERGED as main 3e38e0c (gene bars on CELL + SIDE BY SIDE, training charts, PLANTS/ANTS/HISTORY/BOX bar buttons replacing the 7 speed chips, RESET into MENU), handed to merge desk. Crisp text went to its own thread "Crisp lab text layer" and shipped as PR #817 (2x interface layer, src/bin/lab/layer.rs; Ui::draw split into draw_world_marks + draw_chrome; paint_stats_close/note take a Hud). Not done there: mixed case for prose, a 3x face.
- Owner lesson (2026-10-22): when asked for suggestions, don't propose new threads for small to-dos a finished lane can just do itself; only suggest a thread for work that truly runs in parallel.
- Crisp 2x text: measured, fails the free rule (+~3 ms with two windows). Separate 2x UI layer (second texture) is the owner's call.
- Not done: bar page buttons + speed cell, CELL genes vs colony, SIDE BY SIDE bars, training/OVERVIEW charts, NUMBERS sliders, scrollable +N MORE.

**Why:** a later thread picking this up must not redo the review or re-ask the owner.
**How to apply:** build from the plan doc; check sim results stay bit-identical; PRs go to the merge desk.
