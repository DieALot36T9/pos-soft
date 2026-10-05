# Scale fixtures

Lines as each scale protocol sends them, used by `src/scale/fixture_tests.rs`.
They follow the vendors' published formats. Replace them with recordings from real
scales when they are first tested (M13), and keep the tests passing.

| File | Profile | What it shows |
|---|---|---|
| `scale_generic_continuous.txt` | `generic_continuous` | Weight rising, then 1.234 kg three times (stable after 3 equal readings) |
| `scale_cas.txt` | `cas` | Unstable (`US`) lines, then a stable (`ST`) 1.234 kg |
| `scale_toledo.bin` | `toledo` | Two "moving" answers (STX `?` status CR), then STX `01.234` CR. One answer per `W` request |
