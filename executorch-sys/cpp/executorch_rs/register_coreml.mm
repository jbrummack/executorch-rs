// cpp/executorch_rs/coreml_registration.mm
#include <executorch/backends/apple/coreml/runtime/delegate/executorch_operations.h>

extern "C" void register_coreml_backend() {
    executorch::core_ml_backend_delegate::register_backend_coreml();
}
