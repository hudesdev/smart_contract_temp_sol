use borsh::{BorshDeserialize, BorshSerialize};
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{
    account::Account,
    instruction::{AccountMeta, Instruction},
    message::Message,
    pubkey,
    pubkey::Pubkey,
    signature::Keypair,
    signer::{keypair::read_keypair_file, Signer},
    transaction::Transaction,
};
use std::{borrow::Borrow, str::FromStr};

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

fn get_prgram_accounts(
    rpc_client: &RpcClient,
    program_id: &Pubkey,
    memcmps: Vec<RpcFilterType>,
) -> Vec<(Pubkey, Account)> {
    let config = RpcProgramAccountsConfig {
        filters: Some(memcmps),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
        },
        with_context: Some(false),
    };

    rpc_client
        .get_program_accounts_with_config(&program_id, config)
        .unwrap()
}

fn buy(rpc_client: &RpcClient, usr: &Keypair, program_id: &Pubkey, args: &Vec<String>) {
    let store_pubkey = Pubkey::from_str(&args[2]).unwrap();
    let product = String::from(&args[3]);
    let (product_pubkey, _) =
        Pubkey::find_program_address(&[&store_pubkey.to_bytes(), product.as_bytes()], program_id);

    let store_account = rpc_client.get_account(&store_pubkey).unwrap();

    let store = Store::deserialize(&mut store_account.data.as_ref()).unwrap();
    let store_owner = store.owner;

    let command = Command::Buy;
    let instruction = Instruction::new_with_borsh(
        *program_id,
        &command,
        vec![
            AccountMeta::new(usr.pubkey(), true),
            AccountMeta::new(product_pubkey, false),
            AccountMeta::new_readonly(store_pubkey, false),
            AccountMeta::new(store_owner, false),
            AccountMeta::new_readonly(pubkey!("11111111111111111111111111111111"), false),
        ],
    );

    let message = Message::new(&[instruction], None);
    let tx = Transaction::new(&[usr], message, rpc_client.get_latest_blockhash().unwrap());
    send(rpc_client, &tx);
    println!("You bought {}", product);
}

fn get_store(rpc_client: &RpcClient, args: &Vec<String>) {
    let store_pubkey = Pubkey::from_str(&args[2]).unwrap();

    let store_account = rpc_client.get_account(&store_pubkey).unwrap();

    let store = Store::deserialize(&mut store_account.data.as_ref()).unwrap();
    let program_id = store_account.owner;

    println!(
        "Name of store: {}\nOwner: {}\n",
        store.name_of_store, store.owner
    );

    let memcmp = RpcFilterType::Memcmp(Memcmp {
        offset: 0,
        bytes: MemcmpEncodedBytes::Base64(store_pubkey.to_string()),
        encoding: None,
    });

    let accounts = get_prgram_accounts(rpc_client, &program_id, vec![memcmp]);

    println!("Products:");
    for (_, account) in accounts.iter() {
        let product = Product::deserialize(&mut account.data.borrow()).unwrap();
        println!("{}: {} SOL", product.name, product.price);
    }
}

fn get_product(rpc_client: &RpcClient, args: &Vec<String>) {
    let product_pubkey = Pubkey::from_str(&args[2]).unwrap();

    let product_account = rpc_client.get_account(&product_pubkey).unwrap();

    let product = Product::deserialize(&mut product_account.data.as_ref()).unwrap();

    println!("Product: {}: {} SOL", product.name, product.price);
}

fn make_store(rpc_client: &RpcClient, usr: &Keypair, program_id: &Pubkey, args: &Vec<String>) {
    let name_of_store = String::from(&args[2]);
    assert!(name_of_store.len() <= 50);
    let store = Store {
        name_of_store,
        owner: usr.pubkey(),
    };

    let rent_exemption = rpc_client
        .get_minimum_balance_for_rent_exemption(50 + 32)
        .unwrap();

    let (account_pubkey, _) =
        Pubkey::find_program_address(&[store.name_of_store.as_bytes()], program_id);
    let command = Command::MakeStore(store, rent_exemption);

    let instruction = Instruction::new_with_borsh(
        *program_id,
        &command,
        vec![
            AccountMeta::new(account_pubkey, false),
            AccountMeta::new(usr.pubkey(), true),
            AccountMeta::new_readonly(pubkey!("11111111111111111111111111111111"), false),
        ],
    );

    let message = Message::new(&[instruction], None);
    let tx = Transaction::new(&[usr], message, rpc_client.get_latest_blockhash().unwrap());
    send(&rpc_client, &tx);
    println!("Store pubkey: {}", account_pubkey);
}

fn add_to_store(rpc_client: &RpcClient, usr: &Keypair, program_id: &Pubkey, args: &Vec<String>) {
    let name_of_store = String::from(&args[2]);
    let name_of_product = String::from(&args[3]);
    assert!(name_of_product.len() <= 50);
    let (store_pubkey, _) = Pubkey::find_program_address(&[name_of_store.as_bytes()], &program_id);
    let (account_pubkey, _) = Pubkey::find_program_address(
        &[&store_pubkey.to_bytes(), name_of_product.as_bytes()],
        program_id,
    );

    let product = Product {
        owner: store_pubkey,
        store: store_pubkey,
        name: name_of_product,
        price: args[4].parse().unwrap(),
    };

    let rent_exemption = rpc_client
        .get_minimum_balance_for_rent_exemption(32 + 32 + 50 + 8 + 8)
        .unwrap();

    let command = Command::AddToStore(product, rent_exemption);

    let instruction = Instruction::new_with_borsh(
        *program_id,
        &command,
        vec![
            AccountMeta::new(account_pubkey, false),
            AccountMeta::new_readonly(store_pubkey, false),
            AccountMeta::new(usr.pubkey(), true),
            AccountMeta::new_readonly(pubkey!("11111111111111111111111111111111"), false),
        ],
    );

    let message = Message::new(&[instruction], None);
    let tx = Transaction::new(&[usr], message, rpc_client.get_latest_blockhash().unwrap());
    send(&rpc_client, &tx);
    println!("Successfully added: {}", account_pubkey);
}

