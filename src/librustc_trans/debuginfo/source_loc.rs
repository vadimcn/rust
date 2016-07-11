// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use self::InternalDebugLocation::*;

use super::utils::debug_context;
use super::metadata::{scope_metadata,UNKNOWN_COLUMN_NUMBER};
use super::{FunctionDebugContext, DebugLoc};

use llvm;
use llvm::debuginfo::DIScope;
use builder::Builder;
use common::{NodeIdAndSpan, CrateContext, FunctionContext};

use libc::c_uint;
use std::ptr;
use syntax_pos::{self, Span, BytePos};
use syntax::ast;

pub fn get_cleanup_debug_loc_for_ast_node<'a, 'tcx>(cx: &CrateContext<'a, 'tcx>,
                                                    node_id: ast::NodeId,
                                                    node_span: Span,
                                                    is_block: bool)
                                                 -> NodeIdAndSpan {
    // A debug location needs two things:
    // (1) A span (of which only the beginning will actually be used)
    // (2) An AST node-id which will be used to look up the lexical scope
    //     for the location in the functions scope-map
    //
    // This function will calculate the debug location for compiler-generated
    // cleanup calls that are executed when control-flow leaves the
    // scope identified by `node_id`.
    //
    // For everything but block-like things we can simply take id and span of
    // the given expression, meaning that from a debugger's view cleanup code is
    // executed at the same source location as the statement/expr itself.
    //
    // Blocks are a special case. Here we want the cleanup to be linked to the
    // closing curly brace of the block. The *scope* the cleanup is executed in
    // is up to debate: It could either still be *within* the block being
    // cleaned up, meaning that locals from the block are still visible in the
    // debugger.
    // Or it could be in the scope that the block is contained in, so any locals
    // from within the block are already considered out-of-scope and thus not
    // accessible in the debugger anymore.
    //
    // The current implementation opts for the second option: cleanup of a block
    // already happens in the parent scope of the block. The main reason for
    // this decision is that scoping becomes controlflow dependent when variable
    // shadowing is involved and it's impossible to decide statically which
    // scope is actually left when the cleanup code is executed.
    // In practice it shouldn't make much of a difference.

    let mut cleanup_span = node_span;

    if is_block {
        // Not all blocks actually have curly braces (e.g. simple closure
        // bodies), in which case we also just want to return the span of the
        // whole expression.
        let code_snippet = cx.sess().codemap().span_to_snippet(node_span);
        if let Ok(code_snippet) = code_snippet {
            let bytes = code_snippet.as_bytes();

            if !bytes.is_empty() && &bytes[bytes.len()-1..] == b"}" {
                cleanup_span = Span {
                    lo: node_span.hi - syntax_pos::BytePos(1),
                    hi: node_span.hi,
                    expn_id: node_span.expn_id
                };
            }
        }
    }

    NodeIdAndSpan {
        id: node_id,
        span: cleanup_span
    }
}


/// Sets the current debug location at the beginning of the span.
///
/// Maps to a call to llvm::LLVMSetCurrentDebugLocation(...).
pub fn set_source_location(fcx: &FunctionContext,
                           builder: Option<&Builder>,
                           debug_loc: DebugLoc) {
    let builder = builder.map(|b| b.llbuilder);
    let function_debug_context = match fcx.debug_context {
        FunctionDebugContext::DebugInfoDisabled => return,
        FunctionDebugContext::FunctionWithoutDebugInfo => {
            set_debug_location(fcx.ccx, builder, UnknownLocation);
            return;
        }
        FunctionDebugContext::RegularContext(box ref data) => data
    };

    if function_debug_context.source_location_override.get() {
        // Just ignore any attempts to set a new debug location while
        // the override is active.
        return;
    }

    let dbg_loc = if function_debug_context.source_locations_enabled.get() {
        match debug_loc {
            DebugLoc::At(node_id, span) => {
                let scope = scope_metadata(fcx, node_id, span);
                KnownLocation(scope, span.lo, None)
            }
            DebugLoc::ScopeAt(scope, span, inlined_at) => {
                let inlined_at_loc = match inlined_at {
                    Some((scope, span)) => Some((scope, span.lo)),
                    None => None
                };
                KnownLocation(scope, span.lo, inlined_at_loc)
            }
            DebugLoc::None => UnknownLocation
        }
    } else {
        UnknownLocation
    };
    set_debug_location(fcx.ccx, builder, dbg_loc);
}

