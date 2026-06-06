#include <cstring>
#include <executorch/runtime/backend/interface.h>

namespace executorch::runtime {
    extern Error register_backend(const Backend& backend);
}
extern "C" void ensure_backends_registered() {
    // These calls create a live reference to the backend registry,
    // forcing the linker to include the registration TUs from the prebuilt libs.
    // The return value doesn't matter — it will be nullptr before init,
    // but the reference is what prevents dead-stripping.
    (void)executorch::runtime::get_backend_class("CoreMLBackend");
    (void)executorch::runtime::get_backend_class("XnnpackBackend");
}
__attribute__((constructor))
static void ensure_backends_registered_ctor() {
    // Calling get_backend_class references the backend registry
    // which pulls in the registration TUs from the prebuilt libs
    auto* coreml = executorch::runtime::get_backend_class("CoreMLBackend");
    auto* xnnpack = executorch::runtime::get_backend_class("XnnpackBackend");
    (void)coreml;
    (void)xnnpack;
}
/*void efpc_register_backends() {
    // Explicit registration bypasses the static initializer entirely
    static auto coreml_delegate = executorchcoreml::BackendDelegate::make({});
    torch::executor::Backend coreml_backend{
        "CoreMLBackend",                    // must match the name the backend registers as
        coreml_delegate.get()               // the PyTorchBackendInterface*
    };

    torch::executor::register_backend(coreml_backend);
    }*/
using namespace executorch::ET_RUNTIME_NAMESPACE;

extern "C" {

size_t et_backend_count() {
    return get_num_registered_backends();
}

const char* et_backend_name(size_t i) {
    auto result = get_backend_name(i);
    if (!result.ok()) {
        return nullptr;
    }
    return *result;
}

}
