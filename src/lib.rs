use std::ffi::{CStr, CString};

use libc::{c_char, c_void};

use derive::{Expose, expose};

macro_rules! str_out {
    ($s:expr) => {
        CString::new($s).expect("Invalid outgoing string").as_ptr()
    }
}

macro_rules! str_in {
    ($s:expr) => {
        unsafe {
            CStr::from_ptr($s).to_str().expect("Invalid incoming string")
        }
    }
}

macro_rules! set_ptr_out {
    ($ptr_out:expr, $result:expr) => {
        assert!(($ptr_out as usize) > 0);
        unsafe {
            *$ptr_out = Box::into_raw(Box::new($result));
        }
    };
}

pub struct HelloStruct(hello::HelloStruct);

impl HelloStruct {
    #[no_mangle]
    pub extern "C" fn hello_static() -> *const c_char {
        str_out!(hello::HelloStruct::hello_static())
    }

    #[no_mangle]
    pub extern "C" fn hello_method(&self, string: *const c_char) -> *const c_char {
        str_out!(self.0.hello_method(str_in!(string)))
    }
}

pub struct Wallet(hello::Wallet);

#[repr(C)]
#[derive(Debug)]
pub struct CoinSelection {
    this: *const c_void,
    fn_do_something: unsafe extern "C" fn(*const c_void, u32) -> u32,

    destroy: unsafe extern "C" fn(*const c_void),
}

pub trait IntoAnyCs {
    fn into_any_cs(self) -> CoinSelection;
}

impl<C: hello::CoinSelectionAlgorithm> IntoAnyCs for C {
    fn into_any_cs(self) -> CoinSelection {
        unsafe extern "C" fn fn_do_something<C: hello::CoinSelectionAlgorithm>(this: *const c_void, val: u32) -> u32 {
            assert!((this as usize) > 0);
            let this = Box::from_raw(this as *mut C);
            let result = this.do_something(val);
            std::mem::forget(this);

            result
        }

        unsafe extern "C" fn destroy<C: hello::CoinSelectionAlgorithm>(this: *const c_void) {
            // Do nothing, let `Box` free our memory when it's dropped
            assert!((this as usize) > 0);
            let _ = Box::from_raw(this as *mut C);
        }

        let this = Box::into_raw(Box::new(self)) as *const c_void;

        CoinSelection {
            this,
            fn_do_something: fn_do_something::<C>,
            destroy: destroy::<C>,
        }
    }
}

impl hello::CoinSelectionAlgorithm for CoinSelection {
    fn do_something(&self, val: u32) -> u32 {
        assert!((self.fn_do_something as usize) > 0);

        unsafe {
            (self.fn_do_something)(self.this, val)
        }
    }
}


#[no_mangle]
pub extern "C" fn triple_cs_new(init: u32) -> CoinSelection {
    hello::TripleCS::new(init).into_any_cs()
}

// impl TripleCS {
//     #[no_mangle]
//     pub extern "C" fn triple_cs_new(init: u32, ptr_out: *mut *mut Self) {
//         set_ptr_out!(ptr_out, TripleCS(hello::TripleCS::new(init)));
//     }
// }

// impl AnyCs {
//     #[no_mangle]
//     pub extern "C" fn new(this: *const c_void, fn_do_something: unsafe extern "C" fn(*const c_void, u32) -> u32, destroy: unsafe extern "C" fn(*const c_void)) -> InnerAnyCs {
//         InnerAnyCs { this, fn_do_something, destroy }
//     }
// }

impl std::ops::Drop for CoinSelection {
    fn drop(&mut self) {
        if (self.destroy as usize) > 0 {
            println!("Dropping {:?}", self.this);

            unsafe { (self.destroy)(self.this); }
        }
    }
}

impl Wallet {
    #[no_mangle]
    pub extern "C" fn wallet_new(wallet_name: *const c_char, ptr_out: *mut *mut Self) {
        set_ptr_out!(ptr_out, Wallet(hello::Wallet::new(str_in!(wallet_name))));
    }

    #[no_mangle]
    pub extern "C" fn create_tx<'w>(&'w self, ptr_out: *mut *mut TxBuilder<'w>) {
        set_ptr_out!(ptr_out, TxBuilder(self.0.create_tx().convert_internal_cs(|cs| cs.into_any_cs())));
    }
}

pub struct TxBuilder<'w>(hello::TxBuilder<'w, CoinSelection>);

#[no_mangle]
pub extern "C" fn enable_flag<'w>(this: &mut TxBuilder<'w>) {
    this.0.enable_flag();
}
#[no_mangle]
pub extern "C" fn disable_flag<'w>(this: &mut TxBuilder<'w>) {
    this.0.disable_flag();
}
#[no_mangle]
pub extern "C" fn coin_selection<'w>(this: &mut TxBuilder<'w>, new_cs: CoinSelection) {
    *this.0.mut_cs() = new_cs;
}
#[no_mangle]
pub unsafe extern "C" fn finish<'w>(this: *mut TxBuilder<'w>) -> u32 {
    let this: TxBuilder<'w> = *Box::from_raw(this);
    this.0.finish()
}

fn main() {
}
