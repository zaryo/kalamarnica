# kalamarnica

Simple and opinionated CLI tool which changes github contexts - Accounts and per-account tokens (permissions). It currently supports Github and Gitlab.

## Installation

You can install the latest pre-built binary from [releases](https://github.com/Propfend/kalamarnica/releases).

Build using Docker:

```bash
docker build -t kalamarnica .
```

Build using Cargo:

> [!NOTE]
> [MSRV](https://github.com/foresterre/cargo-msrv) is 1.88.0

> [!NOTE]
> `kalamarnica` needs `zlib` for dynamic linker.

```bash
cargo install kalamarnica
```

Build from source repository:

```bash
git clone https://github.com/Propfend/kalamarnica.git
cd kalamarnica
cargo build --release
```


## Basic usage

```bash
# Create context using current session information
kalamarnica create --name personal --vcs github --from-current


# Create context providing specific information
kalamarnica create --name work --vcs github --hostname github.com --user myuser --transport https

# Switch context
kalamarnica switch personal --vcs github

# Display detailed information about all contexts
kalamarnica auth-status

```

## Commands reference

### `list`

List all saved contexts with their configuration. The active context is marked with `*`.

### `current`

Show the active context and any repository-bound context.

### `create`

Creates a new context.

| Flag | Description |
|---|---|
| `--name` | Name for the new context |
| `--vcs` | Versioning code system platform: `github` or `gitlab` |
| `--from-current` | Detect hostname and user from the current VCS session |
| `--hostname` | VCS hostname (e.g., `github.com`) |
| `--user` | VCS username |
| `--transport` | Git transport protocol: `ssh` (default) or `https` |

Either `--from-current` or both `--hostname` and `--user` are required.

### `switch <name> --vcs <vcs>`

Switches to a context. Applies the stored token and verifies authentication.

### `set-token --name <name> --vcs <vcs> <token>`

Stores a per-context token.

### `delete <name> --vcs <vcs>`

Deletes a context and its stored token.

### `bind <name> --vcs <vcs>`

Binds the current repository to a context. Creates a `.vcs_context` file in the repository root.

### `unbind`

Removes the repository context binding.

### `apply`

Applies the repository-bound context (switch to the context specified in `.vcs_context`).

### `auth-status`

Shows authentication status for all contexts, including host, user, transport, token, and auth verification by versioning code system.
