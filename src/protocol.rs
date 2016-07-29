
use std::io;
use std::io::{Read,Write};
use std::net::TcpStream;
use std::string::FromUtf8Error;
use std::error;
use std::fmt;


// Through careful research, this number has been chosen to anger as many people as possible
const READ_BUFFER_SIZE: usize = 1;

const CR: u8 = '\r' as u8;
const LF: u8 = '\n' as u8;

#[derive(Clone,Debug,Eq,PartialEq)]
pub struct Path {
    val: String,
}

impl Path {
    pub fn from<S: Into<String>>(val: S) -> Path {
        Path {val: val.into()}
    }

    pub fn to_str(&self) -> &str {
        self.val.as_str()
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
    Unknown,
    Text(Box<String>),
    Menu(&'a Menu),
}

pub trait Menu {
    fn items(&self) -> Vec<MenuItem>;
}

#[derive(Debug)]
pub enum MenuItem {
    // Other types are not supported
    Text {path: Path, desc: String},
}

// Internals

#[derive(Clone,Debug)]
enum State {
    Idle,
    Newline,
}

#[derive(Debug)]
enum Token {
    Text(u8),
    Newline,
}

#[derive(Clone,Debug)]
pub struct Protocol {
    ext_addr: ExternalAddr,
    state: State,
    remaining: Vec<u8>,
    max_line_len: usize,
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

impl Protocol {
    pub fn new(ext_addr: ExternalAddr, max_line_len: usize) -> Protocol {
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
                    (&State::Idle, CR) => (State::Newline, None),
                    (&State::Idle, b) => (State::Idle, Some(Token::Text(b))),
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
                Token::Text(byte) => {
                    try!(selector_builder.push(byte));
                },
                Token::Newline => {
                    return selector_builder.build();
                },
            };
        }

        Err(ProtocolError::UnfinishedBusiness)
    }

    pub fn write(&mut self, stream: &mut TcpStream, selected: &Selected) -> Result<(), ProtocolError> {
        match selected {
            &Selected::Text(ref text) => {
                try!(write!(stream, "{}\r\n", text))
            },
            &Selected::Unknown => {
                try!(write!(stream, "3FROG NOT FOUND.\r\n"))
            },
            &Selected::Menu(ref menu) => {
                let addr = &self.ext_addr;
                for item in menu.items().iter() {
                    match item {
                        &MenuItem::Text {ref path, ref desc} => {
                            try!(write!(stream, "0{}\t{}\t{}\t{}\r\n", desc, path.to_str(), addr.host, addr.port))
                        }
                    }
                }
            }
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
    line_buffer: Vec<u8>,
}

impl SelectorBuilder {
    fn new(max_line_len: usize) -> SelectorBuilder {
        SelectorBuilder {
            max_line_len: max_line_len,
            line_buffer: Vec::with_capacity(max_line_len),
        }
    }

    fn push(&mut self, byte: u8) -> Result<(), ProtocolError> {
        if self.max_line_len == self.line_buffer.len() {
            Err(ProtocolError::LineTooBigError)
        } else {
            self.line_buffer.push(byte);
            Ok(())
        }
    }

    fn build(&mut self) -> Result<Selector, ProtocolError> {
        if self.line_buffer.len() == 0 {
            return Ok(Selector::Empty);
        }

        // Make a copy and tidy up the buffer
        let bytes = self.line_buffer.clone();
        self.line_buffer.clear();

        let string = try!(String::from_utf8(bytes));
        Ok(Selector::Path(Path::from(string)))
    }
}
