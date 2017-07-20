// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use {LinkerFlavor, PanicStrategy};
use super::{Target, TargetOptions};
use super::emscripten_base::{cmd};

pub fn target() -> Result<Target, String> {
    let opts = TargetOptions {
        linker: cmd("lld"),
        ar: cmd("llvm-ar"),

        llvm_args: vec!["-thread-model=single".to_string()], // LLVM bug 27124
        dynamic_linking: false,
        executables: true,
        exe_suffix: ".wasm".to_string(),
        dll_suffix: ".wasm".to_string(),
        linker_is_gnu: true,
        allow_asm: false,
        obj_is_bitcode: false,
        is_like_emscripten: false,
        max_atomic_width: Some(32),
        target_family: Some("unix".to_string()),
        panic_strategy: PanicStrategy::Abort,
        .. Default::default()
    };
    Ok(Target {
        llvm_target: "wasm32-unknown-unknown-wasm".to_string(),
        target_endian: "little".to_string(),
        target_pointer_width: "32".to_string(),
        target_os: "emscripten".to_string(),
        target_env: "".to_string(),
        target_vendor: "unknown".to_string(),
        data_layout: "e-m:e-p:32:32-i64:64-n32:64-S128".to_string(),
        arch: "wasm32".to_string(),
        linker_flavor: LinkerFlavor::Wasm,
        options: opts,
    })
}
