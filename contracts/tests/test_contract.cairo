use starknet::{ContractAddress};

use snforge_std::{declare, ContractClassTrait, DeclareResultTrait};

#[starknet::contract]
mod OZAsset {
    use openzeppelin::token::erc20::{
        ERC20Component, ERC20HooksEmptyImpl, DefaultConfig as ERC20DefaultConfig
    };
    use openzeppelin::token::erc20::extensions::erc4626::{
        ERC4626EmptyHooks, DefaultConfig as ERC4626DefaultConfig,
    };
    use starknet::ContractAddress;

    component!(path: ERC20Component, storage: erc20, event: ERC20Event);

    #[abi(embed_v0)]
    impl ERC20MixinImpl = ERC20Component::ERC20MixinImpl<ContractState>;
    impl ERC20InternalImpl = ERC20Component::InternalImpl<ContractState>;

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        ERC20Event: ERC20Component::Event,
    }

    #[storage]
    struct Storage {
        #[substorage(v0)]
        erc20: ERC20Component::Storage,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        owner: ContractAddress,
        initial_supply: felt252,
    ) {
        let name = "Asset";
        let symbol = "ASS";

        self.erc20.initializer(name, symbol);
        self.erc20.mint(owner, initial_supply.into());
    }

}

#[starknet::contract]
mod OZVault {

    use starknet::storage::StoragePointerReadAccess;
    use openzeppelin::token::erc20::{
        ERC20Component, ERC20HooksEmptyImpl, DefaultConfig as ERC20DefaultConfig
    };

    use openzeppelin::interfaces::erc20::{IERC20Dispatcher, IERC20DispatcherTrait};

    use openzeppelin::token::erc20::extensions::erc4626::{
        ERC4626Component, ERC4626EmptyHooks, DefaultConfig as ERC4626DefaultConfig,
    };
    use starknet::ContractAddress;

    component!(path: ERC20Component, storage: erc20, event: ERC20Event);
    component!(path: ERC4626Component, storage: erc4626, event: ERC4626Event);

    #[abi(embed_v0)]
    impl ERC20MixinImpl = ERC20Component::ERC20MixinImpl<ContractState>;
    impl ERC20InternalImpl = ERC20Component::InternalImpl<ContractState>;

    #[abi(embed_v0)]
    impl ERC4626MixinImpl = ERC4626Component::ERC4626Impl<ContractState>;
    impl ERC4626InternalImpl = ERC4626Component::InternalImpl<ContractState>;

    impl ERC4626FeeConfig of ERC4626Component::FeeConfigTrait<ContractState> {}
    impl ERC4626LimitConfigTrait of ERC4626Component::LimitConfigTrait<ContractState> {}

    impl ERC4626AssetsManagement of ERC4626Component::AssetsManagementTrait<ContractState> {
        fn get_total_assets(self: @ERC4626Component::ComponentState<ContractState>) -> u256 {
            IERC20Dispatcher {
                contract_address: self.ERC4626_asset.read()
            }.total_supply()
        }

        fn transfer_assets_in(ref self: ERC4626Component::ComponentState<ContractState>, from: ContractAddress, assets: u256) {
            IERC20Dispatcher {
                contract_address: self.ERC4626_asset.read()
            }.transfer_from(from, starknet::get_contract_address(), assets);
        }

        fn transfer_assets_out(ref self: ERC4626Component::ComponentState<ContractState>, to: ContractAddress, assets: u256) {
            IERC20Dispatcher {
                contract_address: self.ERC4626_asset.read()
            }.transfer_from(starknet::get_contract_address(), to, assets);
        }        
    }


    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        ERC20Event: ERC20Component::Event,
        #[flat]
        ERC4626Event: ERC4626Component::Event
    }

    #[storage]
    struct Storage {
        #[substorage(v0)]
        erc20: ERC20Component::Storage,
        #[substorage(v0)]
        erc4626: ERC4626Component::Storage
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        asset_address: ContractAddress,
        initial_supply: felt252,
    ) {
        let name = "Sharing is caring";
        let symbol = "SHR";

        self.erc20.initializer(name, symbol);
        self.erc20.mint(starknet::get_contract_address(), initial_supply.into());
        self.erc4626.initializer(asset_address);
    }
}

