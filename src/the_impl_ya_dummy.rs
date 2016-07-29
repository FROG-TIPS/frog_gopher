
mod text_source {
    use protocol::{MenuItem,Path,Selected};
    use super::menu::{Source,MenuItemIter};


    pub struct TextSource {
        path: Path,
        text: String,
        desc: String,
    }

    impl TextSource {
        pub fn new<S: Into<String>>(path: Path, desc: S, text: S) -> TextSource {
            TextSource {
                path: path.clone(),
                text: text.into(),
                desc: desc.into(),
            }
        }
    }

    impl Source for TextSource {
        fn find(&self, path: &Path) -> Option<Selected> {
            if self.path == *path {
                Some(Selected::Text(Box::new(self.text.clone())))
            } else {
                None
            }
        }

        fn menu_items(&self) -> MenuItemIter {
            MenuItemIter::new(vec![MenuItem::Text {
                path: self.path.clone(),
                desc: self.desc.clone(),
            }])
        }
    }
}

mod tip_source {
    use hyper;
    use rustc_serialize::json;

    use std::io::Read;
    use std::io;

    use protocol::{MenuItem,Path,Selected};
    use super::menu::{MenuItemIter,Source};


    const ROOT_PATH: &'static str = "/TIP/";

    type TipNum = u64;

    fn tip_num_from_path(path: &Path) -> Option<TipNum> {
        let string = path.to_str();
        if string.starts_with(ROOT_PATH) {
            match string.split("/").last() {
                Some(num) => num.parse::<TipNum>().ok(),
                None => None,
            }
        } else {
            None
        }
    }

    #[allow(dead_code)]
    #[derive(RustcDecodable)]
    struct Tip {
        approved: bool,
        moderated: bool,
        tweeted: u64,
        number: TipNum,
        tip: String,
    }

    #[derive(RustcDecodable)]
    struct SearchResults {
        results: Vec<Tip>,
    }

    pub struct TipSource {
        api_key: String,
        client: hyper::Client,
    }

    impl TipSource {
        pub fn new(api_key: String) -> TipSource {
            TipSource {
                api_key: api_key,
                client: hyper::Client::new(),
            }
        }

        fn one_tip(&self, number: TipNum) -> Result<Option<Tip>, TipError> {
            let url = format!("https://frog.tips/api/2/tips/{}", number);
            let mut resp = try!(
                self.client.get(&url)
                           .header(hyper::header::Authorization(self.api_key.clone()))
                           .header(hyper::header::Connection::close())
                           .send());

           match resp.status {
               hyper::Ok => {
                   let mut body = String::new();
                   try!(resp.read_to_string(&mut body));
                   let tip: Tip = try!(json::decode(&body));
                   Ok(Some(tip))
               },
               other => {
                   info!("{:?}", other);
                   Ok(None)
               }
           }
        }

        fn all_tips(&self) -> Result<Vec<Tip>, TipError> {
            let mut resp = try!(
                self.client.post("https://frog.tips/api/2/tips/search")
                           .body("{\"tweeted\": true}")
                           .header(hyper::header::Authorization(self.api_key.clone()))
                           .header(hyper::header::Connection::close())
                           .send());

            match resp.status {
                hyper::Ok => {
                    let mut body = String::new();
                    try!(resp.read_to_string(&mut body));
                    let results: SearchResults = try!(json::decode(&body));
                    Ok(results.results)
                },
                _ => {
                    Ok(vec![])
                }
            }
        }
    }

    impl Source for TipSource {
        fn find(&self, path: &Path) -> Option<Selected> {
            tip_num_from_path(path).and_then(|num| {
                match self.one_tip(num) {
                    Ok(Some(tip)) => {
                        Some(Selected::Text(Box::new(tip.tip)))
                    },
                    Ok(None) => {
                        None
                    }
                    Err(why) => {
                        warn!("ERROR: {:?}", why);
                        None
                    }
                }
            })
        }

        fn menu_items(&self) -> MenuItemIter {
            let vec = match self.all_tips() {
                Ok(tips) => {
                    tips.into_iter()
                        .rev()
                        .map(|t| {
                            MenuItem::Text {
                                path: Path::from(format!("{}{}", ROOT_PATH, t.number)),
                                desc: format!("TIP #{}", t.number),
                            }
                        })
                        .collect()
                },
                Err(why) => {
                    warn!("ERROR: {:?}", why);
                    vec![]
                }
            };
            MenuItemIter::new(vec)
        }
    }

