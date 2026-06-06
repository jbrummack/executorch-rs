#pragma once

#include "executorch_operations.h"
#import <coreml_backend/delegate.h>
#import "ETCoreMLStrings.h"
#import "backend_delegate.h"

#import <executorch/runtime/core/evalue.h>
#import <executorch/runtime/platform/log.h>
#import <executorch/runtime/backend/interface.h>

#include <array>
#import <memory>


// Patch for ExecuTorch prebuilts targeting iOS 17 — prewarmUsingState:error:
// is iOS 18+ only but called without availability guard in the prebuilt delegate.
// Prewarming is a performance hint only, no-oping is safe.
@interface NSObject (CoreMLPrewarmPatch)
@end
@implementation NSObject (CoreMLPrewarmPatch)
+ (void)load {
    Class cls = NSClassFromString(@"MLDelegateModel");
    if (!cls) {
        NSLog(@"[CoreMLPatch] MLDelegateModel not found — iOS 26 may have renamed it");
        return;
    }
    SEL sel = @selector(prewarmUsingState:error:);
    if (![cls instancesRespondToSelector:sel]) {
        NSLog(@"[CoreMLPatch] patching prewarmUsingState:error: onto MLDelegateModel");
        IMP imp = imp_implementationWithBlock(^BOOL(id self, id state, NSError **err) {
            return YES;
        });
        class_addMethod(cls, sel, imp, "B@:@@");
    } else {
        NSLog(@"[CoreMLPatch] prewarmUsingState:error: already exists, no patch needed");
    }
}
@end

namespace executorch::core_ml_backend_delegate {
  using executorch::runtime::get_backend_class;

static std::unique_ptr<executorch::backends::coreml::CoreMLBackendDelegate> backendInterfaceLazy_;

void register_backend_coreml() {
    auto backendInterface = executorch::runtime::get_backend_class(ETCoreMLStrings.delegateIdentifier.UTF8String);
    if (backendInterface == nullptr) {
      backendInterfaceLazy_ = std::make_unique<executorch::backends::coreml::CoreMLBackendDelegate>();
      executorch::runtime::Backend backend{ETCoreMLStrings.delegateIdentifier.UTF8String, backendInterfaceLazy_.get()};
      std::ignore = register_backend(backend);
    }
  }

} // namespace executorch::core_ml_backend_delegate
