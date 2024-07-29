use casper_macros::*;
use casper_sdk::*;
use host::Entity;
use types::*;
use crate::{error::NFTCoreError, events::{events_cep47::CEP47Event, events_ces::{Approval, ApprovalForAll, ApprovalRevoked, Burn, Event, Mint, RevokedForAll, Transfer, VariablesSet}}, types::*};

#[casper(contract_state)]
pub struct NFTContract {
    state: CEP78State,
}

impl Default for NFTContract {
    fn default() -> Self {
        let state = CEP78State {
            collection_name: "test-collection".into(),
            collection_symbol: "tc".into(),
            total_token_supply: 100,
            allow_minting: true,
            minting_mode: MintingMode::Installer,
            ownership_mode: OwnershipMode::Transferable,
            nft_kind: NFTKind::Virtual,
            whitelist_mode: WhitelistMode::Unlocked,
            acl_whitelist: Vec::new(),
            acl_package_mode: false,
            package_operator_mode: false,
            package_hash: "".into(),
            base_metadata_kind: NFTMetadataKind::CEP78,
            optional_metadata: Vec::new(),
            additional_required_metadata: Vec::new(),
            identifier_mode: NFTIdentifierMode::Ordinal,
            metadata_mutability: MetadataMutability::Immutable,
            burn_mode: BurnMode::Burnable,
            operator_burn_mode: false,
            installer: Entity::Account([0;32]),
            events_mode: EventsMode::NoEvents,
            minted_tokens_count: 0,
            owned_tokens_count: 0
        };

        Self { state }
    }
}

#[casper(contract)]
impl NFTContract {
    #[casper(constructor)]
    pub fn new(
        collection_name: String,
        collection_symbol: String,
        total_token_supply: u64,
        allow_minting: bool,
        minting_mode: MintingMode,
        ownership_mode: OwnershipMode,
        nft_kind: NFTKind,
        whitelist_mode: WhitelistMode,
        acl_whitelist: Vec<Address>,
        acl_package_mode: bool,
        package_operator_mode: bool,
        package_hash: String,
        base_metadata_kind: NFTMetadataKind,
        optional_metadata: Vec<u8>,
        additional_required_metadata: Vec<u8>,
        identifier_mode: NFTIdentifierMode,
        metadata_mutability: MetadataMutability,
        burn_mode: BurnMode,
        operator_burn_mode: bool,
        events_mode: Option<EventsMode>
    ) -> NFTContract {
        let installer = host::get_caller();
        let events_mode = events_mode.unwrap_or(EventsMode::NoEvents);
        let minted_tokens_count = 0u64;
        let owned_tokens_count = 0u64;

        let state = CEP78State {
            collection_name,
            collection_symbol,
            total_token_supply,
            allow_minting,
            minting_mode,
            ownership_mode,
            nft_kind,
            whitelist_mode,
            acl_whitelist,
            acl_package_mode,
            package_operator_mode,
            package_hash,
            base_metadata_kind,
            optional_metadata,
            additional_required_metadata,
            identifier_mode,
            metadata_mutability,
            burn_mode,
            operator_burn_mode,
            minted_tokens_count,
            owned_tokens_count,
            events_mode,
            installer
        };

        Self {
            state
        }
    }

