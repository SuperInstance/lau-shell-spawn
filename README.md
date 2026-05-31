# lau-shell-spawn

Child shell spawning — ZeroClaw and CUDAClaw templates with sandboxing. When the kernel needs more hands, it spawns child shells with scoped capabilities.

## The concept in 60 seconds

The ShellKernel can spawn child shells, each with:
- **Scoped capabilities:** a child can only do what its allowance permits
- **Templates:** ZeroClaw (isolated, no external access) and CUDAClaw (GPU-capable)
- **Sandboxing:** each child runs in its own tile store, unable to corrupt the parent
- **Lifecycle:** children are spawned, do work, report results, and are cleaned up

Think of it as lightweight process spawning, but for PLATO tiles.

## Quick start

```rust
use lau_shell_spawn::{SpawnConfig, ShellTemplate, ChildShell};

// Spawn a ZeroClaw child — fully isolated
let config = SpawnConfig::new()
    .with_template(ShellTemplate::ZeroClaw)
    .with_allowance("compute", 100.0)
    .with_timeout(30); // seconds

let child = ChildShell::spawn(config);
child.send_task("analyze the vibe field");
let result = child.await_result();
child.stand_down(); // clean up
```

## Contributing

[Open an issue](https://github.com/SuperInstance/lau-shell-spawn/issues) or PR.
