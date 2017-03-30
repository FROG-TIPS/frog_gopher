mod genuine_frog_source {
    use protocol::{Menu,MenuItem,Path,Selected};
    use super::menu::{Source,MenuItemIter};

    static SEARCH_PATH: &'static str = "/GENUINEFROG";
    static WE_REGRET_TO_INFORM_YOU: &'static str = "FROG SYSTEMS REGRETS TO INFORM YOU THAT YOU HAVE A COUNTERFEIT FROG. PLEASE CALL +1 (415) FROG-SYS TO TALK TO OUR SUPPORT STAFF.";

    struct SearchResultsMenu {
    }

    impl Menu for SearchResultsMenu {
        fn items(&self) -> Vec<MenuItem> {
            vec![MenuItem::Info { desc: WE_REGRET_TO_INFORM_YOU.to_string() }]
        }
    }

    pub struct GenuineFrogSource {
    }

    impl GenuineFrogSource {
        pub fn new() -> GenuineFrogSource {
            GenuineFrogSource {}
        }
    }

    impl Source for GenuineFrogSource {
        fn find(&self, path: &Path) -> Option<Selected> {
            let val = path.val();
            if val.starts_with(SEARCH_PATH) {
                Some(Selected::TempMenu(Box::new(SearchResultsMenu {})))
            } else {
                None
            }
        }

        fn menu_items(&self) -> MenuItemIter {
            MenuItemIter::new(vec![MenuItem::Search {
                path: Path::from(SEARCH_PATH),
                desc: "CHECK IF YOU HAVE A GENUINE FROG.".to_string(),
            }])
        }
    }
}

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
    use rustc_serialize::json;
    use reqwest;

    use std::io::Read;
    use std::io;

    use protocol::{Menu,MenuItem,Path,Selected};
    use super::menu::{MenuItemIter,Source};

    use itertools::Itertools;


    static ROOT_PATH: &'static str = "/TIP/";
    static SEARCH_PATH: &'static str = "/TIP/SEARCH";

    fn tips_into_menu_items(tips: &Vec<Tip>) -> Vec<MenuItem> {
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
    }

    type TipNum = u64;

    enum TipPath {
        Tip(TipNum),
        Search(Option<String>),
        Unknown,
    }

    impl From<Path> for TipPath {
        fn from(path: Path) -> TipPath {
            let val = path.val();
            if val.starts_with(SEARCH_PATH) {
                TipPath::Search(path.extra().map(|x| x.clone()))
            } else if val.starts_with(ROOT_PATH) {
                match val.split("/").last() {
                    Some(num) => match num.parse::<TipNum>() {
                        Ok(num) => TipPath::Tip(num),
                        _ => TipPath::Unknown,
                    },
                    None => TipPath::Unknown,
                }
            } else {
                TipPath::Unknown
            }
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

    // Search stuff

    #[derive(RustcDecodable)]
    struct SearchResults {
        results: Vec<Tip>,
    }

    #[derive(RustcEncodable)]
    struct SearchQuery {
        tweeted: bool,
        approved: bool,
        tip: Option<String>,
    }

    struct SearchResultsMenu {
        tips: Vec<Tip>,
    }

    impl Menu for SearchResultsMenu {
        fn items(&self) -> Vec<MenuItem> {
            tips_into_menu_items(&self.tips)
        }
    }

    // Access tips

    pub struct TipSource {
        api_key: String,
        client: reqwest::Client,
    }

    impl TipSource {
        pub fn new(api_key: String) -> TipSource {
            let client = reqwest::Client::new().unwrap();
            TipSource {
                api_key: api_key,
                client: client,
            }
        }

        fn one_tip(&self, number: TipNum) -> Result<Option<Tip>, TipError> {
            let url = format!("https://frog.tips/api/2/tips/{}", number);
            let mut resp = try!(
                self.client.get(&url)
                           .header(reqwest::header::Authorization(self.api_key.clone()))
                           .header(reqwest::header::Connection::close())
                           .send());

            {
                let status = resp.status();
                if status != &reqwest::StatusCode::Ok {
                    warn!("NO TIP FOUND BECAUSE: {:?}", status);
                    return Ok(None);
                }
            }

           let mut body = String::new();
           try!(resp.read_to_string(&mut body));
           let tip: Tip = try!(json::decode(&body));
           Ok(Some(tip))
        }

        fn all_tips(&self) -> Result<Vec<Tip>, TipError> {
            self.search_tips(None)
        }

        fn search_tips(&self, text: Option<String>) -> Result<Vec<Tip>, TipError> {
            let query = SearchQuery {
                approved: true,
                tweeted: true,
                tip: text,
            };
            let body = try!(json::encode(&query));

            let mut resp = try!(
                self.client.post("https://frog.tips/api/2/tips/search")
                           .body(body)
                           .header(reqwest::header::Authorization(self.api_key.clone()))
                           .header(reqwest::header::Connection::close())
                           .send());

            {
                let status = resp.status();
                if status != &reqwest::StatusCode::Ok {
                    warn!("NO TIP FOUND BECAUSE: {:?}", status);
                    return Ok(vec![]);
                }
            }

            let mut body = String::new();
            try!(resp.read_to_string(&mut body));
            let results: SearchResults = try!(json::decode(&body));
            Ok(results.results)
        }
    }

    impl Source for TipSource {
        fn find(&self, path: &Path) -> Option<Selected> {
            let tip_path = TipPath::from((*path).clone());
            match tip_path {
                TipPath::Tip(num) => match self.one_tip(num) {
                    Ok(Some(tip)) => {
                        Some(Selected::Text(Box::new(tip.tip)))
                    },
                    Ok(None) => {
                        warn!("TIP {} NOT FOUND.", num);
                        None
                    },
                    Err(why) => {
                        warn!("ERROR FETCHING TIP: {:?}", why);
                        None
                    },
                },
                TipPath::Search(text) => match self.search_tips(text) {
                    Ok(tips) => {
                        Some(Selected::TempMenu(Box::new(SearchResultsMenu { tips: tips })))
                    },
                    Err(why) => {
                        warn!("ERROR SEARCHING FOR TIP: {:?}", why);
                        None
                    },
                },
                _ => {
                    None
                },
            }
        }

        fn menu_items(&self) -> MenuItemIter {
            let mut vec = match self.all_tips() {
                Ok(ref tips) => {
                    tips_into_menu_items(tips)
                },
                Err(why) => {
                    warn!("COULD NOT PROVIDE TIPS: {:?}", why);
                    vec![]
                }
            };
            vec.insert(0, MenuItem::Search {
                path: Path::from(SEARCH_PATH),
                desc: "SEARCH FOR A FROG TIP.".to_string(),
            });
            vec.insert(0, MenuItem::Info {
                desc: "\nINTERACT WITH ALL TWEETED FROG TIPS, SORTED FROM LATEST TO THE EARLIEST TWEETED.".to_string()
            });
            MenuItemIter::new(vec)
        }
    }

    #[derive(Debug)]
    enum TipError {
        Network(reqwest::Error),
        Decoding(json::DecoderError),
        Search(json::EncoderError),
        Io(io::Error),
    }

    impl From<reqwest::Error> for TipError {
        fn from(err: reqwest::Error) -> TipError {
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

    impl From<json::EncoderError> for TipError {
        fn from(err: json::EncoderError) -> TipError {
            TipError::Search(err)
        }
    }
}

mod menu {
    use protocol::{Selected,Menu,MenuItem,Path};


    pub struct AnyMenu {
        sources: Vec<Box<Source>>
    }

    impl AnyMenu {
        pub fn new() -> AnyMenu {
            AnyMenu {
                sources: vec![],
            }
        }

        pub fn push<S: 'static + Source>(&mut self, source: S) {
            self.sources.push(Box::new(source));
        }

        pub fn find(&self, path: &Path) -> Option<Selected> {
            info!("PATH: '{}'", path);
            self.sources.iter()
                        .filter_map(|s| s.find(path))
                        .nth(0)
        }
    }

    impl Menu for AnyMenu {
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

use hyper::Url;

use self::menu::AnyMenu;
use self::tip_source::TipSource;
use self::text_source::TextSource;
use self::bogus_source::BogusSource;
use self::info_source::InfoSource;
use self::url_source::UrlSource;
use self::genuine_frog_source::GenuineFrogSource;
use protocol::{Selector,Selected,Path,Protocol,ProtocolError,ExternalAddr};


static MAX_LINE_LEN: usize = 512;

static README: &'static str = include_str!("../txt/README");
static FROG_MODELS: &'static str = include_str!("../txt/FROG_MODELS");
static FIRMWARE_V2: &'static str = include_str!("../txt/FIRMWARE_V2");
static JOB_OPENINGS: &'static str = include_str!("../txt/JOB_OPENINGS");
static JOB_OPENINGS_MOD_DATE: &'static str = "8 AUGUST 2016";
static EVACUATION_PROCEDURE: &'static str = include_str!("../txt/EVACUATION_PROCEDURE");

pub struct Gopher {
    ext_addr: ExternalAddr,
    menu: AnyMenu,
}

impl Gopher {
    pub fn new(ext_addr: ExternalAddr, frog_tips_api_key: String) -> Gopher {
        let mut menu = AnyMenu::new();

        menu.push(
            InfoSource::new(README));
        menu.push(
            UrlSource::new(Url::parse("https://frog.tips").unwrap(), "FROG TIPS MAIN WEBSPACE."));
        menu.push(
            UrlSource::new(Url::parse("https://github.com/FROG-TIPS").unwrap(), "FROG SYSTEMS TECHNICAL RESOURCES."));
        menu.push(
            UrlSource::new(Url::parse("http://hosting.frog.tips/rules.html").unwrap(), "FROG SYSTEMS (C) SONG CONTEST RULES."));
        menu.push(
            GenuineFrogSource::new());
        menu.push(
            UrlSource::new(Url::parse("https://twitter.com/FrogTips").unwrap(), "FROG SYSTEMS REAL-TIME WIRE SERVICE."));
        menu.push(
            InfoSource::new("IF YOU ARE EXPERIENCING AN EMERGENCY AT OUR MCMURDO BASE OF OPERATIONS,\nPLEASE SEND A WIRE TO THE ABOVE SERVICE IMMEDIATELY.\n"));
        menu.push(
            TextSource::new(Path::from("/JOB_OPENINGS"), "CURRENT FROG SYSTEMS INC. JOB OPENINGS.", JOB_OPENINGS));
        menu.push(
            InfoSource::new(format!("(UPDATED {})\n", JOB_OPENINGS_MOD_DATE)));
        menu.push(
            TextSource::new(Path::from("/README"), "READ ALL ABOUT FROG, THE LATEST SENSATION.", README));
        menu.push(
            BogusSource::new(Path::from("/USER_MANUAL"), "FROG USER MANUAL (EN) 17TH REV. INCLUDING APPENDICES."));
        menu.push(
            TextSource::new(Path::from("/FROG_MODELS"), "NON-CANON FROG MODEL LISTING.", FROG_MODELS));
        menu.push(
            TextSource::new(Path::from("/EVACUATION_PROCEDURE"), "OFFICIAL EVACUATION PROCEDURE.", EVACUATION_PROCEDURE));
        menu.push(
            TextSource::new(Path::from("/FIRMWARE_V2"), "FROG V2 FIRMWARE FOR ALL NON-OCEANIA MODELS", FIRMWARE_V2));
        menu.push(
            TipSource::new(frog_tips_api_key));

        Gopher {
            ext_addr: ext_addr,
            menu: menu,
        }
    }

    pub fn respond(&self, mut stream: TcpStream) -> io::Result<()> {
        let resp = {
            // FIXME: This protocol contains state that should not be shared
            // However, it seems silly to create a new struct every time
            let mut protocol = Protocol::new(&self.ext_addr, MAX_LINE_LEN);

            let selected = match try!(protocol.read(&mut stream)) {
                Selector::Path(ref path) => self.menu.find(path)
                                                     .unwrap_or(
                                                         Selected::Error(
                                                             Box::new(format!("{} NOT FOUND", path)))),
                Selector::Empty => Selected::ForeverMenu(&self.menu),
            };

            try!(protocol.write(&mut stream, &selected))
        };

        Ok(resp)
    }
}

impl From<ProtocolError> for io::Error {
    fn from(err: ProtocolError) -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, err)
    }
}

unsafe impl Sync for Gopher {}