    pub fn set_variables(
        &mut self,
        allow_minting: Option<bool>,
        acl_package_mode: Option<bool>,
        package_operator_mode: Option<bool>,
        operator_burn_mode: Option<bool>,
        acl_whitelist: Option<Vec<Address>>,
        contract_whitelist: Option<Vec<Address>>
    ) -> Result<(), NFTCoreError>{
        // Only the installing account can change the mutable variables.
        if self.state.installer != host::get_caller() {
            return Err(NFTCoreError::InvalidAccount);
        }

        if let Some(allow_minting) = allow_minting {
            self.state.allow_minting = allow_minting;
        }

        if let Some(acl_package_mode) = acl_package_mode {
            self.state.acl_package_mode = acl_package_mode;
        }

        if let Some(package_operator_mode) = package_operator_mode {
            self.state.package_operator_mode = package_operator_mode;
        }

        if let Some(operator_burn_mode) = operator_burn_mode {
            self.state.operator_burn_mode = operator_burn_mode;
        }

        let mut new_acl_whitelist = acl_whitelist.unwrap_or_default();

        // Deprecated in 1.4 in favor of above ARG_ACL_WHITELIST
        if let Some(new_contract_whitelist) = contract_whitelist {
            for contract in new_contract_whitelist {
                new_acl_whitelist.push(contract);
            }
        }

        if !new_acl_whitelist.is_empty() {
            match &self.state.whitelist_mode {
                WhitelistMode::Unlocked => {
                    self.state.acl_whitelist.clear();
                    for key in new_acl_whitelist {
                        self.insert_acl_entry(key, true);
                    }
                },
                WhitelistMode::Locked => return Err(NFTCoreError::InvalidWhitelistMode)
            }
        }

        match &self.state.events_mode {
            EventsMode::NoEvents => (),
            EventsMode::CEP47 => self.write_cep47_event(CEP47Event::VariablesSet),
            EventsMode::CES => self.emit_ces_event(VariablesSet::new())
        }

        Ok(())
    }

    // Mints a new token. Minting will fail if allow_minting is set to false.
    pub fn mint(
        &mut self,
        token_metadata: String,
        token_owner: Entity,
        optional_token_hash: Option<String>,
    ) -> Result<(), NFTCoreError> {
        // The contract owner can toggle the minting behavior on and off over time.
        // The contract is toggled on by default.
        // If contract minting behavior is currently toggled off we revert.
        if !self.state.allow_minting {
            return Err(NFTCoreError::MintingIsPaused);
        }

        let total_token_supply = self.state.total_token_supply;
        let minted_tokens_count = self.state.minted_tokens_count;

        // Revert if the token supply has been exhausted.
        if minted_tokens_count >= total_token_supply {
            return Err(NFTCoreError::TokenSupplyDepleted);
        }

        let caller = host::get_caller();

        // Revert if minting is private and caller is not installer.
        if MintingMode::Installer == self.state.minting_mode {
            if self.state.installer != caller {
                return Err(NFTCoreError::InvalidMinter);
            }
        }

        // Revert if minting is acl and caller is not whitelisted.
        if MintingMode::Acl == self.state.minting_mode {
            let acl_package_mode = self.state.acl_package_mode;
            if !self.is_whitelisted(caller) {
                return Err(NFTCoreError::InvalidMinter);
            }
        }

        let metadata_kinds: Vec<(NFTMetadataKind, bool)> = todo!();
        let token_identifier = match self.state.identifier_mode {
            NFTIdentifierMode::Ordinal => TokenIdentifier::Ordinal(minted_tokens_count),
            NFTIdentifierMode::Hash => TokenIdentifier::Hash(match optional_token_hash {
                Some(hash) => hash,
                None => self.generate_hash(token_metadata)
            })
        };

        for (metadata_kind, required) in metadata_kinds {
            if !required {
                continue;
            }
            let token_metadata_validation = self.validate_metadata(metadata_kind, token_metadata);
            match token_metadata_validation {
                Ok(validated_token_metadata) => self.insert_metadata(
                    token_identifier,
                    validated_token_metadata
                ),
                Err(err) => {
                    return Err(err);
                }
            }
        }

        // The contract's ownership behavior (determined at installation) determines,
        // who owns the NFT we are about to mint.()
        self.insert_token_owner(&token_identifier, token_owner);
        self.insert_token_issuer(&token_identifier, token_owner);

        // Update the forward and reverse trackers
        if NFTIdentifierMode::Hash == self.state.identifier_mode {
            self.insert_hash_id_lookups(
                minted_tokens_count,
                token_identifier.clone()
            );
        }

        // Increment the count of owned tokens.
        self.state.owned_tokens_count += 1;
        
        // Increment number_of_minted_tokens by one
        self.state.minted_tokens_count += 1;

        // Emit mint event
        match self.state.events_mode {
            EventsMode::NoEvents => {},
            EventsMode::CES => self.emit_ces_event(Mint::new(
                Self::unwrap_entity(token_owner),
                token_identifier.clone(),
                token_metadata
            )),
            EventsMode::CEP47 => self.write_cep47_event(CEP47Event::Mint {
                recipient: Self::unwrap_entity(token_owner),
                token_id: token_identifier.clone()
            })
        }

        // TODO: Support reporting_mode. It should be thought through
        // how to best implement this mechanism in the new vm.

        Ok(())
    }

