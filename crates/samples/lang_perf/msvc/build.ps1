#!/usr/bin/env pwsh
# Pure-MSVC build of the C++/WinRT language-projection benchmark -- no cargo, no Rust bins.
# Produces, using only cppwinrt.exe + cl.exe + link.exe:
#   out\LangPerf.dll            the no-op C++/WinRT component (../component_cpp/src/component.cpp)
#   out\lang_perf_cpp_msvc.exe  the bench (../cpp/src/bench.cpp) + main.cpp, staged next to the DLL
#
# Max-optimization flags for a fair C++/WinRT comparison:
#   cppwinrt : -optimize          (unified construction + cached activation factory)
#   cl       : /Ox /GL            (max optimization + whole-program)
#   link     : /LTCG /OPT:REF /OPT:ICF
[CmdletBinding()]
param(
    [long]$Iterations = 0   # if > 0, run the exe after building
)

$ErrorActionPreference = 'Stop'
$lp = (Resolve-Path "$PSScriptRoot\..").Path
$root = (Resolve-Path "$PSScriptRoot\..\..\..\..").Path
$cppwinrt = Join-Path $root 'crates\libs\cppwinrt\cppwinrt.exe'
$winmd = Join-Path $lp 'component\lang.winmd'
$reference = Join-Path $root 'crates\libs\bindgen\default'
$out = Join-Path $PSScriptRoot 'out'
$include = Join-Path $out 'include'

New-Item -ItemType Directory -Force -Path $out | Out-Null

Write-Host 'Generating optimized C++/WinRT projection (cppwinrt -optimize)...' -ForegroundColor Cyan
& $cppwinrt '-in' $winmd $reference '-out' $include '-optimize'
if ($LASTEXITCODE -ne 0) { throw "cppwinrt failed ($LASTEXITCODE)" }

# Locate vcvars64.bat so cl/link run against the x64 MSVC toolchain.
$vswhere = "C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe"
$vsPath = & $vswhere -latest -property installationPath
$vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "vcvars64.bat not found at $vcvars" }

$componentSrc = Join-Path $lp 'component_cpp\src\component.cpp'
$benchSrc = Join-Path $lp 'cpp\src\bench.cpp'
$mainSrc = Join-Path $PSScriptRoot 'main.cpp'

$common = '/nologo /std:c++20 /EHsc /Ox /GL /I "{0}"' -f $include
$linkOpt = '/LTCG /OPT:REF /OPT:ICF'

# One cmd process: vcvars then component DLL then the bench exe. Compile into $out.
$script = @"
call "$vcvars" >nul || exit /b 1
cd /d "$out" || exit /b 1

echo === Building LangPerf.dll (component) ===
cl $common /LD "$componentSrc" /Fe:LangPerf.dll /link $linkOpt /EXPORT:DllGetActivationFactory onecoreuap.lib || exit /b 1

echo === Building lang_perf_cpp_msvc.exe (bench) ===
cl $common "$benchSrc" "$mainSrc" /Fe:lang_perf_cpp_msvc.exe /link $linkOpt onecoreuap.lib || exit /b 1
"@

$bat = Join-Path $out '_build.bat'
Set-Content -Path $bat -Value $script -Encoding Ascii
& $env:ComSpec /c $bat
if ($LASTEXITCODE -ne 0) { throw "MSVC build failed ($LASTEXITCODE)" }

$exe = Join-Path $out 'lang_perf_cpp_msvc.exe'
Write-Host "`nBuilt:" -ForegroundColor Green
Get-ChildItem $out -Include 'LangPerf.dll', 'lang_perf_cpp_msvc.exe' -Recurse | Select-Object Name, Length | Format-Table

if ($Iterations -gt 0) {
    Write-Host "Running $exe --iterations $Iterations ...`n" -ForegroundColor Cyan
    & $exe --iterations $Iterations
}
