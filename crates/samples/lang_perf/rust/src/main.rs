#[allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    dead_code,
    clippy::all
)]
mod bindings;

// Default to a tiny count so `cargo run`/CI stays fast. Pass `--iterations N` or set
// `LANG_PERF_ITER=N` for a real measurement (the original used 10_000_000).
const DEFAULT_ITERATIONS: u64 = 1_000;

fn iterations() -> u64 {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--iterations"
            && let Some(value) = args.next()
        {
            return value.parse().expect("invalid --iterations value");
        }
    }
    std::env::var("LANG_PERF_ITER")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_ITERATIONS)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> windows_core::Result<()> {
    use bindings::*;
    use windows_core::*;

    // Run `body` for `warmup` untimed iterations -- reaching steady CPU frequency and warming this
    // loop's caches/predictors -- then time `iterations`. Generic, so the body is monomorphized and
    // the timed loop stays as tight as an inline for-loop.
    fn measure<F: FnMut() -> windows_core::Result<()>>(
        label: &str,
        warmup: u64,
        iterations: u64,
        mut body: F,
    ) -> windows_core::Result<()> {
        for _ in 0..warmup {
            body()?;
        }
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            body()?;
        }
        report(label, start);
        Ok(())
    }

    stage_component(component_file());

    let iterations = iterations();
    let warmup: u64 = std::env::var("LANG_PERF_WARMUP")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let object = Class::new()?;
    println!(
        "# Rust consumer -> {} component - {iterations} iterations ({warmup} warmup)",
        object.Lang()?.to_string_lossy()
    );

    measure("Create", warmup, iterations, || {
        let _ = Class::new()?;
        Ok(())
    })?;

    measure("Int32", warmup, iterations, || {
        object.SetInt32Property(123)?;
        let _ = object.Int32Property()?;
        Ok(())
    })?;

    measure("String", warmup, iterations, || {
        object.SetStringProperty(h!("value"))?;
        let _ = object.StringProperty()?;
        Ok(())
    })?;

    measure("Object", warmup, iterations, || {
        object.SetObjectProperty(&object)?;
        let _ = object.ObjectProperty()?;
        Ok(())
    })?;

    measure("Cast", warmup, iterations, || {
        let _ = object.ObjectProperty()?.cast::<INonDefault>()?.Value()?;
        Ok(())
    })?;

    {
        let _revoker = object.Event(|_sender, _value| {})?;
        measure("Event", warmup, iterations, || {
            object.Raise()?;
            Ok(())
        })?;
    }

    measure("AddRemove", warmup, iterations, || {
        let _revoker = object.Event(|_sender, _value| {})?;
        Ok(())
    })?;

    {
        let count = iterations.min(u32::MAX as u64) as u32;
        let vector = object.Items(count)?;

        let iterate = || {
            let mut sum = 0i32;
            for value in &vector {
                sum = sum.wrapping_add(value);
            }
            std::hint::black_box(sum);
        };
        if warmup > 0 {
            iterate();
        }
        let start = std::time::Instant::now();
        iterate();
        report("IterateVector", start);

        let mut buffer = vec![0i32; count as usize];
        if warmup > 0 {
            let _ = vector.GetMany(0, &mut buffer)?;
        }
        let start = std::time::Instant::now();
        let _ = vector.GetMany(0, &mut buffer)?;
        std::hint::black_box(&buffer);
        report("GetMany", start);

        let map = object.Map(count)?;
        let iterate_map = || -> windows_core::Result<()> {
            let mut sum = 0i32;
            for pair in &map {
                sum = sum.wrapping_add(pair.Value()?);
            }
            std::hint::black_box(sum);
            Ok(())
        };
        if warmup > 0 {
            iterate_map()?;
        }
        let start = std::time::Instant::now();
        iterate_map()?;
        report("Map", start);
    }

    measure("Async", warmup, iterations, || {
        let _ = object.Operation()?.join()?;
        Ok(())
    })?;

    measure("Reference", warmup, iterations, || {
        object.SetReferenceProperty(Some(0))?;
        let _ = object.ReferenceProperty()?;
        Ok(())
    })?;

    measure("Error", warmup, iterations, || {
        let _ = object.Next();
        Ok(())
    })?;

    // Variant of `Error`: constructs an *originating* error each iteration. `Error::new` with a
    // non-empty message calls `RoOriginateErrorW`, which builds an `IRestrictedErrorInfo` and sets
    // it on the thread, then captures it back -- the windows-rs equivalent of the origination
    // cppwinrt performs at every throw boundary. Isolates windows-rs origination cost with no ABI
    // crossing and no C++ exception throw/unwind.
    measure("ErrorOriginate", warmup, iterations, || {
        std::hint::black_box(Error::new(HRESULT(0x8000_000B_u32 as i32), "value"));
        Ok(())
    })?;

    Ok(())
}

fn report(label: &str, start: std::time::Instant) {
    println!("{label}: {} ms", start.elapsed().as_millis());
}

// The component cdylibs use distinct names so every language's build can coexist in one
// target directory. Copy this consumer's own component in as LangPerf.dll -- the name
// WinRT activation probes -- right next to the executable so it is the one that loads.
fn stage_component(file: &str) {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let _ = std::fs::copy(dir.join(file), dir.join("LangPerf.dll"));
    }
}

// `--component rust|cpp` selects which language's component this consumer activates, so the
// matrix benchmark can point every consumer at either implementation. Defaults to Rust.
fn component_file() -> &'static str {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--component"
            && let Some(value) = args.next()
        {
            return match value.as_str() {
                "cpp" => "langperf_cpp.dll",
                "rust" => "langperf_rust.dll",
                other => panic!("unknown --component '{other}' (expected rust or cpp)"),
            };
        }
    }
    "langperf_rust.dll"
}
