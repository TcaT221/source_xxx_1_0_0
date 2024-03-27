use std::io;
use std::fs;
use std::io::Write;
use serde_derive::{Deserialize, Serialize};
use std::env;

use jupiter_swap_api_client::{
    quote::QuoteRequest, swap::SwapRequest, transaction_config::TransactionConfig,
    JupiterSwapApiClient,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey, transaction::VersionedTransaction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use tokio;

const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

#[derive(Deserialize, Serialize, Debug)]
struct TestData {
    pairs: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResponseType {
    pair: Pair,
}

#[derive(Deserialize, Serialize, Debug)]
struct Pair {
    base_token: BaseToken
}

#[derive(Deserialize, Serialize, Debug)]  
struct BaseToken {
    address: String
}

#[derive(Deserialize, Serialize, Debug)]  
struct CreditData {
    wallet_address: String,
    private_key: String
}

async fn get(pair_address: &String) -> Result<String, reqwest::Error> {
    // Make a GET request to the DexScreener API 
    // to fetch token data for the given pair address
    let url = format!("https://api.dexscreener.com/latest/dex/pairs/solana/{}", pair_address);
  
    let res = reqwest::get(url).await?;
    let response: serde_json::Value = res.json().await?;
    let token = response.get("pair").unwrap().get("baseToken").unwrap().get("address").unwrap();
    let token = token.as_str().unwrap();

    Ok(token.to_string())
}

// Function to swap tokens on the Solana blockchain
async fn swap(wallet_address:Pubkey, private_key:&String, buy_address:Pubkey, sell_address:Pubkey, amount:u64, fee:u64) {
    // Print out the buy and sell token addresses
    println!("Buy: {:#?}, Sell: {:#?}", buy_address, sell_address);
    // Get the API base URL from the environment 
    let api_base_url: String = env::var("API_BASE_URL").unwrap_or("https://quote-api.jup.ag/v6".into());
    // Initialize the client to call the Jupiter Swap API 
    let jupiter_swap_api_client: JupiterSwapApiClient = JupiterSwapApiClient::new(api_base_url);
    // Build the quote request 
    let quote_request = QuoteRequest {
        amount: amount,
        input_mint: sell_address,
        output_mint: buy_address,
        slippage_bps: 50,
        ..QuoteRequest::default()
    };
    // Call the /quote endpoint to get a quote
    let quote_response = jupiter_swap_api_client.quote(&quote_request).await.unwrap();
    
    print!(" Making Transaction...");
    io::stdout().flush().unwrap();

    // Call the /swap endpoint to initiate the transaction 
    let mut config = TransactionConfig::default();
    config.compute_unit_price_micro_lamports = Some(jupiter_swap_api_client::transaction_config::ComputeUnitPriceMicroLamports::MicroLamports(fee));
    let swap_response = jupiter_swap_api_client
        .swap(&SwapRequest {
            user_public_key: wallet_address,
            quote_response: quote_response.clone(),
            config: config
        })
        .await
        .unwrap();
    println!(" Done!");
    
    print!(" Signning...");
    io::stdout().flush().unwrap();

    // Sign and send the transaction
    let mut versioned_transaction: VersionedTransaction =
        bincode::deserialize(&swap_response.swap_transaction).unwrap();
    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".into());

    //Get the latest blockhash with rpc client
    let latest_blockhash = rpc_client
        .get_latest_blockhash()
        .await
        .unwrap();

    //Set recent_blockhash to the latest_blockhash obtained
    versioned_transaction.message.set_recent_blockhash(latest_blockhash);

    // Create a Keypair from the private key bytes
    let keypair = Keypair::from_base58_string(&private_key);
    let signed_versioned_transaction =
    VersionedTransaction::try_new(versioned_transaction.message, &[&keypair]).unwrap();
    println!(" Done!");
    
    // This will fail with "Transaction signature verification failure" as we did not really sign
    print!(" Sending and Confirming Transaction...");
    io::stdout().flush().unwrap();
    let transaction_result = rpc_client
        .send_and_confirm_transaction(&signed_versioned_transaction)
        .await
        .unwrap();
    println!(" Done!");
    println!("https://solscan.io/tx/{}", transaction_result);
    println!("");

    // POST /swap-instructions
    // let swap_instructions = jupiter_swap_api_client
    //     .swap_instructions(&SwapRequest {
    //         user_public_key: TEST_WALLET,
    //         quote_response,
    //         config: TransactionConfig::default(),
    //     })
    //     .await
    //     .unwrap();
    // println!("swap_instructions: {swap_instructions:?}");

}

#[tokio::main]
// Main function that runs the token swap program
async fn main() -> Result<(), std::io::Error> {

    print!("Loading, Please wait...");
    io::stdout().flush().unwrap();

    // Read test data from JSON file 
    let input_path: String = String::from("data.json");
    let test_data: TestData = {
        let test_data = fs::read_to_string(&input_path)?;
        serde_json::from_str(&test_data).unwrap()
    };

    // Read wallet credentials from JSON file 
    let input_path: String = String::from("credit.json");
    let credit_data: CreditData = {
        let credit_data = fs::read_to_string(&input_path)?;
        serde_json::from_str(&credit_data).unwrap()
    };

    // Get wallet address and private key 
    let wallet_address:Pubkey = pubkey!(credit_data.wallet_address).parse().unwrap();
    let private_key = credit_data.private_key;

    // Get quote addresses for token pairs 
    let mut quote_addresses:Vec<String> = Vec::new();

    // Loop through pairs and get quote address
    for indxe in 0..test_data.pairs.len() {
        let quote_address = get(&test_data.pairs[indxe]).await.unwrap();
        quote_addresses.push(quote_address);
    }

    println!(" done!");
    
    println!("Welocome to XXX v1.0.0 !");

    // Print out token pairs
    for indxe in 0..test_data.pairs.len() {
        println!("{:#?}", test_data.pairs[indxe]);
    }

    print!("How much prioritization fee do you want? Auto(a)(0.0005) or Custom(type any number)?: ");
    io::stdout().flush().unwrap();

    // Read user input
    let mut fee_input = String::new();
    io::stdin()
    .read_line(&mut fee_input)
    .expect("Failed to read line");
    let fee_input:String = fee_input.trim().parse().expect("Please type a correct info!");

    let fee: u64;
    if fee_input == "A" || fee_input == "a" {
        fee = (5000000 - 50000) / 14;
    }
    else {
        let fee_input = fee_input.parse::<f64>().unwrap();
        let fee_input = (fee_input * 10000000000.0) as u64;
        fee = (fee_input - 50000) / 14;
    }
    println!("Prioritization fee: {:#?} SOL", fee);

    print!("Do you want to Buy(b) or Sell(s)?: ");
    io::stdout().flush().unwrap();

    // Read user input
    let mut buy_or_sell = String::new();
    io::stdin()
        .read_line(&mut buy_or_sell)
        .expect("Failed to read line");
    let buy_or_sell:String = buy_or_sell.trim().parse().expect("Please type a correct info!");

    // If user selects buy
    if buy_or_sell == "B" || buy_or_sell == "b" {
        
        print!("Please enter estimate amount in forms of SOL: ");
        io::stdout().flush().unwrap();

        // Loop until valid amount confirmed
        loop {
            // Get amount input
            let mut amount = String::new();
            io::stdin()
                .read_line(&mut amount)
                .expect("Failed to read line");
            let amount:f64 = amount.trim().parse().expect("Please type a number!");
            
            print!("You entered: {} !  Yes(y) or Not(n)?: ", amount);
            io::stdout().flush().unwrap();

            // Get confirmation
            let mut confirm = String::new();
            io::stdin()
                .read_line(&mut confirm)
                .expect("Failed to read line");
            let confirm:String = confirm.trim().parse().expect("Please type a correct info!");

            // Run swap if confirmed
            if confirm == "y" || confirm == "Y" {
                println!("Congratulations! Amount is entered as {} SOL\n", amount);
                let amount:f64 = amount * 1000000000.0;
                let amount = amount as u64;
                for indxe in 0..quote_addresses.len() {
                    let buy_address: Pubkey = pubkey!(&quote_addresses[indxe]).parse().unwrap();
                    swap(wallet_address, &private_key, buy_address, NATIVE_MINT, amount, fee).await;
                }
                break;
            }
            // Reprompt for amount if not confirmed
            else {
                print!("Please enter estimate amount in forms of SOL: ");
                io::stdout().flush().unwrap();
            }

        }
    }
    else {

        print!("How much do you want to sell: 50%(a) or 100%(b)?: ");
        io::stdout().flush().unwrap();
        
        loop {
            // Read user input
            let mut percent = String::new();
            io::stdin()
                .read_line(&mut percent)
                .expect("Failed to read line");
            let percent: String = percent.trim().parse().expect("Please type a number!");
            // If typed "a" percent = 50, if typed "b" percent = 100
            let percent = match percent.as_str() {
                "a" => 50,
                "b" => 100,
                _ => panic!("Invalid percent value!"), // This will panic if the percent value is not 1 or 0.
            };

            // Confirm sale percentage with user
            print!("Do you really want to sell {:#?}% of all the tokens? Yes(y) or Not(n)?: ", percent);
            io::stdout().flush().unwrap();

            // Read user input
            let mut confirm = String::new();
            io::stdin()
                .read_line(&mut confirm)
                .expect("Failed to read line");
            let confirm:String = confirm.trim().parse().expect("Please type a correct info!");

            // Run swap if confirmed
            if confirm == "y" || confirm == "Y" {
                println!("");
                for indxe in 0..quote_addresses.len() {
            
                    let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".into());
                    // Get associated token address of array to swap
                    let mint_id:Pubkey = pubkey!(&quote_addresses[indxe]).parse().unwrap();  
                    let addr = spl_associated_token_account::get_associated_token_address(&wallet_address, &mint_id);
                    // Get token balance
                    let balance = rpc_client.get_token_account_balance(&addr).await.unwrap();
                    // Set amount to swap based on token balance info and percentage
                    let amount:u64 = balance.amount.trim().parse().unwrap();
                    let amount:u64 = amount * percent / 100;
                    // Run swap
                    let sell_address: Pubkey = pubkey!(&quote_addresses[indxe]).parse().unwrap();
                    swap(wallet_address, &private_key, NATIVE_MINT, sell_address, amount, fee).await;
                }
                break;
            }
            // Reprompt for percentage if not confirmed
            else {
                print!("How much do you want to sell: 50%(a) or 100%(b)?: ");
                io::stdout().flush().unwrap();
            }

        }

    }

    println!("All the transactions successfully done!");

    Ok(())

}