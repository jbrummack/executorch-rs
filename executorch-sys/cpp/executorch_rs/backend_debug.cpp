#include <cstring>
#include <executorch/runtime/backend/interface.h>

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
