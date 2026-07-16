fn main() {
    msvc_main();
}

#[cfg(not(target_env = "msvc"))]
fn msvc_main() {}

#[cfg(target_env = "msvc")]
fn msvc_main() {
    println!("cargo:rerun-if-changed=../component/lang.winmd");
    println!("cargo:rerun-if-changed=src/bench.cpp");
    println!("cargo:rustc-link-lib=onecoreuap");
    // Max-optimization link for a fair C++/WinRT comparison: whole-program
    // codegen (/GL objects require /LTCG), drop unreferenced (/OPT:REF), and
    // fold identical COMDATs (/OPT:ICF).
    println!("cargo:rustc-link-arg-bins=/LTCG");
    println!("cargo:rustc-link-arg-bins=/OPT:REF");
    println!("cargo:rustc-link-arg-bins=/OPT:ICF");

    let include = std::env::var("OUT_DIR").unwrap();

    cppwinrt::cppwinrt([
        "-in",
        "../component/lang.winmd",
        "../../../libs/bindgen/default",
        "-out",
        &include,
        "-optimize",
    ]);

    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .flag("/EHsc")
        .flag("/W4")
        .flag("/Ox")
        .flag("/GL")
        .file("src/bench.cpp")
        .include(include)
        .compile("lang_perf_cpp");
}
