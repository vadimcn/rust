// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! General DWARF info reading.

pub mod eh;

use prelude::v1::*;
use core::mem;

pub struct DwarfReader {
    pub ptr : *const u8
}

impl DwarfReader {

    pub fn new(pstart : *const u8) -> DwarfReader {
        DwarfReader {
            ptr : pstart
        }
    }

    // for platforms that allow unaligned reads
    unsafe fn read<T:Copy>(&mut self) -> T {
        let result : T = *(self.ptr as *const T);
        self.ptr = self.ptr.offset(mem::size_of::<T>() as isize);
        result
    }

    pub unsafe fn read_uleb128(&mut self) -> u64 {
        let mut shift : usize = 0;
        let mut result : u64 = 0;
        let mut byte : u8;
        loop {
            byte = *self.ptr;
            self.ptr = self.ptr.offset(1);
            result |= ((byte & 0x7F) as u64) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }
        result
    }

    pub unsafe fn read_sleb128(&mut self) -> i64 {
        let mut shift : usize = 0;
        let mut result : u64 = 0;
        let mut byte : u8;
        loop {
            byte = *self.ptr;
            self.ptr = self.ptr.offset(1);
            result |= ((byte & 0x7F) as u64) << shift;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }
        // sign-extend
        if shift < 8 * mem::size_of::<u64>() && (byte & 0x40) != 0 {
            result |= (!0 as u64) << shift;
        }
        result as i64
    }
}
/*
// for platforms that fault on unaligned read
unsafe fn unaligned_read<T : Int>(pptr : &mut *const u8) -> T {
    let mut ptr = *pptr;
    let mut result : T = num::Zero::zero();
    for _ in range(0, mem::size_of::<T>()) {
        result = (result << 8) | (*ptr as T);
        ptr = ptr.offset(1);
    }
    *pptr = ptr;
    Int::from_be(result)
}
*/