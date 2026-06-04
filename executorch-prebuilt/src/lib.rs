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
                let path = download_prebuilt::blocking_download_version(a, b, c).unwrap();
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
    fn link(&self) {
        match self {
            Feature::Threadpool => {
                println!("cargo::rustc-link-lib=static:+whole-archive=threadpool");
            }
            Feature::XNNPack => {
                println!("cargo::rustc-link-lib=static:+whole-archive=backend_xnnpack");
            }
            //Needs accelerate
            Feature::CoreML => {
                println!("cargo:rustc-link-arg=-ObjC");
                // CoreML framework
                println!("cargo:rustc-link-lib=framework=CoreML");
                // SQLite symbols may be needed for CoreML
                println!("cargo:rustc-link-lib=sqlite3");
                println!("cargo::rustc-link-lib=static:+whole-archive=backend_coreml");
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

pub fn link_prebuilts(a: u8, b: u8, c: u8) {
    let target = Target::new();
    target.download(a, b, c);
    for feature in Feature::get_features() {
        feature.link();
    }
    target.link_target(a, b, c);
}
