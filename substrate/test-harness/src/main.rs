use std::sync::mpsc::{channel, Receiver};

use clap::{load_yaml, App};
use codec::Decode;
use log::*;
use hex::ToHex;
use keyring::AccountKeyring;
use primitives::H256 as Hash;
use primitives::sr25519;
use rstd::prelude::*;
use primitives::sr25519::Pair;
use node_runtime::Event;
use indices::address::Address;
use substrate_api_client::{
    extrinsic::{contract, xt_primitives::GenericAddress},
    utils::*,
    Api,
};

// This is the minimal valid substrate contract.
const CONTRACT: &str = r#"
(module
    (func (export "call"))
    (func (export "deploy"))
)
"#;

/// This is a contract which returns a number when executed. When the contract
/// is called it should return the number 7.
const R_CONTRACT: &str = r#"
(module
    ;; Import the "ext_return" opcode from the environment
    (import "env" "ext_return" (func $ext_return (param i32 i32)))
    (import "env" "ext_println" (func $ext_println (param i32 i32)))
    ;; env.println
    (import "env" "memory" (memory 1 1))
    (func (export "call")
        (call $ext_println
            (i32.const 8) ;; The data buffer
            (i32.const 52) ;; The data buffer's length
        )
        ;; (call $ext_return
        ;;     (i32.const 8) ;; The data buffer
        ;;     (i32.const 2) ;; The data buffer's length
        ;; )
    )
    (func (export "deploy"))
    (data (i32.const 8) "This is the value we want to log, it is of length 52")
)
"#;


/// This is a contract which returns a number when executed. When the contract
/// is called it should return the number 7.
const Q_CONTRACT: &str = r#"
(module
    (import "env" "cap9_seven" (func $cap9_seven))
    (import "env" "ext_println" (func $ext_println (param i32 i32)))
    (import "env" "ext_scratch_size" (func $ext_scratch_size (result i32)))
    (import "env" "ext_scratch_read" (func $ext_scratch_read (param i32 i32 i32)))
    ;; env.println
    (import "env" "memory" (memory 1 1))
    (func (export "call")
        call $cap9_seven
        i32.const 1 ;; The pointer where to store the data.
        i32.const 0 ;; Offset from the start of the scratch buffer.
        i32.const 1 ;; Count of bytes to copy.
        call $ext_scratch_read
        (call $ext_println
            (i32.const 1) ;; The data buffer
            (i32.const 1) ;; The data buffer's length
        )
    )
    (func (export "deploy"))
    (data (i32.const 8) "This is the value we want to log, it is of length 52")
)
"#;

// This contract reads the caplist using the $cap_clist function. The scratch
// buffer is filled with the output from this function. We then copy the data
// from the scratch buffer into memory so we can work with it. Currently the
// data is an array[7] of bytes, either 0x00 (false) or 0x01 (true). These
// represent a crude for of capability in order:
//
//   0. ProcedureCall (Cap Index 3)
//   1. ProcedureRegister (Cap Index 4)
//   2. ProcedureDelete (Cap Index 5)
//   3. ProcedureEntry (Cap Index 6)
//   4. StoreWrite (Cap Index 7)
//   5. Log (Cap Index 8)
//   6. AccountCall (Cap Index 9)
//
// This example contract will iteratre through its own capabilities and print
// true or false.
const T_CONTRACT: &str = include_str!("t_contract.wat");

const BAD_CONTRACT: &str = r#"
(module
    ;; Import the "ext_return" opcode from the environment
    (import "env" "ext_return" (func $ext_return (param i32 i32)))
    (import "env" "ext_println" (func $ext_println (param i32 i32)))
    ;; env.println
    (import "env" "memory" (memory 1 1))
    (func $assert (param $test i32)
        (if
            (get_local $test)
            (then
                unreachable
            )
            (else

            )
        )
    )
    (func (export "call")
        i32.const 3
        i32.const 4
        i32.ne
        call $assert
    )
    (func (export "deploy"))
    (data (i32.const 8) "This is the value we want to log, it is of length 52")
)
"#;

