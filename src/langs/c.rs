use crate::mapping::MapFrom;

pub struct Destroy<T>(*mut T);

impl<T> std::ops::Drop for Destroy<T> {
    fn drop(&mut self) {
        let _inner = unsafe { Box::from_raw(self.0) };
    }
}

impl<T> MapFrom<*mut T> for Destroy<T> {
    #[inline]
    fn map_from(ptr: *mut T) -> Self {
        Destroy(ptr)
    }
}

pub trait IntoPlatformError {
    type TargetType: std::fmt::Debug;

    fn into_platform_error(self) -> Self::TargetType;

    fn ok() -> Self::TargetType;
}

#[derive(Debug)]
pub struct PlatformOption;

impl IntoPlatformError for PlatformOption {
    type TargetType = ();

    fn into_platform_error(self) {}

    fn ok() {}
}

pub trait IntoTraitStruct: Sized {
    type Target;

    fn into_trait_struct(self) -> Self::Target;
}

#[inline]
pub fn take_ptr<I>(this: *mut libc::c_void) -> Box<I> {
    unsafe { Box::from_raw(this as *mut I) }
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Arr<T> {
    pub ptr: *const T,
    pub len: usize,
}