    #[derive(Debug)]
    enum TipError {
        Network(hyper::error::Error),
        Decoding(json::DecoderError),
        Io(io::Error),
    }

    impl From<hyper::error::Error> for TipError {
        fn from(err: hyper::error::Error) -> TipError {
            TipError::Network(err)
        }
    }

    impl From<json::DecoderError> for TipError {
        fn from(err: json::DecoderError) -> TipError {
            TipError::Decoding(err)
        }
    }

    impl From<io::Error> for TipError {
        fn from(err: io::Error) -> TipError {
            TipError::Io(err)
        }
    }
}

mod menu {
    use super::text_source::TextSource;
    use super::tip_source::TipSource;
    use protocol::{Selected,Menu,MenuItem,Path};


    const README: &'static str = r#"
       _   _          ___  ___  _   __
      (o)-(o)        | __|| o \/ \ / _|
   .-(   "   )-.     | _| |   ( o | |_n
  /  /`'-=-'`\  \    |_|  |_|\\\_/ \__/
__\ _\ \___/ /_ /__   ___  _  ___  __
  /|  /|\ /|\  |\    |_ _|| || o \/ _|
                      | | | ||  _/\_ \
                      |_| |_||_|  |__/

    W E L C O M E ,  F R I E N D

YOU ARE NOW CONNECTED TO THE LATEST IN
FROG SYSTEMS TECHNOLOGY.

FEEL FREE TO BROWSE AND DOWN-LOAD ALL
TWEETED FROG TIPS.
"#;

    pub struct RootMenu {
        // text_source: TextSource,
        // tip_source: TipSource,
        sources: Vec<Box<Source>>,
    }

    impl RootMenu {
        pub fn new(frog_tips_api_key: String) -> RootMenu {
            RootMenu {
                sources: vec![
                    Box::new(
                        TextSource::new(Path::from("/README"), "READ ALL ABOUT FROG, THE LATEST SENSATION.", README),
                    ),
                    Box::new(
                        TipSource::new(frog_tips_api_key),
                    )
                ],
            }
        }

        pub fn find(&self, path: &Path) -> Selected {
            self.sources.iter()
                        .filter_map(|s| s.find(path))
                        .nth(0)
                        .unwrap_or(Selected::Unknown)
        }
    }

    impl Menu for RootMenu {
        fn items(&self) -> Vec<MenuItem> {
            self.sources.iter()
                        .flat_map(|s| s.menu_items())
                        .collect()
        }
    }

    pub struct MenuItemIter {
        vec: Vec<MenuItem>,
    }

    impl MenuItemIter {
        pub fn new(vec: Vec<MenuItem>) -> MenuItemIter {
            MenuItemIter {
                vec: vec,
            }
        }
    }

    impl Iterator for MenuItemIter {
        type Item = MenuItem;

        fn next(&mut self) -> Option<Self::Item> {
            self.vec.pop()
        }
    }

    pub trait Source: Send {
        fn find(&self, path: &Path) -> Option<Selected>;
        fn menu_items(&self) -> MenuItemIter;
    }
}

use std::net::TcpStream;
use std::io::Write;
use std::io;

use self::menu::{RootMenu};
use protocol::{Selector,Selected,Protocol,ProtocolError,ExternalAddr};


pub struct Gopher {
    protocol: Protocol,
    root_menu: RootMenu,
}

impl Gopher {
    pub fn new(ext_addr: ExternalAddr, frog_tips_api_key: String) -> Gopher {
        let max_line_len = 512;
        Gopher {
            protocol: Protocol::new(ext_addr, max_line_len),
            root_menu: RootMenu::new(frog_tips_api_key),
        }
    }

    pub fn respond(&mut self, mut stream: TcpStream) -> io::Result<()> {
        let resp = {
            let selected = match try!(self.protocol.read(&mut stream)) {
                Selector::Path(ref path) => self.root_menu.find(path),
                Selector::Empty => Selected::Menu(&self.root_menu),
            };

            try!(self.protocol.write(&mut stream, &selected))
        };

        Ok(resp)
    }
}

impl From<ProtocolError> for io::Error {
    fn from(err: ProtocolError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, err)
    }
}
