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

// compile-flags:-g

// === GDB TESTS ===================================================================================

// gdb-command:run
// gdb-command:next
// gdb-check:MARKER1
// gdb-command:next
// gdb-check:MARKER2
// gdb-command:next
// gdb-check:MARKER3
// gdb-command:next
// gdb-check:MARKER4
// gdb-command:next
// gdb-check:MARKER5

// === LLDB TESTS ==================================================================================

// lldb-command:run
// lldb-command:next
// lldb-check:MARKER1
// lldb-command:next
// lldb-check:MARKER2
// lldb-command:next
// lldb-check:MARKER3
// lldb-command:next
// lldb-check:MARKER4
// lldb-command:next
// lldb-check:MARKER5

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
    foo!(); // MARKER1
    foo2!(); // MARKER2
    let x = vec![42]; // MARKER3
    println!("Hello world"); // MARKER4
    zzz(); // MARKER5
}

fn zzz() {()}
