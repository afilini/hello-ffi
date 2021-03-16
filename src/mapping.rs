pub trait MapFrom<Source> {
    fn map_from(s: Source) -> Self;
}

impl<T> MapFrom<T> for T {
    #[inline]
    fn map_from(s: T) -> Self {
        s
    }
}

pub trait MapTo<Target> {
    fn map_to(self) -> Target;
}

impl<T> MapTo<T> for T {
    #[inline]
    fn map_to(self) -> T {
        self
    }
}

#[cfg(feature = "c")]
mod c_mapping {
    use super::{MapFrom, MapTo};

    impl MapFrom<*const libc::c_char> for String {
        fn map_from(s: *const libc::c_char) -> Self {
            unsafe {
                std::ffi::CStr::from_ptr(s)
                    .to_str()
                    .expect("Invalid incoming string")
                    .to_string()
            }
        }
    }

    impl<F: Clone, T: MapFrom<F>> MapFrom<(*const F, usize)> for Vec<T> {
        fn map_from((ptr, len): (*const F, usize)) -> Self {
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            slice.to_vec().into_iter().map(T::map_from).collect()
        }
    }

    impl MapTo<*mut libc::c_char> for String {
        fn map_to(self) -> *mut libc::c_char {
            let cstring = std::ffi::CString::new(self).expect("Invalid outgoing string");
            let ptr = cstring.as_ptr();
            std::mem::forget(cstring);

            ptr as *mut libc::c_char
        }
    }

    impl<F: Clone, T: MapTo<F>> MapTo<(*mut F, usize)> for Vec<T> {
        fn map_to(self) -> (*mut F, usize) {
            let mut mapped: Vec<F> = self.into_iter().map(T::map_to).collect();
            mapped.shrink_to_fit();

            let result = (mapped.as_mut_ptr(), mapped.len());
            std::mem::forget(mapped);

            result
        }
    }

    impl<T> MapTo<*mut T> for T {
        #[inline]
        fn map_to(self) -> *mut T {
            Box::into_raw(Box::new(self))
        }
    }

    impl<T> MapTo<*mut T> for Option<T> {
        #[inline]
        fn map_to(self) -> *mut T {
            self.map(MapTo::map_to)
                .unwrap_or_else(|| std::ptr::null_mut())
        }
    }
}
#[cfg(feature = "c")]
pub use c_mapping::*;
