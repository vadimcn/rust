// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unwinding implementation of top of native Win64 SEH,
//! however the unwind handler data (aka LSDA) uses GCC-compatible encoding.

#![allow(non_snake_case)]
#![allow(improper_ctypes)]

use prelude::v1::*;

use any::Any;
use self::EXCEPTION_DISPOSITION::*;
use rt::dwarf::eh;
use core::mem;
use core::ptr;
use simd;
use libc::{c_void, DWORD};

// Define our exception codes:
// according to http://msdn.microsoft.com/en-us/library/het71c37(v=VS.80).aspx,
//    [31:30] = 3 (error), 2 (warning), 1 (info), 0 (success)
//    [29]    = 1 (user-defined)
//    [28]    = 0 (reserved)
// we define bits:
//    [24:27] = type
//    [0:23]  = magic
const ETYPE: DWORD = 0b1110_u32 << 28;
const MAGIC: DWORD = 0x525354; // "RST"

const RUST_PANIC: DWORD  = ETYPE | (1 << 24) | MAGIC;

const EXCEPTION_NONCONTINUABLE: DWORD = 0x1;    // Noncontinuable exception
const EXCEPTION_UNWINDING: DWORD = 0x2;         // Unwind is in progress
const EXCEPTION_EXIT_UNWIND: DWORD = 0x4;       // Exit unwind is in progress
const EXCEPTION_STACK_INVALID: DWORD = 0x8;     // Stack out of limits or unaligned
const EXCEPTION_NESTED_CALL: DWORD = 0x10;      // Nested exception handler call
const EXCEPTION_TARGET_UNWIND: DWORD = 0x20;    // Target unwind in progress
const EXCEPTION_COLLIDED_UNWIND: DWORD = 0x40;  // Collided exception handler call
const EXCEPTION_UNWIND: DWORD = EXCEPTION_UNWINDING | EXCEPTION_EXIT_UNWIND |
                                EXCEPTION_TARGET_UNWIND | EXCEPTION_COLLIDED_UNWIND;

#[repr(C)]
pub struct EXCEPTION_RECORD {
    ExceptionCode: DWORD,
    ExceptionFlags: DWORD,
    ExceptionRecord: *const EXCEPTION_RECORD,
    ExceptionAddress: usize,
    NumberParameters: DWORD,
    ExceptionInformation: [usize; 15],
}

#[repr(C)]
pub struct CONTEXT;

#[repr(C)]
pub struct UNWIND_HISTORY_TABLE;

#[repr(C)]
pub struct RUNTIME_FUNCTION {
	BeginAddress: DWORD,
    EndAddress: DWORD,
    UnwindData: DWORD,
}

#[repr(C)]
pub struct DISPATCHER_CONTEXT {
    ControlPc: usize,
    ImageBase: usize,
    FunctionEntry: *const RUNTIME_FUNCTION,
    EstablisherFrame: usize,
    TargetIp: usize,
    ContextRecord: *const CONTEXT,
    LanguageHandler: *const u8,
    HandlerData: *const u8,
    HistoryTable: *const UNWIND_HISTORY_TABLE,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub enum EXCEPTION_DISPOSITION {
    ExceptionContinueExecution,
    ExceptionContinueSearch,
    ExceptionNestedException,
    ExceptionCollidedUnwind
}

// From kernel32.dll
extern "system" {
    fn RaiseException(dwExceptionCode: DWORD,
                      dwExceptionFlags: DWORD,
                      nNumberOfArguments: DWORD,
                      lpArguments: *const usize);

    fn RtlUnwindEx(TargetFrame: usize,
    			   TargetIp: usize,
                   ExceptionRecord: *const EXCEPTION_RECORD,
                   ReturnValue: usize,
                   OriginalContext: *const CONTEXT,
                   HistoryTable: *const UNWIND_HISTORY_TABLE);
}

#[repr(C)]
struct PanicData {
    data: Box<Any + Send + 'static>
}

pub unsafe fn panic(data: Box<Any + Send + 'static>) -> ! {
    let panic_ctx = Box::new(PanicData { data: data });
    let params = [Box::into_raw(panic_ctx) as usize];
    rtdebug!("panic: ctx={:X}", params[0]);
    RaiseException(RUST_PANIC,
                   EXCEPTION_NONCONTINUABLE,
                   params.len() as DWORD,
                   &params as *const usize);
    rtabort!("could not unwind stack");
}

pub unsafe fn cleanup(ptr: *mut c_void) -> Box<Any + Send + 'static> {
    rtdebug!("cleanup: ctx={:X}", ptr as usize);
    let panic_ctx = Box::from_raw(ptr as *mut PanicData);
    return panic_ctx.data;
}

