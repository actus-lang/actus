# Actus LLVM 23.1.2 source-build cache.
# This file is the single source of truth for the T2 target and dependency policy.

set(CMAKE_BUILD_TYPE Release CACHE STRING "Actus LLVM build type")
set(LLVM_PARALLEL_LINK_JOBS 1 CACHE STRING "Actus serial LLVM linking")

set(LLVM_ENABLE_PROJECTS "clang;lld" CACHE STRING "Actus LLVM projects")
set(LLVM_TARGETS_TO_BUILD "X86;ARM;AArch64" CACHE STRING "Actus LLVM targets")
set(LLVM_ENABLE_RUNTIMES "compiler-rt" CACHE STRING "Actus LLVM runtimes")
set(LLVM_BUILD_LLVM_DYLIB OFF CACHE BOOL "Do not build LLVM shared library")
set(LLVM_BUILD_LLVM_C_DYLIB OFF CACHE BOOL "Do not build LLVM C shared library")
set(LLVM_ENABLE_ASSERTIONS OFF CACHE BOOL "Disable LLVM assertions")
set(LLVM_STATIC_LINK_CXX_STDLIB ON CACHE BOOL "Statically link the C++ standard library")

set(LLVM_ENABLE_ZLIB OFF CACHE BOOL "Actus disables zlib")
set(LLVM_ENABLE_ZSTD OFF CACHE BOOL "Actus disables zstd")
set(LLVM_ENABLE_LIBXML2 OFF CACHE BOOL "Actus disables libxml2")
set(LLVM_ENABLE_TERMINFO OFF CACHE BOOL "Actus disables terminfo")
set(LLVM_ENABLE_LIBEDIT OFF CACHE BOOL "Actus disables libedit")
set(LLVM_ENABLE_LIBPFM OFF CACHE BOOL "Actus disables libpfm")
set(LLVM_ENABLE_CURL OFF CACHE BOOL "Actus disables CURL")
set(LLVM_ENABLE_HTTPLIB OFF CACHE BOOL "Actus disables httplib")
set(LLVM_ENABLE_FFI OFF CACHE BOOL "Actus disables FFI")
set(LLVM_ENABLE_Z3_SOLVER OFF CACHE BOOL "Actus disables Z3")

set(LLVM_BUILTIN_TARGETS
    "thumbv7em-none-eabihf;thumbv7em-none-eabi"
    CACHE STRING "Actus freestanding compiler-rt builtin targets")
set(COMPILER_RT_BUILD_BUILTINS ON CACHE BOOL "Build compiler-rt builtins")
set(COMPILER_RT_BUILD_CRT OFF CACHE BOOL "Do not build CRT")
set(COMPILER_RT_BUILD_LIBFUZZER OFF CACHE BOOL "Do not build libFuzzer")
set(COMPILER_RT_BUILD_ORC OFF CACHE BOOL "Do not build ORC runtime")
set(COMPILER_RT_BUILD_CTX_PROFILE OFF CACHE BOOL "Do not build context profile")
set(COMPILER_RT_BUILD_GWP_ASAN OFF CACHE BOOL "Do not build GWP-ASan")
set(COMPILER_RT_BUILD_SANITIZERS OFF CACHE BOOL "Do not build sanitizers")
set(COMPILER_RT_BUILD_XRAY OFF CACHE BOOL "Do not build XRay")
set(COMPILER_RT_BUILD_MEMPROF OFF CACHE BOOL "Do not build MemProf runtime")
set(COMPILER_RT_BUILD_PROFILE OFF CACHE BOOL "Do not build profiling runtime")

set(BUILTINS_thumbv7em-none-eabihf_CMAKE_SYSTEM_NAME Generic CACHE STRING "Thumb hard-float system")
set(BUILTINS_thumbv7em-none-eabihf_COMPILER_RT_BAREMETAL_BUILD ON CACHE BOOL "Thumb hard-float baremetal")
set(BUILTINS_thumbv7em-none-eabihf_COMPILER_RT_DEFAULT_TARGET_ONLY ON CACHE BOOL "Thumb hard-float named target")
set(BUILTINS_thumbv7em-none-eabihf_LLVM_ENABLE_PER_TARGET_RUNTIME_DIR ON CACHE BOOL "Thumb hard-float isolated runtime directory")
set(BUILTINS_thumbv7em-none-eabihf_COMPILER_RT_OS_DIR baremetal CACHE STRING "Thumb hard-float OS directory")
set(BUILTINS_thumbv7em-none-eabihf_CMAKE_C_COMPILER_TARGET thumbv7em-none-eabihf CACHE STRING "Thumb hard-float compiler target")
set(BUILTINS_thumbv7em-none-eabihf_CMAKE_ASM_COMPILER_TARGET thumbv7em-none-eabihf CACHE STRING "Thumb hard-float assembler target")
set(BUILTINS_thumbv7em-none-eabihf_CMAKE_C_FLAGS "-mthumb -mcpu=cortex-m4 -mfloat-abi=hard -mfpu=fpv4-sp-d16" CACHE STRING "Thumb hard-float C flags")
set(BUILTINS_thumbv7em-none-eabihf_CMAKE_ASM_FLAGS "-mthumb -mcpu=cortex-m4 -mfloat-abi=hard -mfpu=fpv4-sp-d16" CACHE STRING "Thumb hard-float assembler flags")

