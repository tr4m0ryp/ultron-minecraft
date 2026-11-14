use azalea::{prelude::*, ClientBuilder};
use motor_azalea::{handle, State};

const USAGE: &str = "usage: motor-azalea <host:port> <username> [companion-username]";

#[tokio::main]
async fn main() -> AppExit {
    let mut args = std::env::args().skip(1);
    let (Some(addr), Some(username)) = (args.next(), args.next()) else {
        eprintln!("{USAGE}");
        return AppExit::error();
    };
    let companion = args.next();

    ClientBuilder::new()
        .set_handler(handle)
        .set_state(State::new(companion))
        .start(Account::offline(&username), addr)
        .await
}
