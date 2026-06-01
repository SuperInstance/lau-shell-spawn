# lau-shell-spawn

**Shell spawning system — Hermes creates child shells in their own sandboxes**

When the kernel needs more hands, it spawns child shells with scoped capabilities, isolated paths, and resource budgets. Think of it as lightweight process spawning, but for the LAU universe.

---

## What This Does

`lau-shell-spawn` provides a Rust library for managing a hierarchy of **child shells**. A parent shell (typically Hermes) can:

- **Spawn** child shells of different kinds (ZeroClaw, CUDAClaw, Ensign, custom)
- **Template** common shell configurations for one-command instantiation
- **Sandbox** certain shell types so they can only access their own subtree
- **Budget** each child's resource consumption with conservation limits
- **Control** the full lifecycle: spawn → grant/revoke APIs → destroy

Every child shell carries its own set of ports, APIs, rooms, and a resource budget — all serializable to JSON for persistence or network transport.

---

## Key Idea

The library models **capability-based security** for spawned agents:

| Shell Kind | Sandboxed? | Default APIs | Typical Role |
|---|---|---|---|
| `Hermes` | ✗ | read, write, network | Full-access parent |
| `ZeroClaw` | ✓ | read | Isolated worker |
| `CUDAClaw` | ✓ | read, compute, cuda | GPU compute worker |
| `Ensign` | ✗ | read, write | Junior operator |
| `Custom(N)` | ✗ | read | User-defined |

Sandboxed shells (`ZeroClaw`, `CUDAClaw`) have **path-based filesystem isolation**: they can only access paths under their own `universe_path`. Unsandboxed shells can roam freely.

---

## Install

Add to your `Cargo.toml`:

```toml
[dependencies]
lau-shell-spawn = "0.1.0"
```

Requires Rust 2021 edition. The only runtime dependency is `serde` (with `derive`).

---

## Quick Start

### Spawning a child directly

```rust
use lau_shell_spawn::{ShellSpawn, ShellKind};

let mut spawner = ShellSpawn::new("/shells/hermes");

// Spawn a bare ZeroClaw child
let child = spawner.spawn(ShellKind::ZeroClaw, "worker-1").unwrap();

assert_eq!(child.kind, ShellKind::ZeroClaw);
assert_eq!(child.status(), ShellStatus::Running);
assert!(child.kind.is_sandboxed());
```

### Spawning from a template

```rust
use lau_shell_spawn::{ShellSpawn, ShellTemplate, PortConfig};

let mut spawner = ShellSpawn::new("/shells/hermes");

// Register a template
spawner.register_template(ShellTemplate::cudaclaw());

// Spawn from it — gets all template defaults (ports, APIs, budget)
let gpu_child = spawner.spawn_from_template("cudaclaw", "gpu-1").unwrap();

assert!(gpu_child.apis.contains(&"cuda".to_string()));
assert_eq!(gpu_child.conservation_budget, 500.0);
```

### Building a custom template

```rust
let bot_template = ShellTemplate::custom("my-sensor")
    .with_port(PortConfig::new("i2c", "in", "/dev/i2c-1"))
    .with_api("sensor_read")
    .with_room("sensor-lab")
    .with_budget(50.0);

spawner.register_template(bot_template);
let sensor = spawner.spawn_from_template("my-sensor", "s-1").unwrap();
```

### Managing lifecycle

```rust
// Grant and revoke APIs at runtime
spawner.get_child_mut("worker-1").unwrap().grant_api("write");
spawner.get_child_mut("worker-1").unwrap().revoke_api("write");

// Check budget compliance
spawner.get_child_mut("gpu-1").unwrap().conservation_used = 450.0;
assert!(spawner.get_child("gpu-1").unwrap().is_within_budget()); // 450 <= 500

// Sandbox enforcement
let child = spawner.get_child("worker-1").unwrap();
assert!(child.can_access_path("/shells/hermes/worker-1/data"));
assert!(!child.can_access_path("/etc/passwd"));

// Destroy when done
spawner.destroy("worker-1").unwrap();
```

