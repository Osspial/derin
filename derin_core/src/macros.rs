macro_rules! id {
    ($vis:vis $Name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        $vis struct $Name(u32);

        impl $Name {
            #[inline]
            pub fn new() -> $Name {
                use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

                static ID_COUNTER: AtomicUsize = ATOMIC_USIZE_INIT;
                let id = ID_COUNTER.fetch_add(1, Ordering::SeqCst) as u32;

                $Name(id as u32)
            }

            pub fn to_u32(self) -> u32 {
                self.0
            }

            #[allow(dead_code)]
            pub(crate) fn dummy() -> $Name {
                $Name(!0)
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
