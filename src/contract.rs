use std::collections::BTreeMap;

use blake2b_simd::blake2b;
use casper_macros::*;
use casper_sdk::*;
use host::{native::DEFAULT_ADDRESS, Entity};
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
            owned_tokens_count: 0,
            store: Default::default()
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
        let installer = Entity::Account(DEFAULT_ADDRESS);
        let events_mode = events_mode.unwrap_or(EventsMode::NoEvents);
        let minted_tokens_count = 0u64;
        let owned_tokens_count = 0u64;
        let store = StateStore::default();

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
            installer,
            store
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
        acl_whitelist: Option<Vec<Entity>>,
        contract_whitelist: Option<Vec<Entity>>
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
    ) -> Result<TokenIdentifier, NFTCoreError> {
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
            if !self.is_whitelisted(caller) {
                return Err(NFTCoreError::InvalidMinter);
            }
        }

        let metadata_kinds: Vec<(NFTMetadataKind, bool)> = Vec::new(); // TODO: support this modality!
        let token_identifier = match self.state.identifier_mode {
            NFTIdentifierMode::Ordinal => TokenIdentifier::Ordinal(minted_tokens_count),
            NFTIdentifierMode::Hash => TokenIdentifier::Hash(match optional_token_hash {
                Some(hash) => hash,
                None => self.generate_hash(token_metadata.clone())
            })
        };

        for (metadata_kind, required) in metadata_kinds {
            if !required {
                continue;
            }
            let token_metadata_validation = self.validate_metadata(metadata_kind, token_metadata.clone());
            match token_metadata_validation {
                Ok(validated_token_metadata) => self.insert_metadata(
                    &token_identifier,
                    &validated_token_metadata
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
            if let Err(e) = self.insert_hash_id_lookups(&token_identifier) {
                return Err(e);
            }
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

        Ok(token_identifier)
    }

    // Marks token as burnt. This blocks any future call to transfer token.
    pub fn burn(&mut self, token_identifier: TokenIdentifier) -> Result<(), NFTCoreError> {
        if let BurnMode::NonBurnable = self.state.burn_mode {
            return Err(NFTCoreError::InvalidBurnMode);
        }

        let caller = host::get_caller();
        let Some(token_owner) = self.read_token_owner(&token_identifier) else {
            return Err(NFTCoreError::MissingTokenOwner);
        };
        
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

        self.set_token_burned(token_identifier.clone());

        let updated_balance = match self.get_token_balance(token_owner) {
            Some(balance) => {
                if balance > 0u64 {
                    balance - 1u64
                } else {
                    return Err(NFTCoreError::FatalTokenIdDuplication)
                }
            },
            None => return Err(NFTCoreError::FatalTokenIdDuplication)
        };

        if let Err(e) = self.set_token_balance(token_owner, updated_balance) {
            return Err(e);
        }

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

    pub fn approve(&mut self, operator: Option<Entity>, spender: Entity, token_identifier: TokenIdentifier) -> Result<(), NFTCoreError> {
        // If we are in minter or assigned mode it makes no sense to approve an account. Hence we
        // revert.
        if let OwnershipMode::Minter | OwnershipMode::Assigned =
            self.state.ownership_mode
        {
            return Err(NFTCoreError::InvalidOwnershipMode)
        }

        let caller = host::get_caller();

        let number_of_minted_tokens = self.state.minted_tokens_count;
    
        if let NFTIdentifierMode::Ordinal = self.state.identifier_mode {
            // Revert if token_id is out of bounds
            if let TokenIdentifier::Ordinal(index) = &token_identifier {
                if *index >= number_of_minted_tokens {
                    return Err(NFTCoreError::InvalidTokenIdentifier);
                }
            }
        }
    
        let Some(owner) = self.read_token_owner(&token_identifier) else {
            return Err(NFTCoreError::MissingTokenOwner);
        };
    
        // Revert if caller is not token owner nor operator.
        // Only the token owner or an operator can approve an account
        let is_owner = caller == owner;
        let is_operator = !is_owner && self.read_operator(owner, caller);
    
        if !is_owner && !is_operator {
            return Err(NFTCoreError::InvalidTokenOwner);
        }
    
        // We assume a burnt token cannot be approved
        if self.read_token_burned(&token_identifier) {
            return Err(NFTCoreError::PreviouslyBurntToken)
        }

        let spender = match operator {
            None => spender,
            // Deprecated in favor of spender
            Some(operator) => operator,
        };
    
        // If token owner or operator tries to approve itself that's probably a mistake and we revert.
        if caller == spender {
            return Err(NFTCoreError::InvalidAccount);
        }
        
        if let Err(e) = self.set_approved(&token_identifier, spender) {
            return Err(e);
        }
    
        // Emit Approval event.
        let owner = Self::unwrap_entity(owner);
        let spender = Self::unwrap_entity(spender);
        match self.state.events_mode {
            EventsMode::NoEvents => {}
            EventsMode::CES => self.emit_ces_event(Approval::new(owner, spender, token_identifier)),
            EventsMode::CEP47 => self.write_cep47_event(CEP47Event::ApprovalGranted {
                owner,
                spender,
                token_id: token_identifier,
            }),
        };

        Ok(())
    }

    // Revokes an account as approved for an identified token transfer
    pub fn revoke(&mut self, token_identifier: TokenIdentifier) -> Result<(), NFTCoreError> {
        // If we are in minter or assigned mode it makes no sense to approve an account. Hence we
        // revert.
        if let OwnershipMode::Minter | OwnershipMode::Assigned =
            self.state.ownership_mode
        {
            return Err(NFTCoreError::InvalidOwnershipMode)
        }

        let caller = host::get_caller();

        let number_of_minted_tokens = self.state.minted_tokens_count;

        if let NFTIdentifierMode::Ordinal = self.state.identifier_mode {
            // Revert if token_id is out of bounds
            if let TokenIdentifier::Ordinal(index) = &token_identifier {
                if *index >= number_of_minted_tokens {
                    return Err(NFTCoreError::InvalidTokenIdentifier);
                }
            }
        }

        let Some(owner) = self.read_token_owner(&token_identifier) else {
            return Err(NFTCoreError::MissingTokenOwner);
        };
    
        // Revert if caller is not token owner nor operator.
        // Only the token owner or an operator can approve an account
        let is_owner = caller == owner;
        let is_operator = !is_owner && self.read_operator(owner, caller);
    
        if !is_owner && !is_operator {
            return Err(NFTCoreError::InvalidTokenOwner);
        }
    
        // We assume a burnt token cannot be approved
        if self.read_token_burned(&token_identifier) {
            return Err(NFTCoreError::PreviouslyBurntToken)
        }

        if let Err(e) = self.clear_approved(&token_identifier) {
            return Err(e);
        }

        let owner = Self::unwrap_entity(owner);
        match self.state.events_mode {
            EventsMode::NoEvents => {}
            EventsMode::CES => self.emit_ces_event(ApprovalRevoked::new(owner, token_identifier)),
            EventsMode::CEP47 => self.write_cep47_event(CEP47Event::ApprovalRevoked {
                owner,
                token_id: token_identifier,
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
        self.set_operator_for_owner(caller, operator, approve_all);

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
        let is_operator = self.caller_is_operator_for_owner(owner, operator);
        Ok(is_operator)
    }

    // Transfers token from token owner to specified account. Transfer will go through if caller is
    // owner or an approved account or an operator. Transfer will fail if OwnershipMode is Minter or
    // Assigned.
    pub fn transfer(&mut self, source_owner: Entity, target_owner: Entity, token_identifier: TokenIdentifier) -> Result<(), NFTCoreError> {
        // If we are in minter or assigned mode we are not allowed to transfer ownership of token, hence
        // we revert.
        if let OwnershipMode::Minter | OwnershipMode::Assigned =
            self.state.ownership_mode
        {
            return Err(NFTCoreError::InvalidOwnershipMode)
        }

        if self.read_token_burned(&token_identifier) {
            return Err(NFTCoreError::PreviouslyBurntToken);
        }

        let Some(owner) = self.read_token_owner(&token_identifier) else {
            return Err(NFTCoreError::MissingTokenOwner);
        };

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

        // Revert if caller is not owner nor approved nor an operator.
        if !is_owner && !is_approved && !is_operator {
            return Err(NFTCoreError::InvalidTokenOwner);
        }

        // TODO: Impl token hash migration
        // let identifier_mode = self.state.identifier_mode.clone();
        // if NFTIdentifierMode::Hash == identifier_mode && runtime::get_key(OWNED_TOKENS).is_some() {
        //     if utils::should_migrate_token_hashes(source_owner_key) {
        //         utils::migrate_token_hashes(source_owner_key)
        //     }

        //     if utils::should_migrate_token_hashes(target_owner_key) {
        //         utils::migrate_token_hashes(target_owner_key)
        //     }
        // }

        if self.read_token_owner(&token_identifier) != Some(source_owner) {
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
        self.set_token_balance(target_owner, updated_to_account_balance).ok();
        self.clear_approved(&token_identifier).ok();

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

        let Some(owner) = self.read_token_owner(&identifier) else {
            return Err(NFTCoreError::MissingTokenOwner);
        };

        Ok(owner)
    }

    fn unwrap_entity(entity: Entity) -> Address {
        match entity {
            Entity::Account(address) => address,
            Entity::Contract(address) => address
        }
    }

    fn caller_is_operator_for_owner(
        &self,
        caller: Entity,
        owner: Entity
    ) -> bool {
        for entry in &self.state.store.operators {
            if entry.key == owner && entry.value == caller {
                return true;
            }
        }

        return false;
    }

    fn set_operator_for_owner(
        &mut self,
        owner: Entity,
        operator: Entity,
        value: bool
    ) {
        if value == false {
            self.state.store.operators.retain(|entry| {
                let owned = entry.key == owner;
                let is_operator = entry.value == operator;
                let operator_for_owner = owned && is_operator;
                !operator_for_owner
            });
            return;
        }

        for entry in &self.state.store.operators {
            let owned = entry.key == owner;
            let is_operator = entry.value == operator;
            let operator_for_owner = owned && is_operator;
            if operator_for_owner {
                return;
            }
        }

        self.state.store.operators.push(OperatorEntry { 
            key: owner,
            value: operator
        });
    }

    fn clear_approved(
        &mut self,
        token_identifier: &TokenIdentifier
    ) -> Result<(), NFTCoreError> {
        if let Some(mut data) = self.state.store.data.get(token_identifier) {
            data.approved = None;
        }
        Ok(())
    }

    fn set_approved(
        &mut self,
        token_identifier: &TokenIdentifier,
        entity: Entity
    ) -> Result<(), NFTCoreError> {
        if let Some(mut data) = self.state.store.data.get(token_identifier) {
            data.approved = Some(entity);
        } else {
            return Err(NFTCoreError::InvalidTokenIdentifier);
        }

        Ok(())
    }

    fn get_approved(
        &mut self,
        token_identifier: &TokenIdentifier
    ) -> Result<Option<Entity>, NFTCoreError> {
        if let Some(data) = self.state.store.data.get(token_identifier) {
            Ok(data.approved)
        } else {
            Err(NFTCoreError::InvalidTokenIdentifier)
        }
    }

    fn set_token_balance(
        &mut self,
        owner: Entity,
        count: u64
    ) -> Result<(), NFTCoreError> {
        if let Some(mut data) = self.state.store.entity_data.get(&owner) {
            data.balance = count;
            Ok(())
        } else {
            Err(NFTCoreError::InvalidTokenIdentifier)
        }
    }

    fn get_token_balance(&self, owner: Entity) -> Option<u64> {
        if let Some(data) = self.state.store.entity_data.get(&owner) {
            Some(data.balance)
        } else {
            None
        }
    }

    fn set_token_burned(&mut self, token_identifier: TokenIdentifier) {
        self.state.store.burned_tokens.push(token_identifier);
    }

    fn read_token_burned(&self, token_identifier: &TokenIdentifier) -> bool {
        self.state.store.burned_tokens.contains(token_identifier)
    }

    // Check if caller is operator to execute burn
    fn read_operator(&self, owner: Entity, caller: Entity) -> bool {
        for entry in &self.state.store.operators {
            let owned = entry.key == owner;
            let is_operator = entry.value == caller;
            let operator_for_owner = owned && is_operator;
            if operator_for_owner {
                return true;
            }
        }
        false
    }

    fn insert_hash_id_lookups(
        &mut self,
        token_identifier: &TokenIdentifier
    ) -> Result<(), NFTCoreError> {
        let TokenIdentifier::Hash(hash) = token_identifier else {
            return Ok(());
        };

        if self.state.store.index_by_hash.get(hash).is_some() {
            return Err(NFTCoreError::DuplicateIdentifier);
        }

        if self.state.store.hash_by_index.get(&self.state.minted_tokens_count).is_some() {
            return Err(NFTCoreError::DuplicateIdentifier);
        }

        self.state.store.hash_by_index.insert(&self.state.minted_tokens_count, hash);
        self.state.store.index_by_hash.insert(hash, &self.state.minted_tokens_count);

        Ok(())
    }

    fn insert_metadata(
        &mut self,
        identifier: &TokenIdentifier,
        metadata: &String
    ) {
        self.state.store.metadata.insert(identifier, metadata);
    }

    fn insert_token_issuer(
        &mut self,
        token_identifier: &TokenIdentifier,
        issuer: Entity
    ) {
        if let Some(mut data) = self.state.store.data.get(&token_identifier) {
            data.issuer = Some(issuer);
        } else {
            let mut data = TokenData::default();
            data.issuer = Some(issuer);
            self.state.store.data.insert(&token_identifier, &data);
        }
    }

    fn read_token_owner(
        &self,
        token_identifier: &TokenIdentifier
    ) -> Option<Entity> {
        if let Some(data) = self.state.store.data.get(&token_identifier) {
            data.owner
        } else {
            None
        }
    }

    fn insert_token_owner(
        &mut self,
        token_identifier: &TokenIdentifier,
        owner: Entity
    ) {
        if let Some(mut data) = self.state.store.data.get(&token_identifier) {
            data.owner = Some(owner);
        } else {
            let mut data = TokenData::default();
            data.owner = Some(owner);
            self.state.store.data.insert(&token_identifier, &data);
        }
    }

    fn validate_metadata(
        &self,
        kind: NFTMetadataKind,
        metadata: String
    ) -> Result<String, NFTCoreError> {
        let token_schema = self.get_metadata_schema(&kind);
        match &kind {
            NFTMetadataKind::CEP78 => {
                let metadata = serde_json_wasm::from_str::<MetadataCEP78>(&metadata)
                    .map_err(|_| NFTCoreError::FailedToParseCep99Metadata)?;

                if let Some(name_property) = token_schema.properties.get("name") {
                    if name_property.required && metadata.name.is_empty() {
                        return Err(NFTCoreError::InvalidCEP99Metadata)
                    }
                }
                if let Some(token_uri_property) = token_schema.properties.get("token_uri") {
                    if token_uri_property.required && metadata.token_uri.is_empty() {
                        return Err(NFTCoreError::InvalidCEP99Metadata)
                    }
                }
                if let Some(checksum_property) = token_schema.properties.get("checksum") {
                    if checksum_property.required && metadata.checksum.is_empty() {
                        return Err(NFTCoreError::InvalidCEP99Metadata)
                    }
                }
                serde_json::to_string_pretty(&metadata)
                    .map_err(|_| NFTCoreError::FailedToJsonifyCEP99Metadata)
            }
            NFTMetadataKind::NFT721 => {
                let metadata = serde_json_wasm::from_str::<MetadataNFT721>(&metadata)
                    .map_err(|_| NFTCoreError::FailedToParse721Metadata)?;

                if let Some(name_property) = token_schema.properties.get("name") {
                    if name_property.required && metadata.name.is_empty() {
                        return Err(NFTCoreError::InvalidNFT721Metadata)
                    }
                }
                if let Some(token_uri_property) = token_schema.properties.get("token_uri") {
                    if token_uri_property.required && metadata.token_uri.is_empty() {
                        return Err(NFTCoreError::InvalidNFT721Metadata)
                    }
                }
                if let Some(symbol_property) = token_schema.properties.get("symbol") {
                    if symbol_property.required && metadata.symbol.is_empty() {
                        return Err(NFTCoreError::InvalidNFT721Metadata)
                    }
                }
                serde_json::to_string_pretty(&metadata)
                    .map_err(|_| NFTCoreError::FailedToJsonifyNFT721Metadata)
            }
            NFTMetadataKind::Raw => Ok(metadata),
            NFTMetadataKind::CustomValidated => {
                let custom_metadata =
                    serde_json_wasm::from_str::<BTreeMap<String, String>>(&metadata)
                        .map(|attributes| CustomMetadata { attributes })
                        .map_err(|_| NFTCoreError::FailedToParseCustomMetadata)?;

                for (property_name, property_type) in token_schema.properties.iter() {
                    if property_type.required && custom_metadata.attributes.get(property_name).is_none()
                    {
                        return Err(NFTCoreError::InvalidCustomMetadata)
                    }
                }
                serde_json::to_string_pretty(&custom_metadata.attributes)
                    .map_err(|_| NFTCoreError::FailedToJsonifyCustomMetadata)
            }
        }
    }

    fn get_metadata_schema(&self, kind: &NFTMetadataKind) -> CustomMetadataSchema {
        match kind {
            NFTMetadataKind::Raw => CustomMetadataSchema {
                properties: BTreeMap::new(),
            },
            NFTMetadataKind::NFT721 => {
                let mut properties = BTreeMap::new();
                properties.insert(
                    "name".to_string(),
                    MetadataSchemaProperty {
                        name: "name".to_string(),
                        description: "The name of the NFT".to_string(),
                        required: true,
                    },
                );
                properties.insert(
                    "symbol".to_string(),
                    MetadataSchemaProperty {
                        name: "symbol".to_string(),
                        description: "The symbol of the NFT collection".to_string(),
                        required: true,
                    },
                );
                properties.insert(
                    "token_uri".to_string(),
                    MetadataSchemaProperty {
                        name: "token_uri".to_string(),
                        description: "The URI pointing to an off chain resource".to_string(),
                        required: true,
                    },
                );
                CustomMetadataSchema { properties }
            }
            NFTMetadataKind::CEP78 => {
                let mut properties = BTreeMap::new();
                properties.insert(
                    "name".to_string(),
                    MetadataSchemaProperty {
                        name: "name".to_string(),
                        description: "The name of the NFT".to_string(),
                        required: true,
                    },
                );
                properties.insert(
                    "token_uri".to_string(),
                    MetadataSchemaProperty {
                        name: "token_uri".to_string(),
                        description: "The URI pointing to an off chain resource".to_string(),
                        required: true,
                    },
                );
                properties.insert(
                    "checksum".to_string(),
                    MetadataSchemaProperty {
                        name: "checksum".to_string(),
                        description: "A SHA256 hash of the content at the token_uri".to_string(),
                        required: true,
                    },
                );
                CustomMetadataSchema { properties }
            }
            NFTMetadataKind::CustomValidated => {
                let custom_schema_json = self.state.store.json_schema.as_ref().unwrap();
    
                serde_json_wasm::from_str::<CustomMetadataSchema>(custom_schema_json)
                    .map_err(|_| NFTCoreError::InvalidJsonSchema)
                    .unwrap_or_revert()
            }
        }
    }

    fn generate_hash(&self, metadata: String) -> String {
        base16::encode_lower(&blake2b(metadata.as_bytes()))
    }

    fn is_whitelisted(&self, key: Entity) -> bool {
        if let Some(data) = self.state.store.entity_data.get(&key) {
            data.whitelisted
        } else {
            false
        }
    }

    fn insert_acl_entry(&mut self, key: Entity, access: bool) {
        if let Some(mut data) = self.state.store.entity_data.get(&key) {
            data.whitelisted = access;
        }
    }

    // TODO: implement events
    fn write_cep47_event(&mut self, _event: CEP47Event) {
    }

    fn emit_ces_event(&mut self, _event: impl Event) {
    }
}