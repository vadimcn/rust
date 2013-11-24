// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Coroutine result

/// A type that represents the return type of a coroutine
#[lang="coresult"]
pub enum CoResult<Y,R> {
	/// Coroutine executed "yield"
	Yield(Y),
	/// Execution reached end of the coroutine
	Return(R)
}