### Serialization

Everything is `Serialize + Deserialize`:

```rust
let json = serde_json::to_string(&spawner).unwrap();
let restored: ShellSpawn = serde_json::from_str(&json).unwrap();
assert_eq!(restored.child_count(), spawner.child_count());
```

---

## API Reference

### `ShellStatus`

```rust
pub enum ShellStatus {
    Initializing,       // Just created, not yet running
    Running,            // Active and processing
    Idle,               // Alive but waiting
    Error(String),      // Something went wrong
    Destroyed,          // Torn down
}
```

### `ShellKind`

```rust
pub enum ShellKind {
    Hermes,             // Full-access parent shell
    ZeroClaw,           // Sandboxed read-only worker
    CUDAClaw,           // Sandboxed GPU compute worker
    Ensign,             // Unsandboxed junior operator
    Custom(String),     // User-defined shell type
}
```

**Methods:**
- `is_sandboxed() -> bool` — Whether this kind has path isolation
- `label() -> String` — Human-readable name
- `default_apis() -> Vec<String>` — APIs granted on spawn

### `PortConfig`

```rust
pub struct PortConfig {
    pub protocol: String,       // e.g. "http", "cuda", "i2c"
    pub direction: String,      // "in", "out", "inout"
    pub target: String,         // e.g. "0.0.0.0:8080", "gpu-bus"
    pub permissions: Vec<String>,
    pub enabled: bool,
}
```

### `ChildShell`

The core unit — a spawned child with identity, capabilities, and budget.

| Field | Type | Purpose |
|---|---|---|
| `id` | `String` | Unique ID (`shell-{timestamp}-{seq}`) |
| `name` | `String` | Human-assigned name (unique within parent) |
| `kind` | `ShellKind` | Determines sandboxing and default APIs |
| `universe_path` | `String` | This child's isolated subtree |
| `parent_path` | `String` | Path of the spawning parent |
| `ports` | `Vec<PortConfig>` | Communication endpoints |
| `apis` | `Vec<String>` | Granted capabilities |
| `rooms` | `Vec<String>` | Named communication channels |
| `status` | `ShellStatus` | Current lifecycle state |
| `autonomy_level` | `u32` | How independently the child can act |
| `conservation_budget` | `f64` | Resource ceiling |
| `conservation_used` | `f64` | Current resource consumption |

**Key methods:**
- `add_port(port)` — Attach a communication endpoint
- `grant_api(api)` / `revoke_api(api)` — Modify capabilities
- `grant_room(room_id)` — Join a named channel
- `set_budget(budget)` — Set resource ceiling
- `is_within_budget() -> bool` — Check if still under budget
- `can_access_path(path) -> bool` — Sandbox enforcement check
- `describe() -> String` — Human-readable summary

### `ShellTemplate`

Pre-configured shell blueprints with a builder API.

**Built-in templates:**
- `ShellTemplate::zeroclaw()` — Isolated worker (budget: 100, autonomy: 1)
- `ShellTemplate::cudaclaw()` — GPU compute worker (budget: 500, autonomy: 2)
- `ShellTemplate::telegram_bot()` — Messaging bot (budget: 200, autonomy: 3)
- `ShellTemplate::underwater_vehicle()` — Physical vehicle controller (budget: 1000, autonomy: 5, max children: 2)
- `ShellTemplate::media_center()` — Media hub (budget: 300, autonomy: 2, max children: 4)
- `ShellTemplate::custom(name)` — Blank slate for your own types

**Builder methods (chainable):**
- `.with_port(port)` — Add a port
- `.with_api(api)` — Add an API (deduped)
- `.with_room(room)` — Add a room (deduped)
- `.with_budget(budget)` — Set resource ceiling

### `ShellSpawn`

The spawner — owns the parent path and all children.

