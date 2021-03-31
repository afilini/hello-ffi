use syn::{parse_quote, Type};

macro_rules! our_opaque_types {
    () => {
        [
            Type::Verbatim(Default::default()), // dummy type to allow the compiler to understand what we are parsing

            // bitcoin_mod.rs
            parse_quote!(Script),
            parse_quote!(Network),
            parse_quote!(Address),
            parse_quote!(OutPoint),
            parse_quote!(TxOut),
            parse_quote!(TxIn),
            parse_quote!(Transaction),
        ]
    }
}

macro_rules! types_arr {
    ($( $ty:ident ),*) => {
        &[ Type::Verbatim(Default::default()), $( parse_quote!( $ty ) ),* ]
    }
}
