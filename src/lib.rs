use derive::expose_mod;

pub mod mapping;
#[macro_use]
pub mod langs;

#[cfg(not(any(feature = "c", feature = "python")))]
compile_error!("No language enabled");

#[cfg(all(feature = "c", any(feature = "python")))]
compile_error!("Enable at most one language");
#[cfg(all(feature = "python", any(feature = "c")))]
compile_error!("Enable at most one language");

#[derive(Debug)]
pub enum BitcoinError {
    Bitcoin(bdk::bitcoin::Error),
    Hex(bdk::bitcoin::hashes::hex::Error),
    Address(bdk::bitcoin::util::address::Error),
    IO(std::io::Error),
}
impl From<bdk::bitcoin::Error> for BitcoinError {
    fn from(e: bdk::bitcoin::Error) -> Self {
        BitcoinError::Bitcoin(e)
    }
}
impl From<bdk::bitcoin::hashes::hex::Error> for BitcoinError {
    fn from(e: bdk::bitcoin::hashes::hex::Error) -> Self {
        BitcoinError::Hex(e)
    }
}
impl From<bdk::bitcoin::util::address::Error> for BitcoinError {
    fn from(e: bdk::bitcoin::util::address::Error) -> Self {
        BitcoinError::Address(e)
    }
}
impl From<std::io::Error> for BitcoinError {
    fn from(e: std::io::Error) -> Self {
        BitcoinError::IO(e)
    }
}
#[cfg(feature = "python")]
impl_py_error!(BitcoinError);
#[cfg(feature = "c")]
impl langs::IntoPlatformError for BitcoinError {
    type TargetType = i32;

    fn into_platform_error(self) -> Self::TargetType {
        // match self {
        // }
        -1
    }

    fn ok() -> Self::TargetType {
        0
    }
}

#[expose_mod]
mod bitcoin {
    use bdk::bitcoin as bitcoin_orig;

    use super::BitcoinError;

    #[expose_struct("opaque")]
    pub struct Script {
        script: bitcoin_orig::Script,
    }
    impl From<bitcoin_orig::Script> for Script {
        fn from(script: bitcoin_orig::Script) -> Self {
            Script { script }
        }
    }
    impl Into<bitcoin_orig::Script> for Script {
        fn into(self) -> bitcoin_orig::Script {
            self.script
        }
    }
    #[expose_impl]
    impl Script {
        #[constructor]
        fn from_hex(hex: String) -> Result<Self, BitcoinError> {
            use bitcoin_orig::hashes::hex::FromHex;

            Ok(bitcoin_orig::Script::from_hex(&hex)?.into())
        }
        #[destructor]
        fn destroy(_s: Self) {}

        fn to_hex(&self) -> String {
            use bitcoin_orig::hashes::hex::ToHex;

            self.script.to_hex()
        }

        fn asm(&self) -> String {
            self.script.asm()
        }
    }

    #[expose_struct("opaque")]
    pub struct Network {
        network: bitcoin_orig::Network,
    }
    impl From<bitcoin_orig::Network> for Network {
        fn from(network: bitcoin_orig::Network) -> Self {
            Network { network }
        }
    }
    impl Into<bitcoin_orig::Network> for Network {
        fn into(self) -> bitcoin_orig::Network {
            self.network
        }
    }
    #[expose_impl]
    impl Network {
        #[constructor]
        fn from_string(s: String) -> Result<Self, BitcoinError> {
            use std::str::FromStr;

            Ok(bitcoin_orig::Network::from_str(&s)?.into())
        }
        #[destructor]
        fn destroy(_s: Self) {}

        fn bitcoin() -> Self {
            bitcoin_orig::Network::Bitcoin.into()
        }
        fn testnet() -> Self {
            bitcoin_orig::Network::Testnet.into()
        }

        fn to_string(&self) -> String {
            self.network.to_string()
        }
    }

    #[expose_struct("opaque")]
    pub struct Address {
        address: bitcoin_orig::Address,
    }
    impl From<bitcoin_orig::Address> for Address {
        fn from(address: bitcoin_orig::Address) -> Self {
            Address { address }
        }
    }
    impl Into<bitcoin_orig::Address> for Address {
        fn into(self) -> bitcoin_orig::Address {
            self.address
        }
    }
    #[expose_impl]
    impl Address {
        fn from_script(script: &Script, network: &Network) -> Option<Self> {
            bitcoin_orig::Address::from_script(&script.script, network.clone().network)
                .map(|address| Address { address })
        }

        #[constructor]
        fn from_string(s: String) -> Result<Self, BitcoinError> {
            use std::str::FromStr;

            Ok(bitcoin_orig::Address::from_str(&s)?.into())
        }
        #[destructor]
        fn destroy(_s: Self) {}

        fn to_string(&self) -> String {
            self.address.to_string()
        }

        #[getter]
        fn get_script(&self) -> Script {
            self.address.script_pubkey().into()
        }

        fn network(&self) -> Network {
            self.address.network.into()
        }
    }
}
