use std::{collections::HashSet, path::PathBuf};

pub enum AppleTarget {
    IosArm64,
    SimulatorArm64,
    MacOsArm64,
}
pub enum AndroidTarget {
    AndroidX86,
    AndroidArm64,
}
pub enum Target {
    Android(AndroidTarget),
    Apple(AppleTarget),
}
impl AppleTarget {
    pub fn new() -> Self {
        let target = std::env::var("TARGET").unwrap_or_default();
        if target.contains("sim") {
            Self::SimulatorArm64
        } else if target.contains("ios") {
            Self::IosArm64
        } else if target.contains("macos") || target.contains("darwin") {
            Self::MacOsArm64
        } else {
            panic!("Unsupported prebuilt apple platform: {target}")
        }
    }
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
}
impl AndroidTarget {
    pub fn new() -> Self {
        let target = std::env::var("TARGET").unwrap_or_default();
        if target.contains("aarch64") {
            Self::AndroidArm64
        } else if target.contains("x86_64") {
            Self::AndroidX86
        } else {
            panic!("Unsupported prebuilt android platform: {target}")
        }
    }
}
impl Target {
    pub fn new() -> Self {
        let target = std::env::var("TARGET").unwrap_or_default();
        if target.contains("android") {
            Self::Android(AndroidTarget::new())
        } else if target.contains("apple") {
            Self::Apple(AppleTarget::new())
        } else {
            panic!("Unsupported prebuilt platform: {target}")
        }
    }
    pub fn link_target(&self, a: u8, b: u8, c: u8) {
        let dl_path = self.download(a, b, c);
        let libs_dir = dl_path.to_string_lossy();
        match self {
            Target::Android(_) => {
                // build.rs
                println!("cargo:rustc-link-search=native={libs_dir}");
                println!("cargo:rustc-link-lib=dylib=executorch");
            }
            Target::Apple(_) => {
                AppleTarget::link_swift();
                println!("cargo::rustc-link-search=native={libs_dir}");
                println!("cargo::rustc-link-lib=static:+whole-archive=executorch");
            }
        }
    }

    pub fn download(&self, a: u8, b: u8, c: u8) -> PathBuf {
        match self {
            Target::Android(android_target) => {
                let path = download_prebuilt::blocking_download_android(a, b, c).unwrap();

                let arch = match android_target {
                    AndroidTarget::AndroidX86 => path.join("x86_64"),
                    AndroidTarget::AndroidArm64 => path.join("aarch64"),
                };
                arch
            }
            Target::Apple(apple_target) => {
                let path = download_prebuilt::blocking_download_version(a, b, c, true).unwrap();
                let path = PathBuf::from(path);
                let arch = match apple_target {
                    AppleTarget::IosArm64 => "ios-arm64",
                    AppleTarget::SimulatorArm64 => "ios-arm64-simulator",
                    AppleTarget::MacOsArm64 => "macos-arm64",
                };
                path.join(arch)
            }
        }
    }
}
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Feature {
    XNNPack,
    CoreML,
    OptimisedKernels,
    QuantisedKernels,
    Accelerate,
    LlmKernels,
    Llm,
    TorchAO,
    Threadpool,
    //Vulkan, //for android
    //CUDA
}
impl Feature {
    ///Android comes prelinked with all needed features
    ///Link needed features for Apple platforms
    pub fn get_features() -> HashSet<Feature> {
        if let Target::Android(_) = Target::new() {
            return HashSet::new();
        }
        let mut features = HashSet::new();
        if cfg!(feature = "xnnpack") {
            features.insert(Self::XNNPack);
            features.insert(Self::Threadpool);
        }
        if cfg!(feature = "optimised_kernels") {
            features.insert(Self::Accelerate);
            features.insert(Self::OptimisedKernels);
        }

        if cfg!(feature = "torch_ao") {
            features.insert(Self::TorchAO);
        }

        if cfg!(feature = "quantised_kernels") {
            features.insert(Self::QuantisedKernels);
        }

        if cfg!(feature = "llm") {
            features.insert(Self::Llm);
        }

        if cfg!(feature = "llm_kernels") {
            features.insert(Self::LlmKernels);
        }
        if cfg!(feature = "coreml") {
            features.insert(Self::Accelerate);
            features.insert(Self::CoreML);
        }
        features
    }
    fn link(&self, prebuilt_dir: &PathBuf) {
        match self {
            Feature::Threadpool => {
                println!("cargo::rustc-link-lib=static:+whole-archive=threadpool");
            }
            Feature::XNNPack => {
                println!("cargo::rustc-link-lib=static:+whole-archive=backend_xnnpack");
            }
            //Needs accelerate
            Feature::CoreML => {
                let coreml = prebuilt_dir.join("libbackend_coreml.a");
                // -force_load is the Apple equivalent of --whole-archive for a single lib.
                // It forces every object file in the archive to be included, preventing
                // the linker from dropping backend_delegate.o which has no direct callers.
                println!("cargo:rustc-link-arg=-force_load");
                println!("cargo:rustc-link-lib=static:+whole-archive=backend_coreml");
                println!("cargo:rustc-link-arg={}", coreml.display());
                println!("cargo:rustc-link-arg=-ObjC");
                println!("cargo:rustc-link-lib=framework=CoreML");
                println!("cargo:rustc-link-lib=sqlite3");
            }
            //Needs accelerate
            Feature::OptimisedKernels => {
                println!("cargo::rustc-link-lib=static:+whole-archive=kernels_optimized");
            }
            Feature::QuantisedKernels => {
                println!("cargo::rustc-link-lib=static:+whole-archive=kernels_quantized");
            }
            Feature::Accelerate => {
                println!("cargo:rustc-link-lib=framework=Accelerate");
            }
            Feature::LlmKernels => {
                println!("cargo::rustc-link-lib=static:+whole-archive=kernels_llm");
            }
            Feature::Llm => {
                println!("cargo::rustc-link-lib=static:+whole-archive=executorch_llm");
            }
            Feature::TorchAO => {
                println!("cargo::rustc-link-lib=static:+whole-archive=kernels_torchao");
            }
        }
    }
}
pub fn static_lib_merge_step(target: Target, dl_path: &PathBuf, features: &HashSet<Feature>) {
    match &target {
        Target::Apple(apple_target) => {
            AppleTarget::link_swift();

            // Only merge on iOS — macOS links directly fine
            let needs_merge = matches!(
                apple_target,
                AppleTarget::IosArm64 | AppleTarget::SimulatorArm64
            );

            if needs_merge {
                let merged = merge_apple_libs(&dl_path, &features);
                println!(
                    "cargo::rustc-link-search=native={}",
                    merged.parent().unwrap().display()
                );
                println!(
                    "cargo::rustc-link-lib=static:+whole-archive={}",
                    merged
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .trim_start_matches("lib")
                );
            } else {
                // macOS: link as before
                println!("cargo::rustc-link-search=native={}", dl_path.display());
                println!("cargo::rustc-link-lib=static:+whole-archive=executorch");
                for feature in features.clone() {
                    feature.link(dl_path);
                }
            }
        }
        Target::Android(_) => {
            println!("cargo:rustc-link-search=native={}", dl_path.display());
            println!("cargo:rustc-link-lib=dylib=executorch");
        }
    }
}

