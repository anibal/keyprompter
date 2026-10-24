# KeyPrompter PoC: Feature playbook

- [x] 1. `how` over the affected subsystem. (done inline: read OpenTeleprompter src-tauri/src/lib.rs window tricks and GPUI Kit 0.7.3 window/platform source)
- [x] 2. `architect` for parallel design exploration. skip: greenfield PoC with one window; the only real fork (window kind) was settled empirically by the spike below, data shape named in the delegate brief.
- [ ] 3. Write the throughput checkpoint as four todo items.
  - [x] Blocking first steps: offline build with copied lockfile (passed); window spike (PopUp stays on screen after deactivation, layer 101; Floating vanishes).
  - [x] Independent workstreams: n/a: one crate, ~4 small modules coupled by one Show type.
  - [x] Shared mutable state: n/a: single delegate, single worktree.
  - [x] Smallest safe decomposition: one sonnet delegate owns the whole crate; parser and layout are pure and tested, shell is thin.
- [ ] 4. Delegate code-writing to a subagent (sonnet, own worktree).
- [ ] 5. Verify on the matching surface.
- [ ] 6. Rebase into small, ordered commits.
- [ ] 7. If the design is contested, `interrogate` before shipping. skip unless review finds a contested call.
- [ ] 8. Run **Opening a PR**. skip: no remote; local repo PoC.
