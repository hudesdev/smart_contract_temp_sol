use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    pubkey::Pubkey,
    system_instruction::{create_account, transfer},
};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
struct Product {
    owner: Pubkey,
    store: Pubkey,
    name: String,
    price: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct Store {
    name_of_store: String,
    owner: Pubkey,
}

#[derive(BorshSerialize, BorshDeserialize)]
enum Command {
    Buy,
    MakeStore(Store, u64),
    AddToStore(Product, u64),
    DeleteFromStore,
    Close,
    Wipe,
}

fn buy(accounts: &[AccountInfo]) {
    let mut accounts_iter = accounts.iter();

    let from = next_account_info(&mut accounts_iter).unwrap().key;
    let product_info = next_account_info(&mut accounts_iter).unwrap();
    let store_info = next_account_info(&mut accounts_iter).unwrap();

    let store = Store::deserialize(&mut store_info.data.borrow_mut().as_ref()).unwrap();
    let mut product = Product::deserialize(&mut product_info.data.borrow_mut().as_ref()).unwrap();

    assert_eq!(&product.store, store_info.key);

    let instruction = transfer(&from, &store.owner, product.price);
    invoke(&instruction, accounts).unwrap();

    product.owner = *from;
    product
        .serialize(&mut *product_info.try_borrow_mut_data().unwrap())
        .unwrap();
}

fn make_store(program_id: &Pubkey, accounts: &[AccountInfo], store: &Store, rent_exemption: u64) {
    let usr = &store.owner;

    let (account_pubkey, bump) =
        Pubkey::find_program_address(&[store.name_of_store.as_bytes()], program_id);

    let instruction = create_account(&usr, &account_pubkey, rent_exemption, 50 + 32, program_id);

    invoke_signed(
        &instruction,
        accounts,
        &[&[store.name_of_store.as_bytes(), &[bump]]],
    )
    .unwrap();

    let account = next_account_info(&mut accounts.into_iter()).unwrap();
    store
        .serialize(&mut *account.try_borrow_mut_data().unwrap())
        .unwrap();
}

fn add_to_store(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    product: &Product,
    rent_exemption: u64,
) {
    let accounts_iter = &mut accounts.into_iter();

    let product_info = next_account_info(accounts_iter).unwrap();
    let store_info = next_account_info(accounts_iter).unwrap();
    let usr_info = next_account_info(accounts_iter).unwrap();

    let store = Store::deserialize(&mut store_info.data.borrow_mut().as_ref())
        .expect("Cannot deserialize store account");
    assert!(product.name.len() <= 50);
    assert_eq!(&store.owner, usr_info.key);
    assert!(usr_info.is_signer);

    let store_pubkey = &product.owner;

    let (_, bump) = Pubkey::find_program_address(
        &[&store_pubkey.to_bytes(), product.name.as_bytes()],
        program_id,
    );

    let instruction = create_account(
        &usr_info.key,
        &product_info.key,
        rent_exemption,
        32 + 32 + 50 + 8 + 8,
        program_id,
    );

    invoke_signed(
        &instruction,
        accounts,
        &[&[&store_pubkey.to_bytes(), product.name.as_bytes(), &[bump]]],
    )
    .unwrap();

    product
        .serialize(&mut *product_info.try_borrow_mut_data().unwrap())
        .unwrap();
}

fn delete_from_store(accounts: &[AccountInfo]) {
    let accounts_iter = &mut accounts.iter();
    let product_info = next_account_info(accounts_iter).unwrap();
    let store_info = next_account_info(accounts_iter).unwrap();
    let owner_info = next_account_info(accounts_iter).unwrap();

    let product = Product::deserialize(&mut product_info.data.borrow_mut().as_ref()).unwrap();
    let store = Store::deserialize(&mut store_info.data.borrow_mut().as_ref()).unwrap();

    assert_eq!(&product.owner, store_info.key);
    assert_eq!(&store.owner, owner_info.key);

    **owner_info.lamports.borrow_mut() = owner_info
        .lamports()
        .checked_add(product_info.lamports())
        .unwrap();
    **product_info.lamports.borrow_mut() = 0;
}

fn close(accounts: &[AccountInfo]) {
    let accounts_iter = &mut accounts.iter();
    let store_info = next_account_info(accounts_iter).unwrap();
    let owner_info = next_account_info(accounts_iter).unwrap();

    let store = Store::deserialize(&mut store_info.data.borrow_mut().as_ref()).unwrap();
    assert_eq!(&store.owner, owner_info.key);

    **owner_info.lamports.borrow_mut() = owner_info
        .lamports()
        .checked_add(store_info.lamports())
        .unwrap();
    **store_info.lamports.borrow_mut() = 0;
}

fn wipe(accounts: &[AccountInfo]) {
    let accounts_iter = &mut accounts.iter();
    let account_info = next_account_info(accounts_iter).unwrap();
    let owner_info = next_account_info(accounts_iter).unwrap();

    **owner_info.lamports.borrow_mut() = owner_info
        .lamports()
        .checked_add(account_info.lamports())
        .unwrap();
    **account_info.lamports.borrow_mut() = 0;
}

entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let command = Command::try_from_slice(instruction_data).expect("Can't Deserialize command");

    match command {
        Command::MakeStore(store, rent_exemption) => {
            make_store(&program_id, accounts, &store, rent_exemption)
        }
        Command::Buy => buy(accounts),
        Command::AddToStore(product, rent_exemption) => {
            add_to_store(&program_id, accounts, &product, rent_exemption)
        }
        Command::DeleteFromStore => delete_from_store(accounts),
        Command::Close => close(accounts),
        Command::Wipe => wipe(accounts),
    };

    Ok(())
}
