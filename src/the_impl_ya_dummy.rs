mod bogus_source {
    use protocol::{MenuItem,Path,Selected};
    use super::menu::{Source,MenuItemIter};

    pub struct BogusSource {
        path: Path,
        desc: String,
    }

    impl BogusSource {
        pub fn new<S: Into<String>>(path: Path, desc: S) -> BogusSource {
            BogusSource {
                path: path,
                desc: desc.into(),
            }
        }
    }

    impl Source for BogusSource {
        fn find(&self, _: &Path) -> Option<Selected> {
            None
        }

        fn menu_items(&self) -> MenuItemIter {
            MenuItemIter::new(vec![MenuItem::Text {
                path: self.path.clone(),
                desc: self.desc.clone(),
            }])
        }
    }
}

mod url_source {
    use hyper::Url;

    use protocol::{MenuItem,Path,Selected};
    use super::menu::{Source,MenuItemIter};

    pub struct UrlSource {
        pub url: Url,
        pub desc: String,
    }

    impl UrlSource {
        pub fn new<S: Into<String>>(url: Url, desc: S) -> UrlSource {
            UrlSource {
                url: url,
                desc: desc.into(),
            }
        }
    }

    impl Source for UrlSource {
        fn find(&self, _: &Path) -> Option<Selected> {
            None
        }

        fn menu_items(&self) -> MenuItemIter {
            MenuItemIter::new(vec![MenuItem::JohnGoerzenUrl {
                url: self.url.clone(),
                desc: self.desc.clone(),
            }])
        }
    }
}

mod info_source {
    use protocol::{MenuItem,Path,Selected};
    use super::menu::{Source,MenuItemIter};

    pub struct InfoSource {
        desc: String,
    }

    impl InfoSource {
        pub fn new<S: Into<String>>(desc: S) -> InfoSource {
            InfoSource {
                desc: desc.into(),
            }
        }
    }

    impl Source for InfoSource {
        fn find(&self, _: &Path) -> Option<Selected> {
            None
        }

        fn menu_items(&self) -> MenuItemIter {
            MenuItemIter::new(vec![MenuItem::Info {
                desc: self.desc.clone(),
            }])
        }
    }
}

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

    use itertools::Itertools;


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
            let client = {
                let mut client = hyper::Client::new();
                let ten_seconds = Some(::std::time::Duration::from_secs(10));
                client.set_read_timeout(ten_seconds);
                client.set_write_timeout(ten_seconds);
                client
            };

            TipSource {
                api_key: api_key,
                client: client,
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
                   warn!("NO TIP FOUND BECASE: {:?}", other);
                   warn!("NO TIP FOUND BECAUSE: {:?}", other);
                   Ok(None)
               }
           }
        }

        fn all_tips(&self) -> Result<Vec<Tip>, TipError> {
            let mut resp = try!(
                self.client.post("https://frog.tips/api/2/tips/search")
                           .body("{\"tweeted\": true, \"approved\": true}")
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
                other => {
                    warn!("NO TIPS FOUND BECAUSE: {:?}", other);
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
                        warn!("COULD NOT PROVIDE TIP {}: {:?}", path, why);
                        None
                    }
                }
            })
        }

        fn menu_items(&self) -> MenuItemIter {
            let mut vec = match self.all_tips() {
                Ok(tips) => {
                    tips.into_iter()
                        .sorted_by(|t1, t2| {
                            // Latest first
                            Ord::cmp(&t2.tweeted, &t1.tweeted)
                        })
                        .into_iter()
                        .map(|t| {
                            MenuItem::Text {
                                path: Path::from(format!("{}{}", ROOT_PATH, t.number)),
                                desc: format!("TIP #{}", t.number),
                            }
                        })
                        .collect()
                },
                Err(why) => {
                    warn!("COULD NOT PROVIDE TIPS: {:?}", why);
                    vec![]
                }
            };
            vec.insert(0, MenuItem::Info {
                desc: "BELOW ARE ALL TWEETED FROG TIPS, SORTED FROM LATEST TO THE EARLIEST TWEETED.".to_string()
            });
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
    use hyper::Url;

    use super::text_source::TextSource;
    use super::tip_source::TipSource;
    use super::bogus_source::BogusSource;
    use super::info_source::InfoSource;
    use super::url_source::UrlSource;
    use protocol::{Selected,Menu,MenuItem,Path};


    const README: &'static str = include_str!("../txt/README");
    const FROG_MODELS: &'static str = include_str!("../txt/FROG_MODELS");
    const FIRMWARE_V2: &'static str = include_str!("../txt/FIRMWARE_V2");
    const JOB_OPENINGS: &'static str = include_str!("../txt/JOB_OPENINGS");

    pub struct RootMenu {
        sources: Vec<Box<Source>>,
    }

    impl RootMenu {
        pub fn new(frog_tips_api_key: String) -> RootMenu {
            RootMenu {
                sources: vec![
                    // Print the README as a banner as well
                    Box::new(
                        InfoSource::new(README),
                    ),
                    Box::new(
                        // TODO: This should be caught as early as possible and so we panic
                        UrlSource::new(Url::parse("https://frog.tips").unwrap(), "FROG TIPS MAIN WEBSPACE."),
                    ),
                    Box::new(
                        UrlSource::new(Url::parse("http://hosting.frog.tips/rules.html").unwrap(), "FROG SYSTEMS (C) SONG CONTEST RULES."),
                    ),
                    Box::new(
                        UrlSource::new(Url::parse("https://mitpress.mit.edu/sicp/").unwrap(), "LISP WIZARD REFERENCE."),
                    ),
                    Box::new(
                        TextSource::new(Path::from("/README"), "READ ALL ABOUT FROG, THE LATEST SENSATION.", README),
                    ),
                    Box::new(
                        TextSource::new(Path::from("/JOB_OPENINGS"), "CURRENT FROG SYSTEMS INC. JOB OPENINGS.", JOB_OPENINGS),
                    ),
                    Box::new(
                        BogusSource::new(Path::from("/USER_MANUAL"), "FROG USER MANUAL (EN) 17TH REV. INCLUDING APPENDICES."),
                    ),
                    Box::new(
                        TextSource::new(Path::from("/FROG_MODELS"), "NON-CANON FROG MODEL LISTING.", FROG_MODELS),
                    ),
                    Box::new(
                        TextSource::new(Path::from("/FIRMWARE_V2"), "FROG V2 FIRMWARE FOR ALL NON-OCEANIA MODELS", FIRMWARE_V2),
                    ),
                    Box::new(
                        TipSource::new(frog_tips_api_key),
                    )
                ],
            }
        }

        pub fn find(&self, path: &Path) -> Selected {
            info!("PATH: '{}'", path);
            self.sources.iter()
                        .filter_map(|s| s.find(path))
                        .nth(0)
                        .unwrap_or(Selected::Error(
                            Box::new(
                                format!("{} NOT FOUND.", path))))
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
        pub fn new(mut vec: Vec<MenuItem>) -> MenuItemIter {
            // Reverse this so we can pop
            vec.reverse();
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
