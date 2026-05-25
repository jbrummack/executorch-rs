use std::path::{Path, PathBuf};

// const EXECUTORCH_VERSION: &str = "1.1.0";
fn link_swift() {
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.0");
    println!("cargo:rustc-link-lib=swiftCompatibility56");
    println!("cargo:rustc-link-lib=swiftCompatibilityPacks");
    println!("cargo:rustc-link-lib=swiftCompatibilityDynamicReplacements");

    // Point the linker at the Swift stdlib location
    let xcode_path = std::process::Command::new("xcode-select")
        .arg("--print-path")
        .output()
        .unwrap();
    let xcode_path = String::from_utf8(xcode_path.stdout)
        .unwrap()
        .trim()
        .to_string();

    println!(
        "cargo:rustc-link-search={}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx",
        xcode_path
    );
}

fn prebuilt_path() -> Option<String> {
    //#[cfg(all(target_arch = "aarch64", target_os = "ios"))]
    let path = download_prebuilt::blocking_download_version(1, 1, 0).unwrap();
    link_swift();
    let path = PathBuf::from(path);
    if cfg!(all(
        target_vendor = "apple",
        target_os = "ios",
        target_env = "sim"
    )) {
        Some(path.join("ios-arm64-simulator"))
    } else if cfg!(all(target_vendor = "apple", target_os = "ios")) {
        Some(path.join("ios-arm64"))
    } else if cfg!(all(target_vendor = "apple", target_os = "macos")) {
        Some(path.join("macos-arm64"))
    } else {
        None
    }
    .map(|value| value.as_os_str().to_string_lossy().to_string())
}
fn main() {
    // TODO: verify on runtime we use the correct version of executorch
    // println!(
    //     "cargo:rustc-env=EXECUTORCH_RS_EXECUTORCH_VERSION={}",
    //     EXECUTORCH_VERSION
    // );

    //let macpath = PathBuf::from(dl_path).join("macos-arm64");
    let macpath = prebuilt_path();
    //link_swift();
    build_c_bridge();
    #[cfg(feature = "std")]
    build_cxx_bridge();
    generate_bindings();
    link_executorch(macpath);

    println!("cargo::rerun-if-changed={}", cpp_dir().to_str().unwrap());
    println!(
        "cargo::rerun-if-changed={}",
        third_party_dir().to_str().unwrap()
    );

    let check_cfg = rustc_version().map(|v| v >= 80).unwrap_or(false);
    println!("cargo::rerun-if-env-changed=EXECUTORCH_RS_DENY_WARNINGS");
    let deny_warnings = std::env::var("EXECUTORCH_RS_DENY_WARNINGS").as_deref() == Ok("1");
    if check_cfg {
        println!("cargo:rustc-check-cfg=cfg(deny_warnings)");
    }
    if deny_warnings {
        println!("cargo:rustc-cfg=deny_warnings");
    }
}

fn build_c_bridge() {
    let sources_dir = cpp_dir().join("executorch_rs");
    let mut builder = cc::Build::new();
    common_cc(&mut builder);
    builder
        .files([sources_dir.join("c_bridge.cpp")])
        .includes(cpp_includes());
    builder.compile(&format!(
        "executorch_rs_c_bridge_{}",
        env!("CARGO_PKG_VERSION")
    ));
}

#[cfg(feature = "std")]
fn build_cxx_bridge() {
    let sources_dir = cpp_dir().join("executorch_rs");
    let mut bridges = Vec::new();
    bridges.push("src/cxx_bridge/core.rs");
    if cfg!(feature = "module") {
        bridges.push("src/cxx_bridge/module.rs");
    }
    if cfg!(feature = "tensor-ptr") {
        bridges.push("src/cxx_bridge/tensor_ptr.rs");
    }
    let mut builder = cxx_build::bridges(bridges);
    common_cc(&mut builder);
    builder
        .files([sources_dir.join("cxx_bridge.cpp")])
        .includes(cpp_includes());
    builder.compile(&format!(
        "executorch_rs_cxx_bridge_{}",
        env!("CARGO_PKG_VERSION")
    ));
}

fn common_cc(builder: &mut cc::Build) {
    builder.cpp(true).std("c++17").cpp_link_stdlib(None); // linked via link-cplusplus crate
    if !cfg!(feature = "std") {
        // TODO: cpp executorch doesnt support nostd yet
        // builder.flag("-nostdlib");
    }
    for define in cpp_defines() {
        builder.define(define, None);
    }
}

