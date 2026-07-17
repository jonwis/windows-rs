// Pure-MSVC entry point for the C++/WinRT language-projection benchmark.
//
// This is the "no Rust, no cargo" build of the same bench that `../cpp/src/bench.cpp`
// implements. That translation unit exports `lang_perf_cpp(iterations)`; here we provide
// a native `main` that parses `--iterations N` (or the LANG_PERF_ITER env var) and calls
// it directly, so the whole tool is produced by cl.exe + link.exe alone.
//
// The matching no-op component (`../component_cpp/src/component.cpp`) is compiled to
// LangPerf.dll next to this exe by build.ps1, which is the name WinRT activation probes.

#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>

extern "C" int32_t __stdcall lang_perf_cpp(uint64_t iterations) noexcept;

namespace {
// Default to a tiny count so a bare run stays fast; pass `--iterations N` (or set
// LANG_PERF_ITER=N) for a real measurement. Mirrors the Rust shim's defaults.
constexpr uint64_t kDefaultIterations = 1000;

uint64_t iterations(int argc, char** argv) {
    for (int i = 1; i + 1 < argc; i++) {
        if (std::strcmp(argv[i], "--iterations") == 0) {
            return std::strtoull(argv[i + 1], nullptr, 10);
        }
    }
    if (const char* env = std::getenv("LANG_PERF_ITER")) {
        return std::strtoull(env, nullptr, 10);
    }
    return kDefaultIterations;
}
}

int main(int argc, char** argv) {
    const int32_t hr = lang_perf_cpp(iterations(argc, argv));
    if (hr < 0) {
        std::fprintf(stderr, "lang_perf_cpp failed: 0x%08x\n", static_cast<unsigned>(hr));
        return 1;
    }
    return 0;
}
