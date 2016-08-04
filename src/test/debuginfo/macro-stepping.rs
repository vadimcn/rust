// Copyright 2013-2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-windows
// min-lldb-version: 310

// compile-flags:-g -Zorbit

// === GDB TESTS ===================================================================================

// gdb-command:run
// gdb-command:next
// gdb-command:f
// gdb-check:[...]#loc1[...]
// gdb-command:next
// gdb-command:f
// gdb-check:[...]#loc2[...]
// gdb-command:next
// gdb-command:f
// gdb-check:[...]#loc3[...]
// gdb-command:next
// gdb-command:f
// gdb-check:[...]#loc4[...]
// gdb-command:next
// gdb-command:f
// gdb-check:[...]#loc5[...]

// === LLDB TESTS ==================================================================================

// lldb-command:set set stop-line-count-before 0
// lldb-command:set set stop-line-count-after 1
// Can't set both to zero or lldb will stop printing source at all.  So it will output the current
// line and the next.  We deal with this by having at least 2 lines between the #loc's

// lldb-command:run
// lldb-command:next
// lldb-command:f
// lldb-check:[...]#loc1[...]
// lldb-command:next
// lldb-command:f
// lldb-check:[...]#loc2[...]
// lldb-command:next
// lldb-command:f
// lldb-check:[...]#loc3[...]
// lldb-command:next
// lldb-command:f
// lldb-check:[...]#loc4[...]
// lldb-command:next
// lldb-command:f
// lldb-check:[...]#loc5[...]

#![allow(unused)]

macro_rules! foo {
    () => {
        let a = 1;
        let b = 2;
        let c = 3;
    }
}

macro_rules! foo2 {
    () => {
        foo!();
        let x = 1;
        foo!();
    }
}

fn main() {
    zzz(); // #break

    foo!(); // #loc1

    foo2!(); // #loc2

    let x = vec![42]; // #loc3

    println!("Hello {}", "world"); // #loc4

    zzz(); // #loc5
}

fn zzz() {()}
