macro_rules! impl_window_traits {
    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
        for $window:ty
    ) => ();

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            BaseWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => (
        unsafe impl<$($lt,)* W: $($window_bound +)* $(, $gen: $gen_bound)*> BaseWindow for $window {
            #[inline]
            fn hwnd(&self) -> HWND {
                self.inner().hwnd()
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    );

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            IconWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: IconWindow $(+ $window_bound)* $(, $gen: $gen_bound)*> IconWindow for $window {
            type IconSm = W::IconSm;
            type IconLg = W::IconLg;

            #[inline]
            fn set_icon_sm(&mut self, icon: Option<W::IconSm>) -> Option<W::IconSm> {
                self.inner_mut().set_icon_sm(icon)
            }
            #[inline]
            fn set_icon_lg(&mut self, icon: Option<W::IconLg>) -> Option<W::IconLg> {
                self.inner_mut().set_icon_lg(icon)
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    };

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            FontWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: FontWindow $(+ $window_bound)* $(, $gen: $gen_bound)*> FontWindow for $window {
            type Font = W::Font;
            fn set_font(&mut self, font: W::Font) {
                self.inner_mut().set_font(font)
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    };

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            StaticBitmapWindow
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: StaticBitmapWindow $(+ $window_bound)* $(, $gen: $gen_bound)*> StaticBitmapWindow for $window {
            type Bitmap = W::Bitmap;
            #[inline]
            fn set_bitmap(&mut self, bitmap: W::Bitmap) -> W::Bitmap {
                self.inner_mut().set_bitmap(bitmap)
            }

            #[inline]
            fn bitmap_ref(&self) -> &W::Bitmap {
                self.inner().bitmap_ref()
            }

            #[inline]
            unsafe fn bitmap_mut(&mut self) -> &mut W::Bitmap {
                self.inner_mut().bitmap_mut()
            }
        }
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    };

    (
        unsafe impl<$(lifetime $lt:tt,)* W$(: $window_bound:path)* $(, $gen:ident: $gen_bound:path)*>
            $trait_name:ident
            $(, $trait_rest:ident)*
        for $window:ty
    ) => {
        unsafe impl<$($lt,)* W: $trait_name $(+ $window_bound)* $(, $gen: $gen_bound)*> $trait_name for $window {}
        impl_window_traits!{
            unsafe impl<$(lifetime $lt,)* W$(: $window_bound)* $(, $gen: $gen_bound)*>
                $($trait_rest),*
            for $window
        }
    };
}