    // Marks token as burnt. This blocks any future call to transfer token.
    fn burn(&mut self, token_identifier: TokenIdentifier) -> Result<(), NFTCoreError> {
        if let BurnMode::NonBurnable = self.state.burn_mode {
            return Err(NFTCoreError::InvalidBurnMode);
        }

        let caller = host::get_caller();
        let token_owner = self.read_token_owner(&token_identifier);
        
        // Check if caller is owner
        let is_owner = token_owner == caller;

        // Check if caller is operator to execute burn
        // With operator package mode check if caller's package is operator to let contract execute burn
        let is_operator = if !is_owner {
            self.read_operator(token_owner, caller)
        } else {
            false
        };

        // Revert if caller is not token_owner nor operator for the owner
        if !is_owner && !is_operator {
            return Err(NFTCoreError::InvalidTokenOwner);
        }

        // It makes sense to keep this token as owned by the caller. It just happens that the caller
        // owns a burnt token. That's all. Similarly, we should probably also not change the
        // owned_tokens dictionary.
        if self.read_token_burned(&token_identifier) {
            return Err(NFTCoreError::PreviouslyBurntToken)
        }

        self.set_token_burned(&token_identifier);

        let updated_balance = match self.get_token_balance(token_owner) {
            Some(balance) => {
                if balance < 0u64 {
                    balance - 1u64
                } else {
                    return Err(NFTCoreError::FatalTokenIdDuplication)
                }
            },
            None => return Err(NFTCoreError::FatalTokenIdDuplication)
        };

        self.set_token_balance(token_owner, updated_balance);

        // Emit Burn event
        match self.state.events_mode {
            EventsMode::NoEvents => {},
            EventsMode::CES => {
                self.emit_ces_event(Burn::new(
                    token_owner,
                    token_identifier,
                    caller
                ))
            },
            EventsMode::CEP47 => {
                self.write_cep47_event(CEP47Event::Burn {
                    owner: token_owner,
                    token_id: token_identifier,
                    burner: caller
                })
            }
        };

        Ok(())
    }

    pub fn approve(&mut self, operator: Option<Entity>, spender: Option<Entity>) -> Result<(), NFTCoreError> {
        // If we are in minter or assigned mode it makes no sense to approve an account. Hence we
        // revert.
        if let OwnershipMode::Minter | OwnershipMode::Assigned =
            self.state.ownership_mode
        {
            return Err(NFTCoreError::InvalidOwnershipMode)
        }

        let caller = host::get_caller();
    
        let token_id = self.get_token_identifier();
        let number_of_minted_tokens = self.state.minted_tokens_count;
    
        if let NFTIdentifierMode::Ordinal = self.state.identifier_mode {
            // Revert if token_id is out of bounds
            if let TokenIdentifier::Ordinal(index) = &token_id {
                if *index >= number_of_minted_tokens {
                    return Err(NFTCoreError::InvalidTokenIdentifier);
                }
            }
        }
    
        let owner = self.read_token_owner(&token_id);
    
        // Revert if caller is not token owner nor operator.
        // Only the token owner or an operator can approve an account
        let is_owner = caller == owner;
        let is_operator = !is_owner && self.read_operator(owner, caller);
    
        if !is_owner && !is_operator {
            return Err(NFTCoreError::InvalidTokenOwner);
        }
    
        // We assume a burnt token cannot be approved
        if self.read_token_burned(&token_id) {
            return Err(NFTCoreError::PreviouslyBurntToken)
        }

        let spender = match operator {
            None => match spender {
                Some(spender) => spender,
                None => return Err(NFTCoreError::MissingSpenderAccountHash)
            },
            // Deprecated in favor of spender
            Some(operator) => operator,
        };
    
        // If token owner or operator tries to approve itself that's probably a mistake and we revert.
        if caller == spender {
            return Err(NFTCoreError::InvalidAccount);
        }
        
        self.set_approved(&token_id, spender);
    
        // Emit Approval event.
        let owner = Self::unwrap_entity(owner);
        let spender = Self::unwrap_entity(spender);
        match self.state.events_mode {
            EventsMode::NoEvents => {}
            EventsMode::CES => self.emit_ces_event(Approval::new(owner, spender, token_id)),
            EventsMode::CEP47 => self.write_cep47_event(CEP47Event::ApprovalGranted {
                owner,
                spender,
                token_id,
            }),
        };

        Ok(())
    }