/// This is a contract which executes a simple substrate opcode when executed.
const S_CONTRACT: &str = r#"
(module
    (import "env" "ext_scratch_size" (func $ext_scratch_size (result i32)))
    (import "env" "ext_scratch_read" (func $ext_scratch_read (param i32 i32 i32)))
    (import "env" "ext_set_storage" (func $ext_set_storage (param i32 i32 i32 i32)))
    (import "env" "ext_get_storage" (func $ext_get_storage (param i32) (result i32)))
    (import "env" "memory" (memory 1 1))
    (func (export "call")
        ;; Return value is 0 for success
        (call $ext_get_storage
            (i32.const 16) ;; Pointer to the key
        )
        (call $ext_scratch_read
            (i32.const 1) ;; The pointer where to store the data.
            (i32.const 0) ;; Offset from the start of the scratch buffer.
            (i32.const 1) ;; Count of bytes to copy.
        )
        (call $ext_set_storage
            (i32.const 1)
            (i32.const 1)
            (i32.const 0)
            (i32.const 4)
        )
    )
    (func (export "deploy"))
    ;; The value to store
    (data (i32.const 0) "\28")
)
"#;

const STORE_CONTRACT: &str = include_str!("store_contract.wat");
const CALLER_CONTRACT: &str = include_str!("caller_contract.wat");
const CALLEE_CONTRACT: &str = include_str!("callee_contract.wat");

fn deploy_contract(api: &substrate_api_client::Api<primitives::sr25519::Pair>, contract: &str, init_data: Vec<u8>, call_data: Vec<u8>) -> [u8; 32] {
    let wasm = wabt::wat2wasm(contract).expect("invalid wabt");

    // 1. Put the contract code as a wasm blob on the chain
    let xt = api.contract_put_code(500_000, wasm);
    println!(
        "[+] Putting contract code on chain with extrinsic:\n\n{:?}\n",
        xt
    );
    let tx_hash = api.send_extrinsic(xt.hex_encode()).unwrap();
    println!("[+] Transaction got finalized. Hash: {:?}\n", tx_hash);

    // setup the events listener for our chain.
    let (events_in, events_out) = channel();
    api.subscribe_events(events_in.clone());

    // wait for the `contract.CodeStored(code_hash)` event, which returns code hash that is needed
    // to define what contract shall be instantiated afterwards.
    println!("[+] Waiting for the contract.Stored event");
    let code_hash = subcribe_to_code_stored_event(&events_out);
    println!("[+] Event was received. Got code hash: {:?}\n", code_hash);

    // 2. Create an actual instance of the contract
    let xt = api.contract_create(10_000_000_000_000_000, 500_000, code_hash, init_data);

    println!(
        "[+] Creating a contract instance with extrinsic:\n\n{:?}\n",
        xt
    );
    println!(
        "[+] Creating a contract instance with extrinsic:\n\n{}\n",
        xt.hex_encode()
    );
    let tx_hash = api.send_extrinsic(xt.hex_encode()).unwrap();
    println!("[+] Transaction got finalized. Hash: {:?}\n", tx_hash);

    // Now if the contract has been instantiated successfully, the following events are fired:
    // - indices.NewAccountIndex, balances.NewAccount -> generic events when an account is created
    // - balances.Transfer(from, to, balance) -> Transfer from caller of contract.create/call to the contract account
    // - contract.Instantiated(from, deployedAt) -> successful deployment at address. We Want this one.
    // Note: Trying to instantiate the same contract with the same data twice does not work. No event is
    // fired correspondingly.
    println!("[+] Waiting for the contract.Instantiated event");
    // TODO: print as hex hash
    let deployed_at = subscribe_to_code_instantiated_event(&events_out);
    let address = match deployed_at {
        Address::Id(address) => address,
        _ => panic!("address not id"),
    };

    println!(
        "[+] Event was received. Contract deployed at (address of contract): 0x{}\n",
        // address.encode_hex::<String>()
        hex::encode(address)
    );

    // 3. Call the contract instance
    let xt = api.contract_call(deployed_at, 500_000, 500_000, call_data);

    // Currently, a contract call does not fire any events nor interact in any other fashion with
    // the outside world. Only node logs can supply information on the consequences of a contract
    // call. Still, it can be checked if the transaction was successful.
    println!(
        "[+] Calling the contract with extrinsic Extrinsic:\n{:?}\n\n",
        xt
    );
    let tx_hash: primitive_types::H256 = api.send_extrinsic(xt.hex_encode()).unwrap();
    println!("[+] Transaction got finalized. Hash: {:?}", tx_hash);
    // We can't get return values from contract calls. We want to get
    // information about this extrinsic, such as if it was successful.
    address
}

