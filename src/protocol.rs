
use std::io;
use std::io::{Read,Write};
use std::net::TcpStream;
use std::string::FromUtf8Error;
use std::error;
use std::fmt;

use hyper::Url;


#[derive(Clone,Debug,Eq,PartialEq)]
pub struct Path {
    val: String,
    // Additional crap
    extra: Option<String>,
}

impl Path {
    pub fn from<S: Into<String>>(val: S) -> Path {
        Path {
            val: val.into(),
            extra: None,
        }
    }

    pub fn new<S: Into<String>>(val: S, extra: Option<S>) -> Path {
        Path {
            val: val.into(),
            extra: extra.map(|x| x.into()),
        }
    }

    pub fn val(&self) -> &String {
        &self.val
    }

    pub fn extra(&self) -> Option<&String> {
        self.extra.as_ref()
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref extra) = self.extra {
            write!(f, "{} (extra: {})", self.val, extra)
        } else {
            write!(f, "{}", self.val)
        }
    }
}

// Reading

#[derive(Debug)]
pub enum Selector {
    Path(Path),
    Empty,
}

// Writing

pub enum Selected<'a> {
    Error(Box<String>),
    // Newline-delimited lines to write
    Text(Box<String>),
    TempMenu(Box<Menu>),
    ForeverMenu(&'a Menu),
}

pub trait Menu {
    fn items(&self) -> Vec<MenuItem>;
}

#[derive(Debug)]
pub enum MenuItem {
    // Other types are not supported
    Text {path: Path, desc: String},
    Info {desc: String},
    JohnGoerzenUrl {url: Url, desc: String},
    Search {path: Path, desc: String},
}

// Internals

#[derive(Clone,Debug)]
enum State {
    Idle,
    Path,
    Extra,
    Newline,
}

#[derive(Debug)]
enum Token {
    Extra(u8),
    Path(u8),
    Newline,
}

#[derive(Clone,Debug)]
pub struct ExternalAddr {
    host: String,
    port: u16,
}

impl ExternalAddr {
    pub fn new<S: Into<String>>(host: S, port: u16) -> ExternalAddr {
        ExternalAddr {
            host: host.into(),
            port: port,
        }
    }
}

#[derive(Debug)]
pub enum ParseExternalAddrError {
    BadFormat,
    BadPort,
}

impl From<::std::num::ParseIntError> for ParseExternalAddrError {
    fn from(_: ::std::num::ParseIntError) -> ParseExternalAddrError {
        ParseExternalAddrError::BadPort
    }
}

impl fmt::Display for ParseExternalAddrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseExternalAddrError::BadFormat => write!(f, "Invalid format error"),
            ParseExternalAddrError::BadPort => write!(f, "Invalid port error"),
        }
    }
}

impl error::Error for ParseExternalAddrError {
    fn description(&self) -> &str {
        match *self {
            ParseExternalAddrError::BadFormat => "Invalid format. Valid formats are 'host.name:1111' or 'host.name 1111'",
            ParseExternalAddrError::BadPort => "Invalid port",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        None
    }
}

impl ::std::str::FromStr for ExternalAddr {
    type Err = ParseExternalAddrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(|x| (x == ' ') || (x == ':')).collect();
        if parts.len() != 2 {
            Err(ParseExternalAddrError::BadFormat)
        } else {
            let host = parts[0];
            let port = try!(parts[1].parse::<u16>());
            Ok(ExternalAddr::new(host, port))
        }
    }
}

// Through careful research, this number has been chosen to anger as many people as possible
const READ_BUFFER_SIZE: usize = 1;

const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;

#[derive(Clone,Debug)]
pub struct Protocol<'a> {
    ext_addr: &'a ExternalAddr,
    state: State,
    remaining: Vec<u8>,
    max_line_len: usize,
}

impl<'a> Protocol<'a> {
    pub fn new(ext_addr: &ExternalAddr, max_line_len: usize) -> Protocol {
        Protocol {
            ext_addr: ext_addr,
            state: State::Idle,
            remaining: Vec::with_capacity(READ_BUFFER_SIZE),
            max_line_len: max_line_len,
        }
    }

    fn read_stream(&mut self, stream: &mut TcpStream) -> Result<Option<Token>, ProtocolError> {
        loop {
            while let Some(byte) = self.remaining.pop() {
                let (new_state, token) = match (&self.state, byte) {
                    (&State::Idle, CR) | (&State::Path, CR) | (&State::Extra, CR) => (State::Newline, None),
                    (&State::Idle, w) | (&State::Path, w) if w == 9 || w == 32 => (State::Extra, None),
                    (&State::Idle, b) | (&State::Path, b) => (State::Path, Some(Token::Path(b))),
                    (&State::Extra, b) => (State::Extra, Some(Token::Extra(b))),
                    (&State::Newline, LF) => (State::Idle, Some(Token::Newline)),
                    (&State::Newline, _) => (State::Idle, None),
                };

                self.state = new_state;

                if token.is_some() {
                    return Ok(token);
                }
            }

            let mut buffer = [0; READ_BUFFER_SIZE];
            let bytes_read = try!(stream.read(&mut buffer));

            // Just stop trying already
            if bytes_read == 0 {
                return Ok(None);
            }

            self.remaining.extend_from_slice(&buffer[0 .. bytes_read]);
            self.remaining.reverse();
        }
    }

