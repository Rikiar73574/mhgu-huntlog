# Combined HuntLog plugin source

The full monster HUD, DPS meter, background recorder and countdowns are built
from this Rust crate. See [the project README](../README.md) for installation,
portable builds, tests, desktop applications and validation limits.

This crate adapts sadgnk169/MHGU-DPS-Meter under MIT; its original notice is
retained in LICENSE. Forge Rust SDK is pinned in Cargo.toml/Cargo.lock.
The historical upstream build.sh/build.ps1 use devkitPro; the supported
standalone build entry is build-local.sh and produces ../build/hunter_hud.nro.

No character saves, captured player memory or personal hunt history are needed
to build or run the host tests. Game ABI checks use explicitly supplied owned
inputs only; see the optional tools/verify_*_arm.py commands.
