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
