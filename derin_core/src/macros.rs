// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#[allow(unused_macros)]
macro_rules! bench {
    ($($tt:tt)*) => {{
        let start_time = std::time::Instant::now();
        let r = {$($tt)*};
        static mut A: (std::time::Duration, u32) = (std::time::Duration::from_secs(0), 0);
        let elapsed = std::time::Instant::now() - start_time;
        unsafe {
            A.0 += elapsed;
            A.1 += 1;
            println!("bench {}:{} avg={:?}\tthis={:?}", file!(), line!(), A.0 / A.1, elapsed);
        }
        r
    }}
}
