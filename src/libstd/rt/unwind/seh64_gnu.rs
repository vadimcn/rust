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
const RUST_UNWIND: DWORD = ETYPE | (2 << 24) | MAGIC;

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
#[derive(Copy, Clone)]
#[allow(raw_pointer_derive)]
pub struct EXCEPTION_RECORD {
    ExceptionCode: DWORD,
    ExceptionFlags: DWORD,
    ExceptionRecord: *const EXCEPTION_RECORD,
    ExceptionAddress: usize,
    NumberParameters: DWORD,
    ExceptionInformation: [usize; 15],
}

#[repr(C)]
pub struct CONTEXT {
    _align: [simd::u64x2; 0], // FIXME align on 16-byte
    _data: [u8; 1232] // sizeof(CONTEXT) == 1232
}

#[repr(C)]
pub struct UNWIND_HISTORY_TABLE {
    _align: [simd::u64x2; 0], // FIXME align on 16-byte
    _data: [u8; 216] // sizeof(UNWIND_HISTORY_TABLE) == 216
}

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

    fn RtlCaptureContext(context: *mut CONTEXT);
}

// How does this work?
// --------------------
// SEH exception handling, just like DWARF exception handling, happens in two phases:
// 1. Dispatch
// Somebody somewhere calls RaiseException(), which in turn calls DispatchException(), which starts going
// down the stack invoking exception handlers (aka language personality functions) for each frame.
// rust_eh_personality() does not participate in this phase and just returns ExceptionContinueSearch
// (after all, some handler down the stack might fix the error condition and resume execution).
// rust_eh_personality_catch(), on the other hand, always initiates stack unwind by calling RtlUnwindEx()
// targetting its landing pad.
//
// 2. Unwinding
// RtlUnwindEx(), goes down the stack and invokes handlers again, this time setting EXCEPTION_UNWINDING
// flag in EXCEPTION_RECORD.ExceptionFlags - to indicate that unwind is in progress.
// This time around, rust_eh_personality() would want to call into the landing pads to execute object
// destructors, etc. However, it cannot call them directly, because DWARF landing pads expect the original
// function's stack frame to be active, which is not the case here.
// (Side note: in MSVC world, code that is the equivalent of landing pads, is outlined into separate functions
// that know how to reach into the stack frame of the "parent" function, so it's OK for the handler to invoke
// them directly).
// The trick to making this work is as follows:
// - rust_eh_personality() saves the parameters of the current unwind, then calls RtlUnwindEx() targetting
//   the landing pad it needs to execute.  This results in cancelling the original unwind, and instead unwinding
//   to the frame of the function owning the landing pad.
// - The control is passed to the landing pad, at the end of which _Unwind_Resume() is called.
// - _Unwind_Resume(), in turn, calls RtlUnwindEx() yet again, restoring the original unwind target.

pub unsafe fn panic(data: Box<Any + Send + 'static>) -> ! {
    //let data = Box::into_raw(data);
    //let params = [data as usize];
    RaiseException(RUST_PANIC,
                   EXCEPTION_NONCONTINUABLE,
                   0,
                   0 as *const usize);
                   //params.len() as DWORD,
                   //&params as *const usize);
    rtabort!("could not unwind stack");
}

pub unsafe fn cleanup(_ptr: *mut c_void) -> Box<Any + Send + 'static> {
    rtabort!("not implemented");
}

#[no_mangle] // referenced from rust_try.ll
//#[cfg(not(test))]
pub unsafe extern "C" fn rust_eh_personality_catch(
    exceptionRecord: *mut EXCEPTION_RECORD,
    establisherFrame: usize,
    _contextRecord: *mut CONTEXT,
    dispatcherContext: *mut DISPATCHER_CONTEXT
) -> EXCEPTION_DISPOSITION
{
    let er = &*exceptionRecord;
    let dc = &*dispatcherContext;

    println!("rust_eh_personality_catch: code={:X}, flags={:X}, frame={:X}",
        er.ExceptionCode, er.ExceptionFlags, establisherFrame);

    if er.ExceptionFlags & EXCEPTION_UNWIND == 0 { // we are in the dispatch phase
        if let Some(lpad) = find_landing_pad(dc) {
        	RtlUnwindEx(establisherFrame,
        				lpad,
        	            exceptionRecord,
        	            0,
        	            dc.ContextRecord,
        	            dc.HistoryTable);
            rtabort!("could not unwind");
        }
        rtabort!("could not locate landing pad");
    }

    ExceptionContinueSearch
}

#[lang="eh_personality"]
#[no_mangle] // referenced from rust_try.ll
#[allow(private_no_mangle_fns)]
//#[cfg(not(test))]
pub unsafe extern "C" fn rust_eh_personality(
    exceptionRecord: *mut EXCEPTION_RECORD,
    establisherFrame: usize,
    contextRecord: *mut CONTEXT,
    dispatcherContext: *mut DISPATCHER_CONTEXT
) -> EXCEPTION_DISPOSITION
{
    let er = &*exceptionRecord;
    let dc = &*dispatcherContext;

    println!("rust_eh_personality: code={:X}, flags={:X}, frame={:X}, PC={:X}",
        er.ExceptionCode, er.ExceptionFlags, establisherFrame, dc.ControlPc);

    if er.ExceptionFlags & EXCEPTION_UNWIND == 0 { // we are in the dispatch phase
        if let Some(lpad) = find_landing_pad(dc) { // ...and we have a landing pad
            // ...then we need to suspend the current unwind and target our landing pad instead
            println!("Found landing pad {:X}", lpad);

            let rd = Box::new(ResumeData {
                origException: *exceptionRecord,
            });

        	RtlUnwindEx(establisherFrame,
        				lpad,
        	            exceptionRecord,
        	            Box::into_raw(rd) as usize,
        	            dc.ContextRecord,
        	            dc.HistoryTable);
            rtabort!("could not unwind");
        }
    }

    ExceptionContinueSearch
}

#[repr(C)]
pub struct ResumeData {
    origException: EXCEPTION_RECORD,
}

#[no_mangle]
pub unsafe extern "C" fn _Unwind_Resume(resumeData: *const ResumeData) {
    println!("_Unwind_Resume");
    let rd = &*resumeData;
    RaiseException(rd.origException.ExceptionCode,
                   rd.origException.ExceptionFlags,
                   rd.origException.NumberParameters,
                   &rd.origException.ExceptionInformation as *const usize);
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
