#[macro_use]
extern crate log;

#[macro_use]
extern crate itertools;

#[macro_use]
extern crate rustc_serialize;
extern crate hyper;
extern crate getopts;
extern crate time;

mod frog_log;
mod protocol;
mod the_impl_ya_dummy;

mod cli {
    use getopts::Options;
    use std::env;
    use std::net::{SocketAddr,AddrParseError};
    use std::error;

    use super::protocol::{ExternalAddr,ParseExternalAddrError};


    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    #[derive(Clone, Debug)]
    pub struct Config {
        pub int_addr: SocketAddr,
        pub ext_addr: ExternalAddr,
        pub frog_tips_api_key: String,
    }

    enum Error<'a> {
        Usage(&'a str, Options),
        BadOpt(Box<error::Error>),
        MissingOpt(String),
        Version,
    }

    impl<'a> From<AddrParseError> for Error<'a> {
        fn from(err: AddrParseError) -> Error<'a> {
            Error::BadOpt(Box::new(err))
        }
    }

    impl<'a> From<ParseExternalAddrError> for Error<'a> {
        fn from(err: ParseExternalAddrError) -> Error<'a> {
            Error::BadOpt(Box::new(err))
        }
    }

    fn print_usage(program: &str, opts: Options) {
        let brief = format!("usage: {} ADDR [OPTIONS]", program);
        print!("{}", opts.usage(&brief));
    }

    fn parse<'a>(program: &'a str, args: &Vec<String>) -> Result<Config, Error<'a>> {
        let mut opts = Options::new();
        opts.optopt("x", "ext-addr", "EXTERNAL ADDRESS.", "EXT_ADDR");
        opts.optopt("k", "api-key", "YOUR FROG.TIPS API KEY.", "API_KEY");
        opts.optflag("h", "help", "SHOW THIS HELP THEN EXIT.");
        opts.optflag("v", "version", "SHOW THE CURRENT VERSION THEN EXIT.");

        let matches = match opts.parse(&args[1..]) {
            Ok(m) => { m }
            Err(f) => { panic!(f.to_string()) }
        };

        if matches.opt_present("h") {
            return Err(Error::Usage(program, opts));
        }

        if matches.opt_present("v") {
            return Err(Error::Version);
        }

        let addr: SocketAddr = if !matches.free.is_empty() {
            try!(matches.free[0].clone().parse())
        } else {
            return Err(Error::Usage(program, opts));
        };

        let api_key = try!(matches.opt_str("k").ok_or(Error::MissingOpt("API_KEY".to_string())));
        let ext_addr = {
            let opt = try!(matches.opt_str("x").ok_or(Error::MissingOpt("EXT_ADDR".to_string())));
            try!(opt.parse())
        };

        Ok(Config {
            int_addr: addr,
            ext_addr: ext_addr,
            frog_tips_api_key: api_key
        })
    }

    pub fn main<F: Fn(Config) -> ()>(success: F) {
        let args: Vec<String> = env::args().collect();
        let program = args[0].clone();

        match parse(&program, &args) {
            Ok(config) => success(config),
            Err(err) => match err {
                Error::Usage(program, opts) => print_usage(&program, opts),
                Error::BadOpt(err) => println!("ERROR: INVALID VALUE: '{}'.", err),
                Error::MissingOpt(name) => println!("ERROR: {} IS REQUIRED.", name),
                Error::Version => println!("{}, version {}", program, VERSION),
            },
        }
    }
}

use std::net::{TcpListener};
use std::thread;
use std::sync::{Arc,Mutex};
use std::time::Duration;


fn main() {
    cli::main(|config| {
        frog_log::init().unwrap();
        info!("FROG IS PREPARING TO PLAY WITH GOPHERS.");

        let listener = TcpListener::bind(config.int_addr).unwrap();
        let shared_gopher = Arc::new(
                                Mutex::new(
                                    the_impl_ya_dummy::Gopher::new(config.ext_addr, config.frog_tips_api_key)));

        info!("FROG IS NOW PLAYING WITH GOPHERS");
        for stream in listener.incoming() {
            match stream {
                Err(why) => {
                    error!("GOPHER FAILED TO POP OUT OF ITS BURROW: {}", why);
                },
                Ok(stream) => {
                    match stream.peer_addr() {
                        Ok(addr) => {
                            let my_thread_name = format!("GOPHER_{}", addr);
                            let my_gopher = shared_gopher.clone();

                            {
                                let just_a_wee_bit = Some(Duration::from_secs(60));
                                let _ = stream.set_read_timeout(just_a_wee_bit);
                                let _ = stream.set_write_timeout(just_a_wee_bit);
                            }

                            let _ = thread::Builder::new().name(my_thread_name)
                                                          .spawn(move || {
                                                              info!("A GOPHER HAS POPPED OUT OF ITS BURROW.");

                                                              let mut gopher = my_gopher.lock().unwrap();
                                                              match gopher.respond(stream) {
                                                                  Ok(_) => info!("A GOPHER HAS RETREATED INTO ITS BURROW ON GOOD TERMS. GOODBYE GOPHER."),
                                                                  Err(why) => error!("A GOPHER HAS RETREATED INTO ITS BURROW ON BAD TERMS: {}", why),
                                                              }
                            });
                        },
                        Err(why) => error!("GOPHER DOES NOT HAVE AN ADDRESS: {}", why)
                    };
                }
            }
        }
    });
}