    // Revokes an account as approved for an identified token transfer
    pub fn revoke(&mut self) -> Result<(), NFTCoreError> {
        // If we are in minter or assigned mode it makes no sense to approve an account. Hence we
        // revert.
        if let OwnershipMode::Minter | OwnershipMode::Assigned =
            self.state.ownership_mode
        {
            return Err(NFTCoreError::InvalidOwnershipMode)
        }

        let caller = host::get_caller();
    
        let token_id = self.get_token_identifier();
        let number_of_minted_tokens = self.state.minted_tokens_count;

        if let NFTIdentifierMode::Ordinal = self.state.identifier_mode {
            // Revert if token_id is out of bounds
            if let TokenIdentifier::Ordinal(index) = &token_id {
                if *index >= number_of_minted_tokens {
                    return Err(NFTCoreError::InvalidTokenIdentifier);
                }
            }
        }

        let owner = self.read_token_owner(&token_id);
    
        // Revert if caller is not token owner nor operator.
        // Only the token owner or an operator can approve an account
        let is_owner = caller == owner;
        let is_operator = !is_owner && self.read_operator(owner, caller);
    
        if !is_owner && !is_operator {
            return Err(NFTCoreError::InvalidTokenOwner);
        }
    
        // We assume a burnt token cannot be approved
        if self.read_token_burned(&token_id) {
            return Err(NFTCoreError::PreviouslyBurntToken)
        }

        self.clear_approved(&token_id);

        let owner = Self::unwrap_entity(owner);
        match self.state.events_mode {
            EventsMode::NoEvents => {}
            EventsMode::CES => self.emit_ces_event(ApprovalRevoked::new(owner, token_id)),
            EventsMode::CEP47 => self.write_cep47_event(CEP47Event::ApprovalRevoked {
                owner,
                token_id,
            }),
        };

        Ok(())
    }

    // Approves the specified operator for transfer of owner's tokens.
    pub fn set_approval_for_all(
        &mut self,
        approve_all: bool,
        operator: Entity
    ) -> Result<(), NFTCoreError> {
        // If we are in minter or assigned mode it makes no sense to approve an operator. Hence we
        // revert.
        if let OwnershipMode::Minter | OwnershipMode::Assigned =
            self.state.ownership_mode
        {
            return Err(NFTCoreError::InvalidOwnershipMode)
        }

        let caller = host::get_caller();

        // If caller tries to approve itself as operator that's probably a mistake and we revert.
        if caller == operator {
            return Err(NFTCoreError::InvalidAccount);
        }

        // Depending on approve_all we either approve all or disapprove all.
        self.set_operator_from_caller(caller, operator, approve_all);

        let caller = Self::unwrap_entity(caller);
        let operator = Self::unwrap_entity(operator);
        match self.state.events_mode {
            EventsMode::NoEvents => {}
            EventsMode::CES => {
                match approve_all {
                    true => self.emit_ces_event(ApprovalForAll::new(caller, operator)),
                    false => self.emit_ces_event(RevokedForAll::new(caller, operator))
                }
            }
            EventsMode::CEP47 => {
                self.write_cep47_event(match approve_all {
                    true => CEP47Event::ApprovalForAll { owner: caller, operator },
                    false => CEP47Event::RevokedForAll { owner: caller, operator },
                });
            }
        }

        Ok(())
    }