fn generate_bindings() {
    let builder = bindgen::Builder::default()
        .clang_arg(format!("-I{}", cpp_dir().to_str().unwrap()))
        .clang_args(cpp_defines().iter().map(|d| format!("-D{d}")))
        .use_core()
        .generate_cstr(true)
        .header("cpp/executorch_rs/c_bridge.h")
        .allowlist_file("cpp/executorch_rs/c_bridge.h")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .no_copy(".*")
        .manually_drop_union(".*")
        .opaque_type("EValueStorage")
        .opaque_type("TensorStorage")
        .opaque_type("TensorImpl")
        .opaque_type("Program")
        .opaque_type("TensorInfo")
        .opaque_type("TensorLayout")
        .opaque_type("MethodMeta")
        .opaque_type("Method")
        .opaque_type("FlatTensorDataMap")
        .opaque_type("BufferDataLoader")
        .opaque_type("FileDataLoader")
        .opaque_type("MmapDataLoader")
        .opaque_type("MemoryAllocator")
        .opaque_type("HierarchicalAllocator")
        .opaque_type("MemoryManager")
        .opaque_type("OptionalTensorStorage")
        .opaque_type("ETDumpGen")
        .blocklist_item("FreeableBuffer")
        .blocklist_item(".*_bindgen_ty_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()));
    let bindings = builder.generate().expect("Unable to generate bindings");

    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("executorch_bindings.rs"))
        .expect("Couldn't write bindings!");
}
///Use XNNPACK Kernels
fn use_xnnpack() {
    println!("cargo::rustc-link-lib=static:+whole-archive=backend_xnnpack");
    println!("cargo::rustc-link-lib=static:+whole-archive=threadpool");
}
///Use optimised kernels
fn optimised_kernels() {
    //Needed for OpenMP
    println!("cargo:rustc-link-lib=framework=Accelerate");
    println!("cargo::rustc-link-lib=static:+whole-archive=kernels_optimized");
}
///Use quantised kernels
fn quantised_kernels() {
    println!("cargo::rustc-link-lib=static:+whole-archive=kernels_quantized");
}
///Use Torch AO
fn use_torch_ao() {
    println!("cargo::rustc-link-lib=static:+whole-archive=kernels_torchao");
}
fn use_coreml() {
    println!("cargo:rustc-link-lib=framework=Accelerate");
    println!("cargo:rustc-link-arg=-ObjC");
    // CoreML framework
    println!("cargo:rustc-link-lib=framework=CoreML");
    // SQLite symbols needed for CoreML
    println!("cargo:rustc-link-lib=sqlite3");
    println!("cargo::rustc-link-lib=static:+whole-archive=backend_coreml");
}
fn use_llm() {
    println!("cargo::rustc-link-lib=static:+whole-archive=executorch_llm");
}
fn use_llm_kernels() {
    println!("cargo::rustc-link-lib=static:+whole-archive=kernels_llm");
}
fn link_executorch(libdir: Option<String>) {
    println!("cargo::rerun-if-env-changed=EXECUTORCH_RS_EXECUTORCH_LIB_DIR");
    println!("cargo::rerun-if-env-changed=EXECUTORCH_RS_LINK");

    let link_enabled = std::env::var("EXECUTORCH_RS_LINK").as_deref() != Ok("0");

    let check_cfg = rustc_version().map(|v| v >= 80).unwrap_or(false);

    if check_cfg {
        println!("cargo::rustc-check-cfg=cfg(link_cxx)");
    }
    if link_enabled {
        println!("cargo::rustc-cfg=link_cxx");
    }

    if std::env::var("DOCS_RS").is_ok() || !link_enabled {
        // Skip linking to the static library when building documentation
        return;
    }
    let libs_dir = libdir.or(std::env::var("EXECUTORCH_RS_EXECUTORCH_LIB_DIR").ok());
    //let libs_dir = Some(libdir); //;
    if libs_dir.is_none() {
        println!("cargo::warning=EXECUTORCH_RS_EXECUTORCH_LIB_DIR is not set, can't locate executorch static libs");
    }

    if let Some(libs_dir) = &libs_dir {
        println!("cargo::rustc-link-search=native={libs_dir}");
    }
    println!("cargo::rustc-link-lib=static:+whole-archive=executorch");
    if cfg!(feature = "xnnpack") {
        use_xnnpack();
    }
    if cfg!(feature = "optimised_kernels") {
        optimised_kernels();
    }

    if cfg!(feature = "torch_ao") {
        use_torch_ao();
    }

    if cfg!(feature = "quantised_kernels") {
        quantised_kernels();
    }

    if cfg!(feature = "llm") {
        use_llm();
    }

    if cfg!(feature = "llm_kernels") {
        use_llm_kernels();
    }
    if cfg!(feature = "coreml") {
        use_coreml();
    }

    //Link executorch core only if it exists
    if let Some(libs_dir) = &libs_dir {
        let path = PathBuf::from(libs_dir).join("libexecutorch_core.a");
        if let Ok(true) = std::fs::exists(path) {
            println!("cargo::rustc-link-lib=static:+whole-archive=executorch_core");
        }
    } else {
        println!("cargo::rustc-link-lib=static:+whole-archive=executorch_core");
    }
    if cfg!(feature = "download_prebuilt") {
        return;
    }

    if cfg!(feature = "data-loader") {
        if let Some(libs_dir) = &libs_dir {
            println!("cargo::rustc-link-search=native={libs_dir}/extension/data_loader/");
        }
        println!("cargo::rustc-link-lib=static:+whole-archive=extension_data_loader");
    }

    if cfg!(feature = "module") {
        if let Some(libs_dir) = &libs_dir {
            println!("cargo::rustc-link-search=native={libs_dir}/extension/module/");
        }
        println!("cargo::rustc-link-lib=static:+whole-archive=extension_module_static");
    }

    let feature_named_data_map = cfg!(feature = "module");
    if feature_named_data_map {
        if let Some(libs_dir) = &libs_dir {
            println!("cargo::rustc-link-search=native={libs_dir}/extension/named_data_map/");
        }
        println!("cargo::rustc-link-lib=static:+whole-archive=extension_named_data_map");
    }

    if cfg!(feature = "flat-tensor") {
        if let Some(libs_dir) = &libs_dir {
            println!("cargo::rustc-link-search=native={libs_dir}/extension/flat_tensor/");
        }
        println!("cargo::rustc-link-lib=static:+whole-archive=extension_flat_tensor");
    }

    if cfg!(feature = "tensor-ptr") {
        if let Some(libs_dir) = &libs_dir {
            println!("cargo::rustc-link-search=native={libs_dir}/extension/tensor/");
        }
        println!("cargo::rustc-link-lib=static:+whole-archive=extension_tensor");
    }

    if cfg!(feature = "etdump") {
        if let Some(libs_dir) = &libs_dir {
            println!("cargo::rustc-link-search=native={libs_dir}/devtools/etdump/");
        }
        println!("cargo::rustc-link-lib=static:+whole-archive=etdump");
    }
}

