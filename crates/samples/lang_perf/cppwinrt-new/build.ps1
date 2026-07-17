#!/usr/bin/env pwsh
# "cppwinrt-new" benchmark: the same C++/WinRT language-projection bench as ../msvc, but with
# the opt-in throughput fast paths baked in (String uses the `_hs` compile-time HSTRING literal;
# the async component returns `winrt::make_ready(0)` instead of `co_return 0;`), and built with a
# caller-supplied cppwinrt.exe. Point -CppWinRTExe at a cppwinrt.exe from the throughput-perf
# branch to measure the "best possible effort" C++/WinRT result.
#
# Pure-MSVC, no cargo/Rust. Flags match ../msvc: cppwinrt -optimize, cl /Ox /GL, link /LTCG
# /OPT:REF /OPT:ICF. Produces out\LangPerf.dll (component) + out\lang_perf_cppwinrt_new.exe.
[CmdletBinding()]
param(
    # Path to the cppwinrt.exe to generate the projection with. Defaults to the stock embedded
    # one, so with no argument this builds the same thing ../msvc does (minus the opt-in source,
    # which needs a cppwinrt that supports _hs / make_ready).
    [string]$CppWinRTExe = (Join-Path (Resolve-Path "$PSScriptRoot\..\..\..\..").Path 'crates\libs\cppwinrt\cppwinrt.exe'),
    [long]$Iterations = 0
)

$ErrorActionPreference = 'Stop'
$lp = (Resolve-Path "$PSScriptRoot\..").Path
$root = (Resolve-Path "$PSScriptRoot\..\..\..\..").Path
$cppwinrt = (Resolve-Path $CppWinRTExe).Path
$winmd = Join-Path $lp 'component\lang.winmd'
$reference = Join-Path $root 'crates\libs\bindgen\default'
$out = Join-Path $PSScriptRoot 'out'
$include = Join-Path $out 'include'

New-Item -ItemType Directory -Force -Path $out | Out-Null

Write-Host "Generating C++/WinRT projection with $cppwinrt ..." -ForegroundColor Cyan
& $cppwinrt '-in' $winmd $reference '-out' $include '-optimize'
if ($LASTEXITCODE -ne 0) { throw "cppwinrt failed ($LASTEXITCODE)" }

$vswhere = "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"
$vsPath = & $vswhere -latest -property installationPath
$vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "vcvars64.bat not found at $vcvars" }

$componentSrc = Join-Path $PSScriptRoot 'component.cpp'
$benchSrc = Join-Path $PSScriptRoot 'bench.cpp'
$mainSrc = Join-Path $PSScriptRoot 'main.cpp'

$common = '/nologo /std:c++20 /EHsc /Ox /GL /I "{0}"' -f $include
$linkOpt = '/LTCG /OPT:REF /OPT:ICF'

$script = @"
call "$vcvars" >nul || exit /b 1
cd /d "$out" || exit /b 1

echo === Building LangPerf.dll (component, make_ready) ===
cl $common /LD "$componentSrc" /Fe:LangPerf.dll /link $linkOpt /EXPORT:DllGetActivationFactory onecoreuap.lib || exit /b 1

echo === Building lang_perf_cppwinrt_new.exe (bench, _hs) ===
cl $common "$benchSrc" "$mainSrc" /Fe:lang_perf_cppwinrt_new.exe /link $linkOpt onecoreuap.lib || exit /b 1
"@

$bat = Join-Path $out '_build.bat'
Set-Content -Path $bat -Value $script -Encoding Ascii
& $env:ComSpec /c $bat
if ($LASTEXITCODE -ne 0) { throw "MSVC build failed ($LASTEXITCODE)" }

$exe = Join-Path $out 'lang_perf_cppwinrt_new.exe'
Write-Host "`nBuilt with $cppwinrt" -ForegroundColor Green
Get-ChildItem $out -Include 'LangPerf.dll', 'lang_perf_cppwinrt_new.exe' -Recurse | Select-Object Name, Length | Format-Table

if ($Iterations -gt 0) {
    Write-Host "Running $exe --iterations $Iterations ...`n" -ForegroundColor Cyan
    & $exe --iterations $Iterations
}