fn delete_from_store(
    rpc_client: &RpcClient,
    usr: &Keypair,
    program_id: &Pubkey,
    args: &Vec<String>,
) {
    let name_of_store = String::from(&args[2]);
    let name_of_product = String::from(&args[3]);
    let (store_pubkey, _) = Pubkey::find_program_address(&[name_of_store.as_bytes()], program_id);
    let (product_pubkey, _) = Pubkey::find_program_address(
        &[&store_pubkey.to_bytes(), name_of_product.as_bytes()],
        program_id,
    );

    let command = Command::DeleteFromStore;
    let instruction = Instruction::new_with_borsh(
        *program_id,
        &command,
        vec![
            AccountMeta::new(product_pubkey, false),
            AccountMeta::new(store_pubkey, false),
            AccountMeta::new(usr.pubkey(), true),
        ],
    );

    let message = Message::new(&[instruction], None);
    let tx = Transaction::new(&[usr], message, rpc_client.get_latest_blockhash().unwrap());
    send(&rpc_client, &tx);
    println!("Successfully deleted: {}", product_pubkey);
}

fn close(rpc_client: &RpcClient, usr: &Keypair, program_id: &Pubkey, args: &Vec<String>) {
    let store_pubkey = Pubkey::from_str(&args[2]).unwrap();
    let command = Command::Close;

    let memcmp_owner = RpcFilterType::Memcmp(Memcmp {
        offset: 0,
        bytes: MemcmpEncodedBytes::Base64(store_pubkey.to_string()),
        encoding: None,
    });

    let accounts = get_prgram_accounts(rpc_client, program_id, vec![memcmp_owner]);

    for (product_pubkey, _) in accounts.iter() {
        let command = Command::DeleteFromStore;
        let instruction = Instruction::new_with_borsh(
            *program_id,
            &command,
            vec![
                AccountMeta::new(*product_pubkey, false),
                AccountMeta::new(store_pubkey, false),
                AccountMeta::new(usr.pubkey(), true),
            ],
        );

        let message = Message::new(&[instruction], None);
        let tx = Transaction::new(&[usr], message, rpc_client.get_latest_blockhash().unwrap());
        send(&rpc_client, &tx);
    }

    let instruction = Instruction::new_with_borsh(
        *program_id,
        &command,
        vec![
            AccountMeta::new(store_pubkey, false),
            AccountMeta::new(usr.pubkey(), true),
        ],
    );

    let message = Message::new(&[instruction], None);
    let tx = Transaction::new(&[usr], message, rpc_client.get_latest_blockhash().unwrap());
    send(&rpc_client, &tx);
    println!("Successfully deleted: {}", store_pubkey);
}

fn send(rpc_client: &RpcClient, tx: &Transaction) {
    let _result = rpc_client.send_and_confirm_transaction(&tx).unwrap();
    // println!("{:#?}", rpc_client.get_transaction(&_result, UiTransactionEncoding::Json).unwrap());
}

fn wipe(rpc_client: &RpcClient, program_id: &Pubkey, usr: &Keypair) {
    let accounts = get_prgram_accounts(rpc_client, program_id, vec![]);

    for (account_pubkey, _) in accounts.iter() {
        let command = Command::Wipe;
        let instruction = Instruction::new_with_borsh(
            *program_id,
            &command,
            vec![
                AccountMeta::new(*account_pubkey, false),
                AccountMeta::new(usr.pubkey(), true),
            ],
        );

        let message = Message::new(&[instruction], None);
        let tx = Transaction::new(&[usr], message, rpc_client.get_latest_blockhash().unwrap());
        send(&rpc_client, &tx);
    }
    println!("Wiped");
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <command>", args[0]);
        std::process::exit(-1);
    }

    let arg = &args[1];

    let rpc_client = RpcClient::new("https://api.devnet.solana.com");
    let usr = read_keypair_file("/home/gohnny/.config/solana/dev.json").unwrap();
    let program = pubkey!("74yFYZY2m29ViLuFnVkmMAmhRmjKgREVcbqhtSTG3bJY");

    match arg.as_str() {
        "get-store" => get_store(&rpc_client, &args),
        "get-product" => get_product(&rpc_client, &args),
        "make-store" => make_store(&rpc_client, &usr, &program, &args),
        "add-to-store" => add_to_store(&rpc_client, &usr, &program, &args),
        "delete-from-store" => delete_from_store(&rpc_client, &usr, &program, &args),
        "buy" => buy(&rpc_client, &usr, &program, &args),
        "close" => close(&rpc_client, &usr, &program, &args),
        "wipe" => wipe(&rpc_client, &program, &usr),
        _ => println!("Wrong command"),
    }
}
