# Attempted minimum repro for unwanted stdout in wasm32-wasi builds

This is an attempt at a minimal reproduction to show how `wasi_snapshot_preview1.fd_write` and friends are sneaking their way into WASM binaries.

This library contains two scenarios between which we can toggle using the `NO_PRINTF` env var.

- When unset, we use `./lib/repro.c` which has a reference to `printf` in the `Print` function.
- When set, we use `./lib/repro_no_printf.c` which does NOT have a reference to `printf`.

These two scenarios should ideally NOT have an impact on the final WASM binary because:

1. We're using `lto=true`
2. The `Print` function should only get linked in when the `print` feature is enabled. It is disabled by default.

## Usage

**Scenario 1**: No `printf` in the c code.

```sh
NO_PRINTF=1 cargo build --target=wasm32-wasi --profile=release
wasm-tools print target/wasm32-wasi/release/wasi_import_repro.wasm | grep "(import \| (export"
```

We can see that there is are absolutely no WASI imports:

```wasm
  (export "memory" (memory 0))
  (export "fallible_func" (func $fallible_func.command_export))
```

**Scenario 2**: A `printf` call in c code that should be unreferenced.

```sh
cargo build --target=wasm32-wasi --profile=release
wasm-tools print target/wasm32-wasi/release/wasi_import_repro.wasm | grep "(import \| (export"
```

We can see that the `fd_*` family get pulled along even though they shouldn't have a strong reference.

```wasm
  (import "wasi_snapshot_preview1" "fd_close" (func $__imported_wasi_snapshot_preview1_fd_close (;0;) (type 2)))
  (import "wasi_snapshot_preview1" "fd_fdstat_get" (func $__imported_wasi_snapshot_preview1_fd_fdstat_get (;1;) (type 3)))
  (import "wasi_snapshot_preview1" "fd_seek" (func $__imported_wasi_snapshot_preview1_fd_seek (;2;) (type 4)))
  (import "wasi_snapshot_preview1" "fd_write" (func $__imported_wasi_snapshot_preview1_fd_write (;3;) (type 5)))
  (export "memory" (memory 0))
  (export "fallible_func" (func $fallible_func.command_export))
```
