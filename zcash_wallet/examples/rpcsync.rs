extern crate hex;
extern crate pirate;
extern crate zcash_wallet;

use std::env;
use zcash_wallet::chain::{sync::RpcChainSync, ChainSync};

// Testnet blocks with Sapling data:
// - 282635
// - 289461
// - 290807
// - 294035
fn main() {
    let options = vec![
        "H/host#IP that zcashd is listening on; default=127.0.0.1",
        "p/port#Port that zcashd's JSON-RPC interface is on; default=18232",
        "s/start#Block height to synchronise from; default=289460",
        "e/end#Block height to synchronise to; default=289465",
        ":/user#Value of rpcuser in zcash.conf",
        ":/password#Value of rpcpassword in zcash.conf",
    ];

    let mut vars = match pirate::vars("rpcsync", &options) {
        Ok(v) => v,
        Err(why) => panic!("Error: {}", why),
    };

    let args: Vec<String> = env::args().collect();
    let matches = match pirate::matches(&args, &mut vars) {
        Ok(m) => m,
        Err(why) => {
            println!("Error: {}", why);
            pirate::usage(&vars);
            return;
        }
    };

    let host = match matches.get("host") {
        Some(host) => host,
        None => "127.0.0.1",
    };
    let port = match matches.get("port") {
        Some(p) => p.parse::<u32>().unwrap(),
        None => 18232,
    };
    let start = match matches.get("start") {
        Some(s) => s.parse::<u32>().unwrap(),
        None => 289460,
    };
    let end = match matches.get("end") {
        Some(e) => e.parse::<u32>().unwrap(),
        None => 289465,
    };
    let user = matches.get("user").map(|s| s.clone());
    let password = matches.get("password").map(|s| s.clone());

    let server = format!("http://{}:{}", host, port);
    let cs = RpcChainSync::new(server, user, password);
    let mut session = match cs.start_session(start) {
        Ok((session, _)) => session,
        Err(e) => panic!("Failed to start session: {}", e),
    };

    while let Some(block) = session.next() {
        let block = block.unwrap();
        println!("Block {}: {}", block.height, block.hash);
        for tx in block.txs {
            println!("  txid: {}", tx.txid);
            if !tx.shielded_spends.is_empty() {
                println!("    Shielded Spends:");
                for nf in tx.shielded_spends {
                    println!("      nf: {}", nf)
                }
            }
            if !tx.shielded_outputs.is_empty() {
                println!("    Shielded Outputs:");
                for (cmu, epk, enc_ct) in tx.shielded_outputs {
                    let mut epk_data = Vec::with_capacity(32);
                    epk.write(&mut epk_data)
                        .expect("Should be able to write to a Vec<u8>");
                    epk_data.reverse();
                    println!("      cmu: {}", cmu);
                    println!("      epk: {}", hex::encode(epk_data));
                    println!("      enc_ct:");
                    println!("        {}...", enc_ct);
                }
            }
        }

        if block.height >= end {
            break;
        }
    }
}
