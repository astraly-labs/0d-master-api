use starknet::ContractAddress;

/// Interface for the Proxy contract that can deposit assets into an ERC4626 vault 
/// Open Zeppelin implementation.
///
/// This proxy wraps ERC4626 vault deposits to track partner referrals via `partner_id`.
/// It allows users to deposit assets into an underlying vault while associating the
/// deposit with a referral partner for tracking purposes.
#[starknet::interface]
pub trait IOZVaultDepositProxy<TContractState> {
    /// Deposits assets into the underlying vault on behalf of the receiver.
    ///
    /// # Arguments
    /// * `assets` - The amount of underlying assets to deposit
    /// * `receiver` - The address that will receive the vault shares
    /// * `partner_id` - The referral partner identifier for tracking purposes
    ///
    /// # Returns
    /// The amount of vault shares minted to the receiver
    ///
    /// # Prerequisites
    /// The caller must have approved this contract to spend at least `assets` amount
    /// of the underlying asset before calling this function.
    fn deposit(ref self: TContractState, assets: u256, receiver: ContractAddress, partner_id: felt252) -> u256;

    /// Returns the address of the underlying ERC4626 vault.
    ///
    /// # Returns
    /// The contract address of the vault
    fn vault(self: @TContractState) -> ContractAddress;

    /// Returns the address of the underlying vaults asset token.
    /// This is the address of the ERC20 token that should be approved before deposits.
    ///
    /// # Returns
    /// The contract address of the asset (ERC20 token)
    fn asset(self: @TContractState) -> ContractAddress;
}

/// Proxy for depositing into an OpenZeppelin ERC4626 Vault
/// augmenting with partner ID to do a referral program.
/// 
/// IMPORTANT: In order to deposit assets owner should first approve those to this contract.
/// then do deposit call.
/// 
/// Assets will be transferred from the contract to the underlying vault.
/// 
#[starknet::contract]
pub mod OZVaultDepositProxy {
    use core::num::traits::Zero;
use starknet::storage::{StoragePointerWriteAccess, StoragePointerReadAccess};
    use openzeppelin::interfaces::token::erc4626::{IERC4626Dispatcher, IERC4626DispatcherTrait};
    use openzeppelin::interfaces::token::erc20::{IERC20Dispatcher, IERC20DispatcherTrait};
    use super::ContractAddress;

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        Deposit: Deposit,
    }

    #[derive(Drop, starknet::Event)]
    pub struct Deposit {
        #[key]
        pub sender: ContractAddress,
        #[key]
        pub owner: ContractAddress,
        pub assets: u256,
        pub shares: u256,
        pub partner_id: felt252,
    }

    #[storage]
    struct Storage {
        vault_address: ContractAddress,
    }

    #[constructor]
    fn constructor(ref self: ContractState, vault_address: ContractAddress) {
        assert(vault_address.is_zero(), 'vault_addr_is_zero');

        self.vault_address.write(vault_address);
    }

    #[abi(embed_v0)]
    impl OZVaultDepositProxyImpl of super::IOZVaultDepositProxy<ContractState> {
        fn deposit(ref self: ContractState, assets: u256, receiver: ContractAddress, partner_id: felt252) -> u256 {
            let caller = starknet::get_caller_address();

            let vault_address = self.vault_address.read();
            let vault = IERC4626Dispatcher { contract_address: vault_address };

            let asset = IERC20Dispatcher { contract_address: vault.asset() };

            let proxy_address = starknet::get_contract_address();

            assert(asset.allowance(caller, proxy_address) >= assets, 'not_enought_allowance');

            // Transfer assets from caller to this contract
            asset.transfer_from(caller, proxy_address, assets);

            // Approve underlying vault to pull assets
            asset.approve(vault_address, assets);

            // Execute deposit
            let shares = vault.deposit(assets, receiver);
            // Emit event
            self.emit(Deposit { 
                sender: caller, 
                owner: receiver, 
                assets, 
                shares,
                partner_id 
            });
            
            return shares;
        }
        
        fn asset(self: @ContractState) -> ContractAddress {
            let vault_address = self.vault_address.read();
            let vault = IERC4626Dispatcher { contract_address: vault_address };
            return vault.asset();
        }
        
        fn vault(self: @ContractState) -> ContractAddress {
            return self.vault_address.read();
        }
    }
}