fn cpp_dir() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).join("cpp")
}

fn third_party_dir() -> PathBuf {
    Path::new(&env!("CARGO_MANIFEST_DIR")).join("third-party")
}

fn cpp_includes() -> Vec<PathBuf> {
    let third_party_dir = third_party_dir();
    let c10_dir = std::env::var_os("EXECUTORCH_RS_C10_HEADERS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| third_party_dir.join("executorch/runtime/core/portable_type/c10"));
    assert!(
        c10_dir.exists(),
        "C10 directory does not exist: {}",
        c10_dir.display()
    );
    vec![cpp_dir(), third_party_dir.clone(), c10_dir]
}

fn cpp_defines() -> Vec<&'static str> {
    let mut defines = vec!["C10_USING_CUSTOM_GENERATED_MACROS"];
    if cfg!(feature = "data-loader") {
        defines.push("EXECUTORCH_RS_DATA_LOADER");
    }
    if cfg!(feature = "flat-tensor") {
        defines.push("EXECUTORCH_RS_FLAT_TENSOR");
    }
    if cfg!(feature = "module") {
        defines.push("EXECUTORCH_RS_MODULE");
    }
    if cfg!(feature = "tensor-ptr") {
        defines.push("EXECUTORCH_RS_TENSOR_PTR");
    }
    if cfg!(feature = "etdump") {
        defines.push("EXECUTORCH_RS_ETDUMP");
    }
    if cfg!(feature = "std") {
        defines.push("EXECUTORCH_RS_STD");
    }
    defines
}

fn rustc_version() -> Option<u32> {
    // Code copied from cxx crate

    let rustc = std::env::var_os("RUSTC")?;
    let output = std::process::Command::new(rustc)
        .arg("--version")
        .output()
        .ok()?;
    let version = String::from_utf8(output.stdout).ok()?;
    let mut pieces = version.split('.');
    if pieces.next() != Some("rustc 1") {
        return None;
    }
    let minor = pieces.next()?.parse().ok()?;
    Some(minor)
}
