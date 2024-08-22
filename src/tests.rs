use casper_sdk::{host::{self, native::{Environment, DEFAULT_ADDRESS}, Entity}, types::Address};

use crate::{contract::NFTContract, types::{BurnMode, MetadataMutability, MintingMode, NFTIdentifierMode, NFTKind, NFTMetadataKind, OwnershipMode, WhitelistMode}};

#[test]
fn it_works() {
    let stub = Environment::new(Default::default(), DEFAULT_ADDRESS);

    let result = host::native::dispatch_with(stub, || {
        NFTContract::new(
            "test-collection".into(),
            "tc".into(),
            100,
            true,
            MintingMode::Installer,
            OwnershipMode::Transferable,
            NFTKind::Virtual,
            WhitelistMode::Unlocked,
            Vec::new(),
            false,
            false,
            "".into(),
            NFTMetadataKind::CEP78,
            Vec::new(),
            Vec::new(),
            NFTIdentifierMode::Ordinal,
            MetadataMutability::Immutable,
            BurnMode::Burnable,
            false,
            None
        );
    });
    assert_eq!(result, Ok(()));
}

#[test]
fn caller_test() {
    let stub = Environment::new(Default::default(), DEFAULT_ADDRESS);
    let result = host::native::dispatch_with(stub, || {
        let caller = host::get_caller();
        assert_eq!(caller, Entity::Account(DEFAULT_ADDRESS));
    });
    assert_eq!(result, Ok(()));
}