set(BUILTINS_thumbv7em-none-eabi_CMAKE_SYSTEM_NAME Generic CACHE STRING "Thumb soft-float system")
set(BUILTINS_thumbv7em-none-eabi_COMPILER_RT_BAREMETAL_BUILD ON CACHE BOOL "Thumb soft-float baremetal")
set(BUILTINS_thumbv7em-none-eabi_COMPILER_RT_DEFAULT_TARGET_ONLY ON CACHE BOOL "Thumb soft-float named target")
set(BUILTINS_thumbv7em-none-eabi_LLVM_ENABLE_PER_TARGET_RUNTIME_DIR ON CACHE BOOL "Thumb soft-float isolated runtime directory")
set(BUILTINS_thumbv7em-none-eabi_COMPILER_RT_OS_DIR baremetal CACHE STRING "Thumb soft-float OS directory")
set(BUILTINS_thumbv7em-none-eabi_CMAKE_C_COMPILER_TARGET thumbv7em-none-eabi CACHE STRING "Thumb soft-float compiler target")
set(BUILTINS_thumbv7em-none-eabi_CMAKE_ASM_COMPILER_TARGET thumbv7em-none-eabi CACHE STRING "Thumb soft-float assembler target")
set(BUILTINS_thumbv7em-none-eabi_CMAKE_C_FLAGS "-mthumb -mcpu=cortex-m4 -mfloat-abi=soft" CACHE STRING "Thumb soft-float C flags")
set(BUILTINS_thumbv7em-none-eabi_CMAKE_ASM_FLAGS "-mthumb -mcpu=cortex-m4 -mfloat-abi=soft" CACHE STRING "Thumb soft-float assembler flags")

set(LLVM_INCLUDE_BENCHMARKS OFF CACHE BOOL "Do not build LLVM benchmarks")
set(LLVM_INCLUDE_EXAMPLES OFF CACHE BOOL "Do not build LLVM examples")
set(LLVM_INCLUDE_TESTS OFF CACHE BOOL "Do not build LLVM tests")
set(LLVM_INCLUDE_TOOLS ON CACHE BOOL "Build required LLVM tools")
set(LLVM_INCLUDE_UTILS ON CACHE BOOL "Build required LLVM utilities")

set(LLVM_TOOLCHAIN_TOOLS
    llvm-ar llvm-cxxfilt llvm-nm llvm-objdump llvm-readobj llvm-size llvm-symbolizer llc opt
    CACHE STRING "Actus inspection and archive tools")
set(LLVM_DISTRIBUTION_COMPONENTS
    llvm-headers llvm-libraries llvm-config lld
    builtins-thumbv7em-none-eabihf builtins-thumbv7em-none-eabi
    ${LLVM_TOOLCHAIN_TOOLS}
    CACHE STRING "Actus distribution components")

# Hosted platform policies are isolated from the Linux baseline.
if(APPLE)
    set(CMAKE_OSX_DEPLOYMENT_TARGET "13.0" CACHE STRING "Actus macOS deployment target" FORCE)
    set(CMAKE_OSX_ARCHITECTURES "arm64" CACHE STRING "Actus macOS archive architecture" FORCE)
    set(LLVM_STATIC_LINK_CXX_STDLIB OFF CACHE BOOL "Use the system macOS libc++" FORCE)
endif()

if(WIN32)
    set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreadedDLL" CACHE STRING "Use the dynamic MSVC CRT (/MD)" FORCE)
    set(CMAKE_C_FLAGS_RELEASE "/O2 /DNDEBUG" CACHE STRING "Release C flags without debug information" FORCE)
    set(CMAKE_CXX_FLAGS_RELEASE "/O2 /DNDEBUG" CACHE STRING "Release C++ flags without debug information" FORCE)
    set(LLVM_STATIC_LINK_CXX_STDLIB OFF CACHE BOOL "Use the MSVC runtime contract" FORCE)
endif()
