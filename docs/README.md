# The documents

The house standard for this tree. Each one says one thing, and says it where
somebody will look for it.

| Document | What it is |
|---|---|
| [`../AGENTS.md`](../AGENTS.md) | the entry point for an agent working in this repository |
| [`../KNOWN-GAPS.md`](../KNOWN-GAPS.md) | what this tree does not do, and what its gates cannot see |
| [`../CONTRIBUTING.md`](../CONTRIBUTING.md) | how to contribute: one topic per commit, and what must be green |
| [`architecture.md`](architecture.md) | the layers, the seams, the doors and the front ends |
| [`agent-playbook.md`](agent-playbook.md) | how to drive craftkit from a tool, and how to read the results |
| [`clean-room.md`](clean-room.md) | the independent rule, and the table of products |
| [`dependency-policy.md`](dependency-policy.md) | the licence, ban and source policy |
| [`recipes/add-a-command.md`](recipes/add-a-command.md) | add a command to a battery |
| [`recipes/add-a-battery.md`](recipes/add-a-battery.md) | add a battery (a new plugin) |
| [`recipes/add-a-crate.md`](recipes/add-a-crate.md) | add a crate, and give it a layer |
| [`recipes/change-a-command.md`](recipes/change-a-command.md) | change a command's parameters — a format and a prose change |
| [`recipes/fix-a-crash.md`](recipes/fix-a-crash.md) | the regression-test discipline for a crash |
| [`../skills/craftkit-editing/SKILL.md`](../skills/craftkit-editing/SKILL.md) | the checklist for changing something |
| [`../skills/craftkit-checking/SKILL.md`](../skills/craftkit-checking/SKILL.md) | the checklist for something that is broken |

The **README** at the root is the design: what the tree is, what it does, and
how it got there. It is long, and it is a build log — the sections are numbered
and they were written in order. `KNOWN-GAPS.md` §6 lists the places where the
README has been overtaken by the code.

There are two gates, and they are not the same thing:

- `crates/craftkit-gates` is **this tree's own gate** — the deny attribute, the
  layering, "a plugin is a plugin", the front end is thin, and the bounds in
  the prose. Run it with `cargo test -p craftkit-gates`.
- `xtask/` is the **outer gate** — `cargo xtask doctor`, `cargo xtask ci`, and
  the phases `ci` runs. `KNOWN-GAPS.md` §1 says what is true of it in this
  tree today.