/// This function makes sure that all debug locations emitted while executing
/// `wrapped_function` are set to the given `debug_loc`.
pub fn with_source_location_override<F, R>(fcx: &FunctionContext,
                                           debug_loc: DebugLoc,
                                           wrapped_function: F) -> R
    where F: FnOnce() -> R
{
    match fcx.debug_context {
        FunctionDebugContext::DebugInfoDisabled => {
            wrapped_function()
        }
        FunctionDebugContext::FunctionWithoutDebugInfo => {
            set_debug_location(fcx.ccx, None, UnknownLocation);
            wrapped_function()
        }
        FunctionDebugContext::RegularContext(box ref function_debug_context) => {
            if function_debug_context.source_location_override.get() {
                wrapped_function()
            } else {
                debug_loc.apply(fcx);
                function_debug_context.source_location_override.set(true);
                let result = wrapped_function();
                function_debug_context.source_location_override.set(false);
                result
            }
        }
    }
}

/// Enables emitting source locations for the given functions.
///
/// Since we don't want source locations to be emitted for the function prelude,
/// they are disabled when beginning to translate a new function. This functions
/// switches source location emitting on and must therefore be called before the
/// first real statement/expression of the function is translated.
pub fn start_emitting_source_locations(fcx: &FunctionContext) {
    match fcx.debug_context {
        FunctionDebugContext::RegularContext(box ref data) => {
            data.source_locations_enabled.set(true)
        },
        _ => { /* safe to ignore */ }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum InternalDebugLocation {
    KnownLocation(DIScope, BytePos, Option<(DIScope, BytePos)>),
    UnknownLocation
}

pub fn set_debug_location(cx: &CrateContext,
                          builder: Option<llvm::BuilderRef>,
                          debug_location: InternalDebugLocation) {
    if builder.is_none() {
        if debug_location == debug_context(cx).current_debug_location.get() {
            return;
        }
    }

    let cm = cx.sess().codemap();
    let metadata_node = match debug_location {
        KnownLocation(scope, pos, inlined_at) => {

            let inlined_at_loc = match inlined_at {
                Some((scope, pos)) => {
                    let loc = cm.lookup_char_pos(pos);
                    unsafe {
                        llvm::LLVMValueAsMetadata(
                            llvm::LLVMDIBuilderCreateDebugLocation(
                                debug_context(cx).llcontext,
                                loc.line as c_uint,
                                UNKNOWN_COLUMN_NUMBER as c_uint,
                                scope,
                                ptr::null_mut())
                        )
                    }
                },
                None => ptr::null_mut()
            };

            let loc = cm.lookup_char_pos(pos);
            debug!("setting debug location to line {}", loc.line);
            // Set the column to zero like Clang and GCC
            unsafe {
                llvm::LLVMDIBuilderCreateDebugLocation(
                    debug_context(cx).llcontext,
                    loc.line as c_uint,
                    UNKNOWN_COLUMN_NUMBER as c_uint,
                    scope,
                    inlined_at_loc)
            }
        }
        UnknownLocation => {
            debug!("clearing debug location ");
            ptr::null_mut()
        }
    };

    if builder.is_none() {
        debug_context(cx).current_debug_location.set(debug_location);
    }

    let builder = builder.unwrap_or_else(|| cx.raw_builder());
    unsafe {
        llvm::LLVMSetCurrentDebugLocation(builder, metadata_node);
    }
}
