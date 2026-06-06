#import <objc/runtime.h>
#import <objc/message.h>
// cpp/executorch_rs/coreml_registration.mm
#include <executorch/backends/apple/coreml/runtime/delegate/executorch_operations.h>
void patch_coreml_prewarm() {
    Class cls = NSClassFromString(@"MLDelegateModel");
    if (!cls) {
        NSLog(@"[CoreMLPatch] MLDelegateModel not found");
        return;
    }
    SEL sel = @selector(prewarmUsingState:error:);
    if (![cls instancesRespondToSelector:sel]) {
        NSLog(@"[CoreMLPatch] patching");
        IMP imp = imp_implementationWithBlock(^BOOL(id self, id state, NSError **err) {
            return YES;
        });
        class_addMethod(cls, sel, imp, "B@:@@");
    } else {
        NSLog(@"[CoreMLPatch] not needed");
    }
}
extern "C" void register_coreml_backend() {
    patch_coreml_prewarm();
    executorch::core_ml_backend_delegate::register_backend_coreml();
}