    pub fn is_approved_for_all(
        &self,
        owner: Entity,
        operator: Entity
    ) -> Result<bool, NFTCoreError> {    
        let is_operator = self.read_operator_from_owner(owner, operator);
        Ok(is_operator)
    }

    // Transfers token from token owner to specified account. Transfer will go through if caller is
    // owner or an approved account or an operator. Transfer will fail if OwnershipMode is Minter or
    // Assigned.
    pub fn transfer(&mut self, source_owner: Entity, target_owner: Entity) -> Result<(), NFTCoreError> {
        // If we are in minter or assigned mode we are not allowed to transfer ownership of token, hence
        // we revert.
        if let OwnershipMode::Minter | OwnershipMode::Assigned =
            self.state.ownership_mode
        {
            return Err(NFTCoreError::InvalidOwnershipMode)
        }

        let identifier_mode = self.state.identifier_mode.clone();
        let token_identifier = self.get_token_identifier();

        if self.read_token_burned(&token_identifier) {
            return Err(NFTCoreError::PreviouslyBurntToken);
        }

        let owner = self.read_token_owner(&token_identifier);

        if source_owner != owner {
            return Err(NFTCoreError::InvalidAccount);
        }

        let caller = host::get_caller();

        // Check if caller is owner
        let is_owner = owner == caller;

        // Check if caller is approved to execute transfer
        let is_approved = !is_owner
            && match self.get_approved(&token_identifier) {
                Ok(Some(maybe_approved)) => caller == maybe_approved,
                Ok(None) | Err(_) => false,
            };

        // Check if caller is operator to execute transfer
        let is_operator = if !is_owner && !is_approved {
            self.read_operator(source_owner, caller)
        } else {
            false
        };

        // TODO: Investiagate this later
        // if let Some(filter_contract) = utils::get_transfer_filter_contract() {
        //     let mut args = RuntimeArgs::new();
        //     args.insert(ARG_SOURCE_KEY, source_owner_key).unwrap();
        //     args.insert(ARG_TARGET_KEY, owner).unwrap();

        //     match &token_identifier {
        //         TokenIdentifier::Index(idx) => {
        //             args.insert(ARG_TOKEN_ID, *idx).unwrap();
        //         }
        //         TokenIdentifier::Hash(hash) => {
        //             args.insert(ARG_TOKEN_ID, hash.clone()).unwrap();
        //         }
        //     }

        //     let result: TransferFilterContractResult =
        //         call_contract::<u8>(filter_contract, TRANSFER_FILTER_CONTRACT_METHOD, args).into();
        //     if TransferFilterContractResult::DenyTransfer == result {
        //         revert(NFTCoreError::TransferFilterContractDenied);
        //     }
        // }

        // Revert if caller is not owner nor approved nor an operator.
        if !is_owner && !is_approved && !is_operator {
            return Err(NFTCoreError::InvalidTokenOwner);
        }

        // if NFTIdentifierMode::Hash == identifier_mode && runtime::get_key(OWNED_TOKENS).is_some() {
        //     if utils::should_migrate_token_hashes(source_owner_key) {
        //         utils::migrate_token_hashes(source_owner_key)
        //     }

        //     if utils::should_migrate_token_hashes(target_owner_key) {
        //         utils::migrate_token_hashes(target_owner_key)
        //     }
        // }

        if self.read_token_owner(&token_identifier) != source_owner {
            return Err(NFTCoreError::InvalidTokenOwner);
        }
        self.insert_token_owner(&token_identifier, target_owner);

        // Update the from_account balance
        match self.get_token_balance(source_owner) {
            Some(balance) => {
                self.set_token_balance(
                    source_owner,
                    if balance > 0u64 {
                        balance - 1u64
                    } else {
                        // This should never happen...
                        return Err(NFTCoreError::FatalTokenIdDuplication)
                    }
                ).unwrap();
            },
            None => return Err(NFTCoreError::FatalTokenIdDuplication),
        }

        // Update the to_account balance
        let updated_to_account_balance = match self.get_token_balance(target_owner) {
            Some(balance) => balance + 1u64,
            None => 1u64
        };
        self.set_token_balance(target_owner, updated_to_account_balance);
        self.clear_approved(&token_identifier);

        match self.state.events_mode {
            EventsMode::NoEvents => {},
            EventsMode::CEP47 => self.write_cep47_event(CEP47Event::Transfer {
                sender: Self::unwrap_entity(source_owner),
                recipient: Self::unwrap_entity(target_owner),
                token_id: token_identifier
            }),
            EventsMode::CES => {
                let spender = if caller == owner { None } else { Some(Self::unwrap_entity(caller)) };
                self.emit_ces_event(Transfer::new(
                    Self::unwrap_entity(owner),
                    spender,
                    Self::unwrap_entity(target_owner),
                    token_identifier
                ));
            }
        }

        // TODO: Implement rev lookup mode
        // let reporting_mode = utils::get_reporting_mode();
        // if let OwnerReverseLookupMode::Complete | OwnerReverseLookupMode::TransfersOnly = reporting_mode
        // {
        //     // Update to_account owned_tokens. Revert if owned_tokens list is not found
        //     let tokens_count = utils::get_token_index(&token_identifier);
        //     if OwnerReverseLookupMode::TransfersOnly == reporting_mode {
        //         utils::add_page_entry_and_page_record(tokens_count, &source_owner_item_key, false);
        //     }

        //     let (page_table_entry, page_uref) = utils::update_page_entry_and_page_record(
        //         tokens_count,
        //         &source_owner_item_key,
        //         &target_owner_item_key,
        //     );

        //     let owned_tokens_actual_key = Key::dictionary(page_uref, source_owner_item_key.as_bytes());

        //     let receipt_string = utils::get_receipt_name(page_table_entry);

        //     let receipt = CLValue::from_t((receipt_string, owned_tokens_actual_key))
        //         .unwrap_or_revert_with(NFTCoreError::FailedToConvertToCLValue);
        //     runtime::ret(receipt)
        // }

        Ok(())
    }

