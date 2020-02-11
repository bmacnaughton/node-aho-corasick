Using perf:

```bash
perf record -g node --perf-basic-prof wasm.js
perf report --no-children
```

Results:

```
+    9.09%  node     [JIT] tid 6577       [.] Function:fast_match::Matcher::run::hdc325dfd51ff7151-19
-    3.79%  node     node                 [.] v8::internal::Heap::RegisterNewArrayBuffer
   - 3.76% v8::internal::Heap::RegisterNewArrayBuffer
      - 3.73% v8::internal::JSArrayBuffer::Setup
           v8::ArrayBuffer::New
           node::Buffer::New
           node::Buffer::Copy
           node::i18n::(anonymous namespace)::ToBufferEndian<char16_t>
         - node::i18n::(anonymous namespace)::ConverterObject::Decode
            - 3.73% Builtins_CallApiCallback
               - 3.73% LazyCompile:*module.exports.__wbindgen_string_new /home/gripsou/Documents/projects/fast-match/pkg/fast_match.js:135
                    0x32917cdb9994
                    Function:fast_match::Matcher::run::hdc325dfd51ff7151-19
                    Function:matcher_run-59
                  - Stub:js_to_wasm:iii:i
                     + 3.24% LazyCompile:* /home/gripsou/Documents/projects/fast-match/node/wasm.js:9
+    2.87%  node     libc-2.30.so         [.] malloc
-    2.80%  node     node                 [.] v8::internal::ArrayBufferTracker::PrepareToFreeDeadInNewSpace
     2.78% v8::internal::ArrayBufferTracker::PrepareToFreeDeadInNewSpace
        v8::internal::ScavengerCollector::CollectGarbage
        v8::internal::Heap::Scavenge
        v8::internal::Heap::PerformGarbageCollection
        v8::internal::Heap::CollectGarbage
      - v8::internal::Heap::AllocateRawWithRetryOrFail
         - 1.10% v8::internal::Factory::NewFillerObject
              v8::internal::Runtime_AllocateInYoungGeneration
            - Builtins_CEntry_Return1_DontSaveFPRegs_ArgvOnStack_NoBuiltinExit
               - 0.92% Builtins_CreateTypedArray
                    Builtins_TypedArrayPrototypeSubArray
                    LazyCompile:*module.exports.__wbindgen_string_new /home/gripsou/Documents/projects/fast-match/pkg/fast_match.js:135
                    0x32917cdb9994
                    Function:fast_match::Matcher::run::hdc325dfd51ff7151-19
                    Function:matcher_run-59
                  + Stub:js_to_wasm:iii:i
         + 0.89% v8::internal::Factory::NewJSArrayBufferView
         + 0.64% v8::internal::Factory::NewJSArrayBuffer
+    2.76%  node     libc-2.30.so         [.] _int_malloc
+    2.61%  node     [JIT] tid 6577       [.] LazyCompile:*module.exports.__wbindgen_string_new /home/gripsou/Documents/projects/fast-match/pkg/fast_match.js:135
+    2.29%  node     libc-2.30.so         [.] _int_free
+    2.24%  node     node                 [.] std::_Hashtable<v8::internal::JSArrayBuffer, std::pair<v8::internal::JSArrayBuffer const, v8::internal::JSArrayBuffer::Allocation>, std
+    2.22%  node     node                 [.] Builtins_CreateTypedArray
+    2.20%  node     node                 [.] Builtins_CallApiCallback
```

Regexp 1 batch:

```
real    3m29.504s
user    3m28.918s
sys     0m0.387s
```
