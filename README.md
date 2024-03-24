# Snapcase

This repository contains the source code for our demo submission _"Snapcase - Regain Control over Your Predictions with Low-Latency Machine Unlearning"_ to VLDB'24.


## Running the demonstration yourself

  1. Make sure you have a recent version of [Rust](https://www.rust-lang.org/tools/install) installed.
  2. Clone this repository locally via `git clone https://github.com/amsterdata/snapcase-demo` and change into the `snapcase-demo` folder
  3. Download the prebuilt top-k index and the purchase database [Google Drive](https://drive.google.com/drive/folders/1JCpR5RIfgmtUaxTMzkdVjfSODx41t3FF?usp=sharing). The `__instacart-index.bin` file 
must be placed directly in the `snapcase-demo` folder, and the `*.parquet` files must be placed in the `datasets/instacart/` subfolder.
  4. Start the demo with the following command: `cargo run --release --bin service`
  5. You should see some console output from DuckDB and Differential Dataflow, after which the demo will be served at http://localhost:8080 , which you can open in a browser