    pub fn balance_of(
        &self,
        owner: Entity,
    ) -> Result<u64, NFTCoreError> {
        let balance = self.get_token_balance(owner).unwrap_or(0);
        Ok(balance)
    }

    pub fn owner_of(
        &self,
        identifier: TokenIdentifier,
    ) -> Result<Entity, NFTCoreError> {
        let number_of_minted_tokens = self.state.minted_tokens_count;

        // Revert if token_id is out of bounds
        if let NFTIdentifierMode::Ordinal = self.state.identifier_mode {
            if let TokenIdentifier::Ordinal(ord) = identifier {
                if ord >= number_of_minted_tokens {
                    return Err(NFTCoreError::InvalidTokenIdentifier);
                }
            }
        }

        let owner = self.read_token_owner(&identifier);

        Ok(owner)
    }

    fn unwrap_entity(entity: Entity) -> Address {
        match entity {
            Entity::Account(address) => address,
            Entity::Contract(address) => address
        }
    }

    // utils::get_dictionary_value_from_key (utils::make_key(owner, caller))
    fn read_operator_from_owner(
        &self,
        caller: Entity,
        owner: Entity
    ) -> bool {
        todo!()
    }

    // utils::upsert_dictionary_value_from_key
    fn set_operator_from_caller(
        &mut self,
        caller: Entity,
        operator: Entity,
        value: bool
    ) {
        todo!()
    }

    fn clear_approved(
        &mut self,
        token_identifier: &TokenIdentifier
    ) -> Result<(), NFTCoreError> {
        // utils::upsert_dictionary_value_from_key(
        //     APPROVED,
        //     &token_identifier_dictionary_key,
        //     Some(spender),
        // );
        todo!()
    }

