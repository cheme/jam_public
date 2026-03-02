//! client exposed operations.
//! - opening state from file and others.
//! - produce refinement payload from json.

use std::env;
use std::io::Read;
use codec::Encode;

const HELP: &str = {
    "Build a refinement payload: 
		input_json_file_path output_payload_file_path
		balance.db and "
};

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(args.clone());
    if args.len() != 3 || &args[1] == "--help" {
        println!("{}", HELP);
        return;
    }

    let mut input = std::fs::File::open(&args[1]).unwrap();
    let mut input_vec = Vec::new();
    input.read_to_end(&mut input_vec).unwrap();
    let operations = token_ledger::json::parse_signed_operations(input_vec.as_slice()).unwrap();
    dbg!(operations.len());
    let mut output = std::fs::File::create(&args[2]).unwrap();
    let mut opt_db = std::fs::OpenOptions::new();
    opt_db.read(true).write(true);
    let mut state = if let Ok(balances) = opt_db.open("balances.db") {
        let tokens = opt_db.open("tokens.db").unwrap();
        token_ledger_external_state::external_client::state::State::from_files(balances, tokens)
    } else {
        let balances_file = std::fs::File::create_new("balances.db").unwrap();
        let tokens_file = std::fs::File::create_new("tokens.db").unwrap();
        let mut state = token_ledger_external_state::external_client::state::State::default();
        state.set_new_persist_files(balances_file, tokens_file);
        state
    };
		dbg!(state.get_root());
    token_ledger_external_state::external_client::state_transition(&mut state, &operations);
		dbg!(state.get_root());
		let witness = state.take_witness();

    let refine_payload = token_ledger_external_state::RefinePayload {
        operations,
        witness,
    };
		refine_payload.encode_to(&mut output);
}