#[no_mangle] // referenced from rust_try.ll
pub unsafe extern "C" fn rust_eh_personality_catch(
    exceptionRecord: *mut EXCEPTION_RECORD,
    establisherFrame: usize,
    contextRecord: *mut CONTEXT,
    dispatcherContext: *mut DISPATCHER_CONTEXT
) -> EXCEPTION_DISPOSITION
{
    let er = &*exceptionRecord;
    let dc = &*dispatcherContext;
    rtdebug!("rust_eh_personality_catch: code={:X}, flags={:X}, frame={:X}, ip={:X}",
        er.ExceptionCode, er.ExceptionFlags, establisherFrame, dc.ControlPc);

    if er.ExceptionFlags & EXCEPTION_UNWIND == 0 { // we are in the dispatch phase
        if er.ExceptionCode == RUST_PANIC {
            if let Some(lpad) = find_landing_pad(dc) {
            	RtlUnwindEx(establisherFrame,
            				lpad,
            	            exceptionRecord,
            	            er.ExceptionInformation[0], // pointer to PanicData
            	            contextRecord,
            	            dc.HistoryTable);
                rtabort!("could not unwind");
            }
            rtabort!("could not locate the landing pad");
        }
    }

    ExceptionContinueSearch
}

#[lang="eh_personality"]
#[no_mangle] // referenced from rust_try.ll
#[allow(private_no_mangle_fns)]
pub unsafe extern "C" fn rust_eh_personality(
    exceptionRecord: *mut EXCEPTION_RECORD,
    establisherFrame: usize,
    contextRecord: *mut CONTEXT,
    dispatcherContext: *mut DISPATCHER_CONTEXT
) -> EXCEPTION_DISPOSITION
{
    let er = &*exceptionRecord;
    let dc = &*dispatcherContext;
    rtdebug!("rust_eh_personality: code={:X}, flags={:X}, frame={:X}, ip={:X}",
        er.ExceptionCode, er.ExceptionFlags, establisherFrame, dc.ControlPc);

    if er.ExceptionFlags & EXCEPTION_UNWIND == 0 {      // we are in the dispatch phase
        if er.ExceptionCode == RUST_PANIC {
            if let Some(lpad) = find_landing_pad(dc) {
                rtdebug!("unwinding to landing pad {:X}", lpad);

            	RtlUnwindEx(establisherFrame,
            				lpad,
            	            exceptionRecord,
            	            er.ExceptionInformation[0], // pointer to PanicData
            	            contextRecord,
            	            dc.HistoryTable);
                rtabort!("could not unwind");
            }
        }
    }
    // Note that we let non-Rust exceptions to pass through without running destructors!
    // This is considered acceptable because throwing exceptions through a C ABI boundary
    // is an undefined behavior.

    ExceptionContinueSearch
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_Resume(panic_ctx: usize) {
    rtdebug!("_Unwind_Resume: ctx={:X}", panic_ctx);
    let params = [panic_ctx];
    RaiseException(RUST_PANIC,
                   EXCEPTION_NONCONTINUABLE,
                   params.len() as DWORD,
                   &params as *const usize);
    rtabort!("could not resume unwind");
}

unsafe fn find_landing_pad(dc: &DISPATCHER_CONTEXT) -> Option<usize> {
    let eh_ctx = eh::EHContext {
        ip: dc.ControlPc,
        func_start: dc.ImageBase + (*dc.FunctionEntry).BeginAddress as usize,
        text_start: dc.ImageBase,
        data_start: 0
    };
    eh::find_landing_pad(dc.HandlerData, eh_ctx)
}
