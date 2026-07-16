fn main() {
    msvc_main();
}

#[cfg(not(target_env = "msvc"))]
fn msvc_main() {}

#[cfg(target_env = "msvc")]
fn msvc_main() {
    println!("cargo:rerun-if-changed=../component/lang.winmd");
    println!("cargo:rerun-if-changed=src/component.cpp");
    println!("cargo:rustc-link-lib=onecoreuap");
    println!("cargo:rustc-link-arg-cdylib=/export:DllGetActivationFactory");
    // Max-optimization link (see cpp/build.rs): /GL objects need /LTCG, plus
    // /OPT:REF (drop unreferenced) and /OPT:ICF (fold identical COMDATs).
    println!("cargo:rustc-link-arg-cdylib=/LTCG");
    println!("cargo:rustc-link-arg-cdylib=/OPT:REF");
    println!("cargo:rustc-link-arg-cdylib=/OPT:ICF");

    let include = std::env::var("OUT_DIR").unwrap();
    let reference = "../../../libs/bindgen/default";

    cppwinrt::cppwinrt([
        "-in",
        "../component/lang.winmd",
        reference,
        "-out",
        &include,
        "-optimize",
    ]);

    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .flag("/EHsc")
        .flag("/W4")
        .flag("/WX")
        .flag("/Ox")
        .flag("/GL")
        .file("src/component.cpp")
        .include(include)
        .compile("component");
}
