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
    use crate::langs::*;

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

    impl<F: Clone, T: MapFrom<F>> MapFrom<Arr<F>> for Vec<T> {
        fn map_from(arr: Arr<F>) -> Self {
            let Arr { ptr, len } = arr;
            Self::map_from((ptr, len))
        }
    }

    impl MapFrom<*const u8> for [u8; 32] {
        fn map_from(ptr: *const u8) -> Self {
            use std::convert::TryInto;

            let slice = unsafe { std::slice::from_raw_parts(ptr, 32) };
            slice.try_into().unwrap()
        }
    }

    impl<T> MapFrom<T> for *mut T {
        fn map_from(t: T) -> Self {
            Box::into_raw(Box::new(t))
        }
    }

    impl<T> MapFrom<T> for Box<T> {
        fn map_from(t: T) -> Self {
            Box::new(t)
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

    impl MapTo<*const u8> for &[u8] {
        fn map_to(self) -> *const u8 {
            self.as_ptr()
        }
    }
}
#[cfg(feature = "c")]
pub use c_mapping::*;

#[cfg(feature = "python")]
mod python_mapping {
    use pyo3::prelude::*;
    use pyo3::pyclass::PyClass;
    use pyo3::pyclass_init::PyClassInitializer;
    use pyo3::type_object::{PyBorrowFlagLayout, PyTypeInfo};

    use super::*;

    impl<T> MapFrom<T> for pyo3::Py<T>
    where
        T: pyo3::PyTypeInfo + Into<PyClassInitializer<T>> + PyClass,
        <T as PyTypeInfo>::BaseLayout: PyBorrowFlagLayout<<T as PyTypeInfo>::BaseType>,
    {
        fn map_from(owned: T) -> Self {
            pyo3::prelude::Python::with_gil(move |py| -> pyo3::PyResult<pyo3::Py<T>> {
                Ok(pyo3::Py::new(py, owned)?)
            })
            .expect("Unable to allocate cell")
        }
    }
}
#[cfg(feature = "python")]
pub use python_mapping::*;
