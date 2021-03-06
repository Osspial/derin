// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

macro_rules! id {
    ($vis:vis $Name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis struct $Name(std::num::NonZeroU32);

        impl $Name {
            #[inline]
            pub fn new() -> $Name {
                use std::sync::atomic::{AtomicUsize, Ordering};

                static ID_COUNTER: AtomicUsize = AtomicUsize::new(1);
                let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;

                $Name(std::num::NonZeroU32::new(id as u32).unwrap())
            }

            pub fn to_u32(self) -> u32 {
                self.0.get()
            }
        }
    }
}

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