    fn set_approved(
        &mut self,
        token_identifier: &TokenIdentifier,
        entity: Entity
    ) -> Result<(), NFTCoreError> {
        // utils::upsert_dictionary_value_from_key(
        //     APPROVED,
        //     &token_identifier_dictionary_key,
        //     Some(spender),
        // );
        todo!()
    }

    fn get_approved(
        &mut self,
        token_identifier: &TokenIdentifier
    ) -> Result<Option<Entity>, NFTCoreError> {
        todo!()
    }

    /// Below are a lot of read/write convenience methods.
    /// Most of these would be utils::get_ / utils::upsert_ in the original
    /// ref impl. I'll later decide how to actually store these.
    fn set_token_balance(
        &mut self,
        owner: Entity,
        count: u64
    ) -> Result<(), NFTCoreError> {
        // This should never underflow
        todo!()
    }

    // originally utils::get_token_identifier_from_runtime_args
    fn get_token_identifier(&self) -> TokenIdentifier {
        todo!()
    }

    fn get_token_balance(&self, owner: Entity) -> Option<u64> {
        todo!()
    }

    fn set_token_burned(&mut self, token_identifier: &TokenIdentifier) {
        todo!()
    }

    // originally utils::is_token_burned
    fn read_token_burned(&self, token_identifier: &TokenIdentifier) -> bool {
        todo!()
    }

    // Check if caller is operator to execute burn
    // With operator package mode check if caller's package is operator to let contract execute burn
    fn read_operator(&self, token_owner: Entity, caller: Entity) -> bool {
        todo!()
    }

    // originally utils::insert_hash_id_lookups
    fn insert_hash_id_lookups(
        &mut self,
        minted_tokens_count: u64,
        token_identifier: TokenIdentifier
    ) {
        todo!()
    }

    fn insert_metadata(
        &mut self,
        identifier: TokenIdentifier,
        metadata: String
    ) {
        todo!()
    }

    fn insert_token_issuer(
        &mut self,
        token_identifier: &TokenIdentifier,
        issuer: Entity
    ) {
        todo!()
    }

    fn read_token_owner(
        &self,
        token_identifier: &TokenIdentifier
    ) -> Entity {
        todo!()
    }

    fn insert_token_owner(
        &mut self,
        token_identifier: &TokenIdentifier,
        owner: Entity
    ) {
        todo!()
    }

    // This is metadata::validate_metadata in the reference impl.
    fn validate_metadata(
        &self,
        kind: NFTMetadataKind,
        metadata: String
    ) -> Result<String, NFTCoreError> {
        todo!()
    }

    fn generate_hash(&self, metadata: String) -> String {
        // Originally, CEP78 would generate a hash with
        // base16::encode_lower(&runtime::blake2b(token_metadata.clone()))

        // I'm not sure if this is a part of the standard, or if that's just
        // what the reference implementation went with. I think so long as
        // the hashing method won't generally colide within the context of
        // a singular contract, it's fine to use a different algorithm.

        // This should be investigated later.
        todo!()
    }

    fn is_whitelisted(&self, key: Entity) -> bool {
        todo!()
    }

    fn insert_acl_entry(&mut self, key: Address, access: bool) {
        todo!()                                                  
    }

    fn write_cep47_event(&mut self, event: CEP47Event) {
        todo!()
    }

    fn emit_ces_event(&mut self, event: impl Event) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use casper_sdk::{
        host::{
            self,
            native::{current_environment, Environment, DEFAULT_ADDRESS},
        },
        types::Address,
        Contract,
    };

    // This is only done to see if the vm doesn't explode.
    // Proper tests should be ported from the reference impl;

    #[test]
    fn it_works() {
        let stub = Environment::new(Default::default(), DEFAULT_ADDRESS);

        let result = host::native::dispatch_with(stub, || {
            let contract = NFTContract::new(
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
}
