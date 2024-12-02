use casper_sdk::host::{
    self,
    native::{Environment, DEFAULT_ADDRESS},
    Entity,
};

use crate::{
    contract::NFTContract,
    types::{
        BurnMode, MetadataMutability, MintingMode, NFTIdentifierMode, NFTKind, NFTMetadataKind,
        OwnershipMode, WhitelistMode,
    },
};

#[test]
fn should_transfer_token() {
    let stub = Environment::new(Default::default(), DEFAULT_ADDRESS);
    let result = host::native::dispatch_with(stub, || {
        let installer = host::get_caller();
        let recipient = Entity::Account([1; 32]);

        let mut contract = NFTContract::new(
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
            NFTMetadataKind::Raw,
            Vec::new(),
            Vec::new(),
            NFTIdentifierMode::Ordinal,
            MetadataMutability::Immutable,
            BurnMode::Burnable,
            false,
            None,
        );

        assert_eq!(contract.balance_of(installer).unwrap(), 0);
        assert_eq!(contract.balance_of(recipient).unwrap(), 0);

        let minted_token = contract
            .mint("Some token info!".into(), installer, None)
            .unwrap();

        assert_eq!(contract.balance_of(installer).unwrap(), 1);

        contract
            .transfer(installer, recipient, minted_token)
            .unwrap();

        assert_eq!(contract.balance_of(installer).unwrap(), 0);
        assert_eq!(contract.balance_of(recipient).unwrap(), 1);
    });
    assert!(result.is_ok());
}
