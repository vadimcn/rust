# Copyright 2015 The Rust Project Developers. See the COPYRIGHT
# file at the top-level directory of this distribution and at
# http://rust-lang.org/COPYRIGHT.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

import sys

output = sys.argv[1]
name = sys.argv[2]

with open('src/librustc_llvm/lib.rs','r') as f:
    with open(output,'w') as g:
        print >> g, 'LIBRARY ' + name
        print >> g, 'EXPORTS'
        for x in f:
            x = str(x)
            if not x.startswith('    pub fn LLVM'): continue
            name = x[11:x.find('(')]
            print >> g, '  ' + name