fn deploy_vault(asset: ContractAddress, initial_shares: u128) -> ContractAddress {
    let contract = declare("OZVault").unwrap().contract_class();

    let asset_address: felt252 = asset.into();
    let calldata = array![asset_address, initial_shares.into()];

    let (contract_address, _) = contract.deploy(@calldata).unwrap();
    
    contract_address
}

fn deploy_asset(owner: ContractAddress, initial_supply: u128) -> ContractAddress {
    let contract = declare("OZAsset").unwrap().contract_class();
    let (contract_address, _) = contract.deploy(@array![owner.into(), initial_supply.into()]).unwrap();
    contract_address
}

fn deploy_vault_proxy(vault_address: ContractAddress) -> ContractAddress {
    let contract = declare("OZVaultDepositProxy").unwrap().contract_class();
    let (contract_address, _) = contract.deploy(@array![vault_address.into()]).unwrap();
    contract_address
}

#[cfg(test)]
mod test {
    use super::*;
    use snforge_std::cheat_caller_address;
    use snforge_std::cheatcodes::generate_random_felt::generate_random_felt;
    use snforge_std::{spy_events, EventSpyAssertionsTrait};
    
    use oz_vault_deposit_proxy::{IOZVaultDepositProxyDispatcher, IOZVaultDepositProxyDispatcherTrait, OZVaultDepositProxy};
    use openzeppelin::interfaces::erc20::{ERC20ABIDispatcher, ERC20ABIDispatcherTrait};
    use openzeppelin::interfaces::erc4626::{ERC4626ABIDispatcher, ERC4626ABIDispatcherTrait};
    
    #[test]
    fn test_proxy_flow() {
        //
        // Preparation:
        //
        // 1. Generate some asset owner address: random

        let asset_supply = 100000000000_u256;
        let share_supply = 100000000000_u256;
        let partner_id: felt252 = 'argent'.try_into().unwrap();
        
        let asset_owner: ContractAddress = generate_random_felt().try_into().unwrap();
        let deposit_amount: u256 = 100;

        // 2. Deploy asset contract, mint some assets to asset owner

        let asset_address = deploy_asset(asset_owner, asset_supply.try_into().unwrap());

        let asset = ERC20ABIDispatcher {
            contract_address: asset_address
        };
        assert_eq!(asset.balance_of(asset_owner), asset_supply.into());

        // 3. Deploy vault contract, pointing at assets
        let vault_address = deploy_vault(asset_address, share_supply.try_into().unwrap());
        let vault = ERC4626ABIDispatcher {
            contract_address: vault_address
        };
        
        // 4. Deploy vault proxy contract, pointing at vault contract
        let proxy_address = deploy_vault_proxy(vault_address);

        // Test flow:
        // 1. Owner approves proxy to spend their assets
        cheat_caller_address(asset_address, asset_owner, snforge_std::CheatSpan::TargetCalls(1));
        asset.approve(
            proxy_address,
            deposit_amount
        );

        assert_eq!(asset.allowance(
            asset_owner,
            proxy_address
        ), 100);


        let proxy = IOZVaultDepositProxyDispatcher {
            contract_address: proxy_address
        };

        assert_eq!(proxy.vault(), vault_address);
        assert_eq!(proxy.asset(), asset_address);

        let mut spy = spy_events();

        // 1. Owner performs deposit operation against proxy
        cheat_caller_address(proxy_address, asset_owner, snforge_std::CheatSpan::TargetCalls(1));
        let shares = proxy.deposit(deposit_amount, asset_owner, partner_id);

        assert_eq!(vault.total_assets(), asset_supply.into());
        assert_eq!(vault.total_supply(), (share_supply + shares).into());

        // Checking shares minted 
        assert_gt!(shares, 0_u256);

        // Checking if owner assets spent
        assert_eq!(asset.balance_of(asset_owner), (asset_supply - deposit_amount).into());

        // Checking if vault received assets
        assert_eq!(asset.balance_of(vault_address), deposit_amount.into());

        // Checking proxy owns nothing
        assert_eq!(asset.balance_of(proxy_address), 0.into());

        // Checking proxy owes nothing
        assert_eq!(asset.allowance(proxy_address, vault_address), 0.into());

        // Checking that event was emitted
        spy.assert_emitted(@array![
            (
                proxy_address,
                OZVaultDepositProxy::Event::Deposit(
                    OZVaultDepositProxy::Deposit {
                        sender: asset_owner,
                        owner: asset_owner,
                        assets: deposit_amount,
                        shares,
                        partner_id,
                    }
                )
            )
        ]);

    }
}

