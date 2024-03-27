# XXX v1.0.0

## Introduction

Welcome to XXX v1.0.0

## Getting Started

Firstly, type your wallet address and private key into `credit.json` file.

```json
{
    "wallet_address" : "your_wallet_address",
    "private_key" : "your_private_key"
}
```

You should build the project.

```bash
cargo build
```
    
For the Release build, run the code.

```bash
cargo build --release
```

Please copy `credit.json` and `data.json` files and paste to `/target/release/` folder.

## Running

Debug mode:

```bash
cargo run
```

Release mode:

```bash
cd target/release
./xxx
```
