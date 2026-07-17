#include <windows.h>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <vector>
#include "winrt/LangPerf.h"
#include "winrt/Windows.Foundation.h"
#include "winrt/Windows.Foundation.Collections.h"

using namespace winrt;
using namespace winrt::LangPerf;

static long long elapsed_ms(std::chrono::high_resolution_clock::time_point const start)
{
    return std::chrono::duration_cast<std::chrono::milliseconds>(
               std::chrono::high_resolution_clock::now() - start)
        .count();
}

// Run `body` for `warmup` untimed iterations -- bringing the CPU to steady turbo frequency and
// warming this loop's caches and branch predictors -- then time `iterations` measured iterations.
// The warmup count comes from LANG_PERF_WARMUP so the exported entry point keeps its signature.
// The lambda is inlined (F&&), so the timed loop is as tight as a hand-written for-loop.
template <typename F>
static void measure(char const* label, uint64_t warmup, uint64_t iterations, F&& body)
{
    for (uint64_t i = 0; i < warmup; i++) body();
    auto const start = std::chrono::high_resolution_clock::now();
    for (uint64_t i = 0; i < iterations; i++) body();
    printf("%s: %lld ms\n", label, elapsed_ms(start));
}

extern "C" int32_t __stdcall lang_perf_cpp(uint64_t iterations) noexcept
{
    try
    {
        uint64_t warmup = 0;
        if (char const* w = std::getenv("LANG_PERF_WARMUP"))
        {
            warmup = std::strtoull(w, nullptr, 10);
        }

        init_apartment();
        Class object;
        printf("# C++ consumer -> %ls component - %llu iterations (%llu warmup)\n",
               object.Lang().c_str(),
               static_cast<unsigned long long>(iterations),
               static_cast<unsigned long long>(warmup));

        measure("Create", warmup, iterations, [&] {
            Class temp;
            (void)temp;
        });

        measure("Int32", warmup, iterations, [&] {
            object.Int32Property(123);
            auto value = object.Int32Property();
            (void)value;
        });

        measure("String", warmup, iterations, [&] {
            object.StringProperty(L"value");
            auto value = object.StringProperty();
            (void)value;
        });

        measure("Object", warmup, iterations, [&] {
            object.ObjectProperty(object);
            auto value = object.ObjectProperty();
            (void)value;
        });

        measure("Cast", warmup, iterations, [&] {
            auto value = object.ObjectProperty().as<INonDefault>().Value();
            (void)value;
        });

        {
            auto token = object.Event([](Windows::Foundation::IInspectable const&, int32_t) {});
            measure("Event", warmup, iterations, [&] {
                object.Raise();
            });
            object.Event(token);
        }

        measure("AddRemove", warmup, iterations, [&] {
            auto added = object.Event([](Windows::Foundation::IInspectable const&, int32_t) {});
            object.Event(added);
        });

        {
            uint32_t const count = iterations > UINT32_MAX
                ? UINT32_MAX
                : static_cast<uint32_t>(iterations);
            auto vector = object.Items(count);

            auto iterate = [&] {
                int32_t sum = 0;
                for (auto&& value : vector) sum += value;
                volatile int32_t sink = sum;
                (void)sink;
            };
            if (warmup) iterate();
            auto start = std::chrono::high_resolution_clock::now();
            iterate();
            printf("IterateVector: %lld ms\n", elapsed_ms(start));

            std::vector<int32_t> buffer(count);
            auto getmany = [&] { vector.GetMany(0, buffer); };
            if (warmup) getmany();
            start = std::chrono::high_resolution_clock::now();
            getmany();
            printf("GetMany: %lld ms\n", elapsed_ms(start));

            auto map = object.Map(count);
            auto iterate_map = [&] {
                int32_t msum = 0;
                for (auto&& pair : map) msum += pair.Value();
                volatile int32_t msink = msum;
                (void)msink;
            };
            if (warmup) iterate_map();
            start = std::chrono::high_resolution_clock::now();
            iterate_map();
            printf("Map: %lld ms\n", elapsed_ms(start));
        }

        measure("Async", warmup, iterations, [&] {
            auto value = object.Operation().get();
            (void)value;
        });

        measure("Reference", warmup, iterations, [&] {
            object.ReferenceProperty(0);
            auto value = object.ReferenceProperty().Value();
            (void)value;
        });

        measure("Error", warmup, iterations, [&] {
            try
            {
                (void)object.Next();
            }
            catch (hresult_error const&)
            {
            }
        });

        fflush(stdout);
        return 0;
    }
    catch (...)
    {
        return static_cast<int32_t>(winrt::to_hresult());
    }
}