    pub fn read(&mut self, stream: &mut TcpStream) -> Result<Selector, ProtocolError> {
        let mut selector_builder = SelectorBuilder::new(self.max_line_len);

        while let Some(token) = try!(self.read_stream(stream)) {
            match token {
                Token::Path(byte) => {
                    try!(selector_builder.push_path(byte));
                },
                Token::Extra(byte) => {
                    try!(selector_builder.push_extra(byte));
                },
                Token::Newline => {
                    return selector_builder.build();
                },
            };
        }

        Err(ProtocolError::UnfinishedBusiness)
    }

    fn write_menu(&mut self, stream: &mut TcpStream, menu: &Menu) -> Result<(), ProtocolError> {
        let addr = &self.ext_addr;
        for item in menu.items().iter() {
            match item {
                &MenuItem::Text {ref path, ref desc} => {
                    try!(write!(stream, "0{}\t{}\t{}\t{}\r\n", desc, path.val(), addr.host, addr.port))
                }
                &MenuItem::JohnGoerzenUrl {ref url, ref desc} => {
                    try!(write!(stream, "h{}\tURL:{}\t{}\t{}\r\n", desc, url, addr.host, addr.port))
                },
                &MenuItem::Info {ref desc} => {
                    for line in desc.split("\n") {
                        try!(write!(stream, "i{}\t\t\t\r\n", line))
                    }
                },
                &MenuItem::Search {ref path, ref desc} => {
                    try!(write!(stream, "7{}\t{}\t{}\t{}\r\n", desc, path, addr.host, addr.port))
                },
            }
        }

        Ok(())
    }

    pub fn write(&mut self, stream: &mut TcpStream, selected: &Selected) -> Result<(), ProtocolError> {
        match selected {
            &Selected::Text(ref text) => {
                try!(write!(stream, "{}\r\n", text))
            },
            &Selected::Error(ref why) => {
                try!(write!(stream, "3{}\r\n", why))
            },
            &Selected::ForeverMenu(ref menu) => {
                try!(self.write_menu(stream, *menu))
            },
            &Selected::TempMenu(ref menu) => {
                // TODO: Collapse this into the above pattern when box matching is in stable
                try!(self.write_menu(stream, &**menu))
            },
        };

        Ok(try!(write!(stream, ".\r\n")))
    }
}


#[derive(Debug)]
pub enum ProtocolError {
    LineTooBigError,
    ParseLineError(FromUtf8Error),
    UnfinishedBusiness,
    IoError(io::Error),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProtocolError::LineTooBigError => write!(f, "Line too big error"),
            ProtocolError::ParseLineError(ref err) => write!(f, "Parse line error: {}", err),
            ProtocolError::UnfinishedBusiness => write!(f, "Unfinished business error"),
            ProtocolError::IoError(ref err) => write!(f, "IoError: {}", err),
        }
    }
}

impl error::Error for ProtocolError {
    fn description(&self) -> &str {
        match *self {
            ProtocolError::LineTooBigError => "Line is too big to be read.",
            ProtocolError::ParseLineError(ref err) => err.description(),
            ProtocolError::UnfinishedBusiness => "The stream ended while parsing.",
            ProtocolError::IoError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            ProtocolError::LineTooBigError => None,
            ProtocolError::ParseLineError(ref err) => Some(err),
            ProtocolError::UnfinishedBusiness => None,
            ProtocolError::IoError(ref err) => Some(err),
        }
    }
}

impl From<FromUtf8Error> for ProtocolError {
    fn from(err: FromUtf8Error) -> ProtocolError {
        ProtocolError::ParseLineError(err)
    }
}

impl From<io::Error> for ProtocolError {
    fn from(err: io::Error) -> ProtocolError {
        ProtocolError::IoError(err)
    }
}

struct SelectorBuilder {
    max_line_len: usize,
    path_buffer: Vec<u8>,
    extra_buffer: Vec<u8>,
}

impl SelectorBuilder {
    fn new(max_line_len: usize) -> SelectorBuilder {
        SelectorBuilder {
            max_line_len: max_line_len,
            path_buffer: Vec::with_capacity(max_line_len),
            extra_buffer: Vec::with_capacity(max_line_len),
        }
    }

    fn check_capacity(&self) -> Result<(), ProtocolError> {
        if self.max_line_len == self.path_buffer.len() ||
           self.max_line_len == self.extra_buffer.len() {
               Err(ProtocolError::LineTooBigError)
           } else {
               Ok(())
           }
    }

    fn reset(&mut self) {
        // Tidy up the buffers
        self.path_buffer.clear();
        self.extra_buffer.clear();
    }

    fn push_path(&mut self, byte: u8) -> Result<(), ProtocolError> {
        try!(self.check_capacity());
        self.path_buffer.push(byte);
        Ok(())
    }

    fn push_extra(&mut self, byte: u8) -> Result<(), ProtocolError> {
        try!(self.check_capacity());
        self.extra_buffer.push(byte);
        Ok(())
    }

    fn build(&mut self) -> Result<Selector, ProtocolError> {
        if self.path_buffer.len() == 0 {
            self.reset();
            return Ok(Selector::Empty);
        }


        let path_bytes = self.path_buffer.clone();
        let extra_bytes = self.extra_buffer.clone();

        self.reset();

        let path = try!(String::from_utf8(path_bytes));
        let extra = if extra_bytes.len() == 0 {
            None
        } else {
            Some(try!(String::from_utf8(extra_bytes)))
        };

        Ok(Selector::Path(Path::new(path, extra)))
    }
}
