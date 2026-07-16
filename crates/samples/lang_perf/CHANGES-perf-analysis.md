# lang_perf — max-optimization & parity changes

Changes made to the `crates/samples/lang_perf` language-projection benchmark (and the
minimum workspace plumbing to support them) while collecting apples-to-apples throughput
data for the C++/WinRT, Rust (windows-rs), and C#/WinRT projections. Grouped so each can be
reviewed / pushed independently.

## 1. Max-optimization C++/WinRT build (fairness)

The C++ consumer and C++ component are compiled by the `cc` crate (→ `cl.exe`) and linked by
`rustc`'s default MSVC `link.exe`. A cargo *release* build only passed `/O2`; these make the
C++/WinRT side use full optimization so the comparison is fair.

- `cpp/build.rs`
  - cppwinrt: added `-optimize` (unified construction + cached activation factory).
  - `cl.exe`: added `/Ox` (max optimize) and `/GL` (whole-program).
  - `link.exe` (via `cargo:rustc-link-arg-bins`): added `/LTCG /OPT:REF /OPT:ICF`.
- `component_cpp/build.rs`
  - cppwinrt: added `-optimize`.
  - `cl.exe`: added `/Ox /GL`.
  - `link.exe` (via `cargo:rustc-link-arg-cdylib`): added `/LTCG /OPT:REF /OPT:ICF`.

## 2. Pure-MSVC C++-only build (no cargo / no Rust bins)

A standalone build of the identical C++ bench + component using only `cppwinrt.exe`,
`cl.exe`, and `link.exe`. Reuses `cpp/src/bench.cpp` and `component_cpp/src/component.cpp`
verbatim; adds only a native `main`.

- `msvc/main.cpp` (new) — native entry point; parses `--iterations`/`LANG_PERF_ITER` and
  calls the `extern "C" lang_perf_cpp` exported by `bench.cpp`.
- `msvc/build.ps1` (new) — cppwinrt `-optimize` → `cl /Ox /GL` → `link /LTCG /OPT:REF
  /OPT:ICF`; builds `LangPerf.dll` (component) + `lang_perf_cpp_msvc.exe` (bench) and stages
  the DLL next to the exe. Optional `-Iterations N` runs it.
- `Cargo.toml` (workspace root) — added `crates/samples/lang_perf/msvc` to `[workspace]
  exclude` so the non-cargo directory does not break the `crates/samples/*/*` member glob.

## 3. Reference-loop parity — verified equivalent (no change needed)

Initially it looked like the Rust `Reference` loop skipped the consumer-side unbox that C++
(`.Value()`) and C# (`.Value`) perform, because it writes only `let _ =
object.ReferenceProperty()?;`. Building disproved that: windows-rs projects the
`IReference<Int32>` getter as `Result<i32>` and calls `.Value()` **inside the projection**
(`rust/src/bindings.rs:172`: `.and_then(|r__: IReference<i32>| r__.Value())`). So the bare
`ReferenceProperty()?` already returns the unboxed `i32` — fully apples-to-apples with C++/C#.
No source change; the loop was left as-is.

## 4. Rust `ErrorOriginate` variant (origination-cost isolation)

Added a loop to the Rust consumer that measures windows-rs *origination* cost directly, to
quantify what the C++/WinRT `Error` loop pays for `RoOriginateLanguageException` at each
boundary. The stock `Error` loop only checks a returned `Result::Err` (a bare HRESULT, no
origination); this variant constructs an originating error each iteration.

- `rust/src/main.rs` (new `ErrorOriginate` loop, after `Error`)
  - `Error::new(HRESULT(0x8000_000B /* E_BOUNDS */), "value")` per iteration → calls
    `RoOriginateErrorW` + captures the `IRestrictedErrorInfo` (windows-result
    `error.rs:96,293`). No ABI crossing and no C++ throw/unwind, so it isolates the
    origination work itself, comparable to the origination component of the C++ `Error` cost
    (the Rust→C++ minus Rust→Rust delta).

## Notes / not changed

- The 12 other loops (Create, Int32, String, Object, Cast, Event, AddRemove, IterateVector,
  GetMany, Map, Async, Error) were verified semantically equivalent across all three
  consumers and left untouched.
- The `Error` loop's mechanism difference (Rust returns `Result::Err`; C++/C# throw+catch) is
  the projection behavior being measured, not a fairness bug.
