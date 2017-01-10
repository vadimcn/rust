// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![unstable(feature = "coroutines", issue="0")]

//! Coroutine result

/// A type that represents the return type of a coroutine
#[cfg(not(stage0))]
#[unstable(feature = "coroutines", issue="0")]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
//#[cfg_attr(not(stage0), lang="coresult")]
pub enum CoResult<Y,R> {
  /// Coroutine executed "yield"
  Yield(Y),
  /// Execution has reached the end of the coroutine
  Return(R)
}

