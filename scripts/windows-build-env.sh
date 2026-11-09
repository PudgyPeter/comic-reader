#!/bin/sh
# Environment for cross-building the Windows exe from Linux (cargo-xwin).
# Usage: source scripts/windows-build-env.sh && npx tauri build --target x86_64-pc-windows-msvc
export PATH="$HOME/.local/bin:/usr/lib/llvm-18/bin:$HOME/.cargo/bin:$PATH"
export XWIN_CACHE_DIR="$HOME/.cache/cargo-xwin"
export AR_x86_64_pc_windows_msvc="llvm-lib"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER="lld-link"
export CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C linker-flavor=lld-link -Lnative=$HOME/.cache/cargo-xwin/xwin/crt/lib/x86_64 -Lnative=$HOME/.cache/cargo-xwin/xwin/sdk/lib/um/x86_64 -Lnative=$HOME/.cache/cargo-xwin/xwin/sdk/lib/ucrt/x86_64"
export CC_x86_64_pc_windows_msvc="clang-cl"
export CXX_x86_64_pc_windows_msvc="clang-cl"
export CFLAGS_x86_64_pc_windows_msvc="--target=x86_64-pc-windows-msvc -Wno-unused-command-line-argument -fuse-ld=lld-link /imsvc $HOME/.cache/cargo-xwin/xwin/crt/include /imsvc $HOME/.cache/cargo-xwin/xwin/sdk/include/ucrt /imsvc $HOME/.cache/cargo-xwin/xwin/sdk/include/um /imsvc $HOME/.cache/cargo-xwin/xwin/sdk/include/shared /imsvc $HOME/.cache/cargo-xwin/xwin/sdk/include/winrt"
export CXXFLAGS_x86_64_pc_windows_msvc="$CFLAGS_x86_64_pc_windows_msvc /EHsc"
export LIB="$HOME/.cache/cargo-xwin/xwin/crt/lib/x86_64;$HOME/.cache/cargo-xwin/xwin/sdk/lib/um/x86_64;$HOME/.cache/cargo-xwin/xwin/sdk/lib/ucrt/x86_64"
export RCFLAGS="-I$HOME/.cache/cargo-xwin/xwin/crt/include -I$HOME/.cache/cargo-xwin/xwin/sdk/include/ucrt -I$HOME/.cache/cargo-xwin/xwin/sdk/include/um -I$HOME/.cache/cargo-xwin/xwin/sdk/include/shared -I$HOME/.cache/cargo-xwin/xwin/sdk/include/winrt"
export TARGET_AR="llvm-lib"
export TARGET_CC="clang-cl"
export TARGET_CXX="clang-cl"