/// Merges all required .a files for an iOS slice into a single archive
/// using `libtool -static`, so the Apple linker cannot dead-strip
/// static initializers (e.g. CoreML backend registration) during the
/// xcframework → Xcode link hop.
fn merge_apple_libs(prebuilt_dir: &PathBuf, features: &HashSet<Feature>) -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let merged_path = out_dir.join("libexecutorch_merged.a");

    let mut libs: Vec<PathBuf> = vec![prebuilt_dir.join("libexecutorch.a")];

    for feature in features {
        let name = match feature {
            Feature::CoreML => Some("libbackend_coreml.a"),
            Feature::XNNPack => Some("libbackend_xnnpack.a"),
            Feature::Threadpool => Some("libthreadpool.a"),
            Feature::OptimisedKernels => Some("libkernels_optimized.a"),
            Feature::QuantisedKernels => Some("libkernels_quantized.a"),
            Feature::LlmKernels => Some("libkernels_llm.a"),
            Feature::Llm => Some("libexecutorch_llm.a"),
            Feature::TorchAO => Some("libkernels_torchao.a"),
            // Accelerate is a framework, not a .a
            Feature::Accelerate => None,
        };
        if let Some(name) = name {
            libs.push(prebuilt_dir.join(name));
        }
    }

    // Emit framework links separately — these can't go into libtool
    if features.contains(&Feature::CoreML) {
        println!("cargo:rustc-link-arg=-ObjC");
        println!("cargo:rustc-link-lib=framework=CoreML");
        println!("cargo:rustc-link-lib=sqlite3");
    }
    if features.contains(&Feature::Accelerate) {
        println!("cargo:rustc-link-lib=framework=Accelerate");
    }

    let status = std::process::Command::new("libtool")
        .arg("-static")
        .arg("-o")
        .arg(&merged_path)
        .args(&libs)
        .status()
        .expect("failed to run libtool — is Xcode installed?");

    assert!(status.success(), "libtool merge failed");

    // Tell cargo to rerun if any input lib changes
    for lib in &libs {
        println!("cargo:rerun-if-changed={}", lib.display());
    }

    merged_path
}
pub fn link_prebuilts(a: u8, b: u8, c: u8) {
    let target = Target::new();
    let dl_path = target.download(a, b, c);
    /*for feature in Feature::get_features() {
        feature.link();
    }*/
    //target.link_target(a, b, c);
    static_lib_merge_step(target, &dl_path, &Feature::get_features());
}
