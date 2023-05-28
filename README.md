# Solana-SmartContract-Store
This little repo allows you to create your own store on Solana blockchain and interact with other stores. All code was written without Anchor framework.

# On-chain program (back)
First of all, you need to compile program and deploy it to Solana blockchain.
Use `cargo build-bpf` to build .so file, and then deploy it with `solana program deploy`.

# Client (front)
So, now you need to change some configuration in `main.rs: main(){..}` and build client to interact with smartcontract.<br>
Here are all supported commands: 

  * `get-store <Store pubkey>` - get info about store
  * `get-product <Product pubkey>` - get info about product
  * `make-store <Store name>` - make store
  * `add-to-store <Store name> <Product name> <Price in lamports>` - add to store some product
  * `delete-from-store <Store name> <Product name>` - delete product from store
  * `buy <Store pubkey> <Product name>` - buy some product
  * `close <Store name>` - close the shop
  * `wipe` - wipe all data (delele all shops and products)
    