**Methods:**
- `new(parent_path) -> ShellSpawn` — Create a new spawner
- `register_template(template)` — Register a named template
- `spawn(kind, name) -> Result<ChildShell, SpawnError>` — Direct spawn
- `spawn_from_template(template_name, name) -> Result<ChildShell, SpawnError>` — Template spawn
- `get_child(name) -> Option<&ChildShell>` — Look up a child
- `get_child_mut(name) -> Option<&mut ChildShell>` — Mutable access
- `list_children() -> Vec<&ChildShell>` — All active children
- `destroy(name) -> Result<(), SpawnError>` — Tear down a child
- `child_count() -> usize` — Number of active children

### `SpawnError`

```rust
pub enum SpawnError {
    NameExists(String),          // Duplicate child name
    MaxChildrenReached(u32),     // Child limit hit
    PathConflict(String),        // Universe path collision
    TemplateNotFound(String),    // Unknown template name
    SandboxViolation(String),    // Attempted escape from sandbox
}
```

Implements `std::error::Error` and `Display`.

---

## How It Works

### Architecture

```
ShellSpawn (parent: "/shells/hermes")
├── templates: { "zeroclaw": ..., "cudaclaw": ... }
└── children:
    ├── "zc-1" → ChildShell { kind: ZeroClaw, universe: "/shells/hermes/zc-1", ... }
    ├── "cc-1" → ChildShell { kind: CUDAClaw, universe: "/shells/hermes/cc-1", ... }
    └── "ensign-1" → ChildShell { kind: Ensign, ... }
```

### Spawn flow

1. **Direct spawn** (`spawn`): Creates a `ChildShell`, assigns a unique ID via atomic counter, derives the `universe_path` as `{parent_path}/{name}`, grants default APIs for the shell kind, and sets status to `Running`.

2. **Template spawn** (`spawn_from_template`): Looks up a registered `ShellTemplate`, creates a `ChildShell`, copies all template defaults (ports, APIs, rooms, budget, autonomy level), and sets status to `Running`.

3. **Duplicate detection**: Both paths check for name collisions before creation.

### Sandboxing

When `can_access_path(path)` is called on a sandboxed shell (ZeroClaw, CUDAClaw), the method checks if `path` starts with the child's `universe_path`. If not, access is denied. Unsandboxed shells (Hermes, Ensign, Custom) always return `true`.

This is a simple but effective **prefix-based filesystem isolation** — the child can see everything under its own subtree and nothing outside it.

### ID generation

Child IDs use `shell-{unix_timestamp}-{atomic_counter}`. The global `SHELL_COUNTER` is an `AtomicU64` with `Relaxed` ordering — safe for single-process use, no locking required.

---

## The Math

### Budget conservation model

Each child has a resource budget:

```
is_within_budget ⟺ conservation_used ≤ conservation_budget
```

The library doesn't enforce budget limits automatically — it provides the **tracking** so the parent shell can make enforcement decisions. The parent checks `is_within_budget()` and decides what to do (throttle, warn, destroy).

### Uniqueness guarantees

Child names are unique within a `ShellSpawn` instance (enforced by a `HashMap<String, ChildShell>`). Child IDs are globally unique within a process via `AtomicU64` increment. Two children spawned in the same second get different sequential IDs.

### Sandbox invariant

For any sandboxed shell with `universe_path = U`:

```
∀ path ∈ Paths: can_access_path(path) = (path.starts_with(U))
```

This is a **prefix closure** — the accessible set is exactly the subtree rooted at `U`. No escaping upward, no lateral access to siblings.

---

## Testing

The crate includes **52 tests** covering:

- `ShellKind` properties (sandboxing, labels, default APIs)
- `ShellStatus` display and equality
- `PortConfig` construction and equality
- `ChildShell` lifecycle (creation, port/API/room management, budget, sandboxing, describe)
- `ShellTemplate` built-in templates and builder chaining
- `ShellSpawn` full CRUD (spawn, get, mutate, list, destroy, duplicate detection)
- Serde round-trips for all types
- `SpawnError` display formatting
- Integration tests (full lifecycle, multiple templates, sandbox escape prevention)

Run with:

```bash
cargo test
```

---

## License

MIT