fn main() {
    env_logger::init();
    let url = get_node_url_from_cli();

    // initialize api and set the signer (sender) that is used to sign the extrinsics
    // let from = AccountKeyring::new("//Ferdie", Some(""), CryptoKind::Sr25519);
    let from = AccountKeyring::Ferdie.pair();

    println!("Got key");
    let api = Api::new(format!("ws://{}", url)).set_signer(from);
    println!("[+] Ferdie's Account Nonce is {}", api.get_nonce().unwrap());

    // deploy_contract(&api, CONTRACT, vec![1u8]);
    // deploy_contract(&api, R_CONTRACT, vec![1u8]);
    // deploy_contract(&api, BAD_CONTRACT, vec![1u8]);
    // deploy_contract(&api, Q_CONTRACT, vec![1u8]);
    // deploy_contract(&api, T_CONTRACT, vec![1u8]);
    // deploy_contract(&api, STORE_CONTRACT, vec![1u8]);
    let callee_address = deploy_contract(&api, CALLEE_CONTRACT, vec![], vec![]);
    deploy_contract(&api, CALLER_CONTRACT, vec![], callee_address.to_vec());
    get_storage(&api);
}

fn subcribe_to_code_stored_event(events_out: &Receiver<String>) -> Hash {
    loop {
        let event_str = events_out.recv().unwrap();

        let _unhex = hexstr_to_vec(event_str).unwrap();
        let mut _er_enc = _unhex.as_slice();
        let _events = Vec::<system::EventRecord<Event, Hash>>::decode(&mut _er_enc);
        if let Ok(evts) = _events {
            for evr in &evts {
                debug!("decoded: phase {:?} event {:?}", evr.phase, evr.event);
                if let Event::contracts(ce) = &evr.event {
                    if let contracts::RawEvent::CodeStored(code_hash) = &ce {
                        info!("Received Contract.CodeStored event");
                        info!("Codehash: {:?}", code_hash);
                        return code_hash.to_owned();
                    }
                }
            }
        }
    }
}

fn subscribe_to_code_instantiated_event(events_out: &Receiver<String>) -> GenericAddress {
    loop {
        let event_str = events_out.recv().unwrap();
        let _unhex = hexstr_to_vec(event_str).unwrap();
        let mut _er_enc = _unhex.as_slice();
        let _events = Vec::<system::EventRecord<Event, Hash>>::decode(&mut _er_enc);
        if let Ok(evts) = _events {
            for evr in &evts {
                debug!("decoded: phase {:?} event {:?}", evr.phase, evr.event);
                if let Event::contracts(ce) = &evr.event {
                    if let contracts::RawEvent::Instantiated(from, deployed_at) = &ce {
                        info!("Received Contract.Instantiated Event");
                        info!("From: {:?}", from);
                        info!("Deployed at: {:?}", deployed_at);
                        return GenericAddress::from(deployed_at.to_owned().0);
                    }
                }
            }
        }
    }
}

pub fn get_node_url_from_cli() -> String {
    // let yml = load_yaml!("../../src/examples/cli.yml");
    // let matches = App::from_yaml(yml).get_matches();

    // let node_ip = matches.value_of("node-server").unwrap_or("127.0.0.1");
    // let node_port = matches.value_of("node-port").unwrap_or("9944");
    let node_ip = "127.0.0.1";
    let node_port = "9944";
    let url = format!("{}:{}", node_ip, node_port);
    println!("Interacting with node on {}", url);
    url
}


fn get_storage(api: &substrate_api_client::Api<primitives::sr25519::Pair>) {
    // get some plain storage value
    let result_str = api.get_storage("Cap9", "SpecialValue", None).unwrap();
    let result = hexstr_to_u256(result_str).unwrap();
    println!("[+] SpecialValue is {}", result);

    // // get Alice's AccountNonce
    // let accountid = AccountId::from(AccountKeyring::Alice);
    // let result_str = api
    //     .get_storage("System", "AccountNonce", Some(accountid.encode()))
    //     .unwrap();
    // let result = hexstr_to_u256(result_str).unwrap();
    // println!("[+] Ferdie's Account Nonce is {}", result.low_u32());

    // // get Ferdie's AccountNonce with the AccountKey
    // let signer = AccountKeyring::Ferdie.pair();
    // let result_str = api
    //     .get_storage("System", "AccountNonce", Some(signer.public().encode()))
    //     .unwrap();
    // let result = hexstr_to_u256(result_str).unwrap();
    // println!("[+] Ferdie's Account Nonce is {}", result.low_u32());

    // // get Ferdie's AccountNonce with api.get_nonce()
    // api.signer = Some(signer);
    // println!("[+] Ferdie's Account Nonce is {}", api.get_nonce().unwrap());
}
