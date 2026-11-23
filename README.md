# coding-crew

An agentic todo list with superpowers.

A task management tool for agents. Three interfaces:

- **talita** — the CLI tool. Agent-facing. **T**ask **A**dministration,
  **L**ifecycle & **I**nteraction **T**erminal **A**pplication. Writes and
  manages tasks.
- **raul** — the TUI for the work. Human-facing. **R**eview, **A**ssess,
  **U**nderstand & **L**earn. Shows them.
- **hugo** — the TUI for the flow. Human-facing. **H**uman-guided **U**nified
  **G**overnance **&** **O**rchestration. Configures and watches the crew.

## Status

Early, and the two TUIs are built: **raul** shows the work and **hugo**
shows the flow. The task model and the roles are designed in [docs/](docs/);
talita's full command surface comes later, when it is asked for.

| doc | what |
|---|---|
| [docs/tasks/task-model.md](docs/tasks/task-model.md) | the unit of work: format, ids, statuses, transitions, review, evidence |
| [docs/roles/roles.md](docs/roles/roles.md) | actors, roles, ownership, session boundaries |
| [mock-data/](mock-data/) | one task per lifecycle stage, plus edge cases |

## Layout

```
crates/
  core/    # shared model   (coding-crew-core)
  engine/  # markdown engine (coding-crew-engine)
  theme/   # the slots and the presets (coding-crew-theme)
  talita/  # CLI tool       (coding-crew-talita, binary talita)
  raul/    # TUI for the work (coding-crew-raul, binary raul)
  hugo/    # TUI for the flow (coding-crew-hugo, binary hugo)
```

Tasks live in a managed project's own `coding-crew/` folder:

```
<project>/coding-crew/work/TSK01-<slug>.json
```

## Build from source

```sh
make install
```

`make install` installs `hugo` and `raul` with `cargo install`, then prints the
folder to add to PATH. Agents run `hugo --skill <role>`, so `hugo` must resolve
to this build; hugo's `s` refuses to start the orchestrator otherwise. Homebrew
is the intended install and is not yet published.

## Develop

Rust is pinned via `rust-toolchain.toml` (stable, minimal profile, with
`rustfmt` and `clippy`).

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Make targets

| target | what |
|---|---|
| `make help` | list targets |
| `make check` | the gate: fmt, clippy, tests |
| `make build` | build the workspace |
| `make build-raul` | build raul only |
| `make build-talita` | build talita only |
| `make build-release` | release binaries |
| `make install` | install hugo and raul from this checkout, and name the folder to add to PATH |
| `make raul` | run raul |
| `make test` | tests |
| `make fmt` / `make fmt-check` | format / check formatting |
| `make lint` | clippy with warnings as errors |
| `make doc` | API docs |
| `make clean` | remove build artifacts |

## License

MIT OR Apache-2.0
