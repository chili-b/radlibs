use std::env::args;
use std::fs::File;
use std::io::{self, BufReader, BufRead, Stdin, Write, Seek, SeekFrom};
use std::collections::{HashMap, HashSet};

const PROMPT_START: u8 = b'{';
const PROMPT_END: u8 = b'}';
const ESCAPE_CHAR: u8 = b'\\';
const IDENTIFIER: &str = "@";
const SEPARATOR: &str = " ";

fn main() {
    if let Some(path) = args().nth(1) {
        match File::open(&path) {
            Ok(file) => radlibs(file),
            Err(e) => eprintln!("couldn't open {} ({})", path, e)
        }
    } else {
        eprintln!("please provide a file");
    }
}

fn radlibs(mut file: File) {
    let word_map = prompt_for_words_in(&mut file);
    substitute_words_in(&mut file, word_map);
}

#[derive(Clone)]
enum ParseMode {
    Preceding = PROMPT_START as isize,
    Containing = PROMPT_END as isize,
}

fn parse_file<T>(file: &mut File, parser: &mut T) 
where T: Parser 
{
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut reader = BufReader::new(file);
    let mut buf = vec![];
    let mut parse_mode = ParseMode::Preceding;
    loop {
        match reader.read_until(parse_mode.clone() as u8, &mut buf) {
            Ok(bytes) => {
                if bytes == 0 {
                    break;
                } else if buf.len() < 2 || buf[buf.len() - 2] != ESCAPE_CHAR {
                    // remove delimiting character
                    buf.pop();
                    match parse_mode {
                        ParseMode::Preceding => {
                            parser.parse_preceding(&mut buf);
                            parse_mode = ParseMode::Containing;
                        }
                        ParseMode::Containing => {
                            parser.parse_containing(&mut buf);
                            parse_mode = ParseMode::Preceding;
                        },
                    }
                    buf.clear();
                } else if buf[buf.len() - 2] == ESCAPE_CHAR {
                    // remove escape character
                    buf.remove(buf.len() - 2);
                }
            },
            Err(e) => panic!("Error while reading: {}", e),
        }
    }
}

fn prompt_for_words_in(file: &mut File) -> HashMap<String, (HashSet<String>, bool)> {
    let mut input_parser = InputParser::new();
    parse_file(file, &mut input_parser);
    input_parser.get_word_map()
}

fn substitute_words_in(file: &mut File, word_map: HashMap<String, (HashSet<String>, bool)>) {
    let mut output_parser = OutputParser::new(word_map);
    parse_file(file, &mut output_parser);
    io::stdout().flush().unwrap();
    println!();
}

// Parser trait
trait Parser {
    fn parse_preceding(&mut self, buf: &mut Vec<u8>);
    fn parse_containing(&mut self, buf: &mut Vec<u8>);
}

// Parser for asking for input
struct InputParser {
    stdin: Stdin,
    word_map: HashMap<String, (HashSet<String>, bool)>,
}

impl InputParser {
    pub fn new() -> Self {
        Self {
            stdin: io::stdin(),
            word_map: HashMap::new(),
        }
    }

    pub fn get_word_map(self) -> HashMap<String, (HashSet<String>, bool)> {
        self.word_map    
    }

    fn add_identifier(&mut self, identifier: String, is_persistent: bool) {
        if !self.word_map.contains_key(&identifier) {
            self.word_map.insert(identifier, (HashSet::new(), is_persistent));
        }
    }

    fn add_word(&mut self, identifier: String, word: String, is_persistent: bool) {
        self.add_identifier(identifier.clone(), is_persistent);
        self.word_map.get_mut(&identifier).unwrap().0.insert(word);
    }
}

impl Parser for InputParser {
    fn parse_preceding(&mut self, _buf: &mut Vec<u8>) {}

    fn parse_containing(&mut self, buf: &mut Vec<u8>) {
        let prompt_text = String::from_utf8(buf.to_vec()).unwrap();
        let (prompt, identifier, is_persistent) = if prompt_text.starts_with(IDENTIFIER) {
            let prompt_parts: Vec<&str> = prompt_text.split(SEPARATOR).collect();
            if prompt_parts.len() <= 1 {
                return;
            } else {
                let mut prompt = String::new();
                for i in 1..prompt_parts.len() {
                    if i != 1 {
                        prompt.push_str(SEPARATOR);
                    }
                    prompt.push_str(prompt_parts[i]);
                }
                let identifier = String::from(prompt_parts[0]);
                (prompt, identifier, true)
            }
        } else {
            (prompt_text.clone(), prompt_text, false)
        };
        print!("Please input {}: ", &prompt);
        io::stdout().flush().unwrap();
        let mut word = String::new();
        self.stdin.read_line(&mut word).unwrap();
        word.pop();
        self.add_word(identifier, word, is_persistent);
    }
}

// Parser for printing out the result
struct OutputParser {
    word_map: HashMap<String, (HashSet<String>, bool)>
}

impl OutputParser {
    pub fn new(word_map: HashMap<String, (HashSet<String>, bool)>) -> Self {
        Self {
            word_map: word_map
        }
    }

    fn take_word(&mut self, prompt: String) -> String {
        let word_set = self.word_map.get_mut(&prompt).unwrap();
        let word = String::from(word_set.0.iter().nth(0).unwrap());
        if !word_set.1 {
            word_set.0.remove(&word);
            if word_set.0.is_empty() {
                self.word_map.remove(&prompt);
            }
        }
        word
    }
}

impl Parser for OutputParser {
    fn parse_preceding(&mut self, buf: &mut Vec<u8>) {
        let print_text = String::from_utf8(buf.to_vec()).unwrap();
        print!("{}", print_text);
    }

    fn parse_containing(&mut self, buf: &mut Vec<u8>) {
        let identifier_text = String::from_utf8(buf.to_vec()).unwrap();
        let identifier = if identifier_text.starts_with(IDENTIFIER) {
            String::from(identifier_text.split(SEPARATOR).nth(0).unwrap())
        } else {
            identifier_text
        };
        let word = self.take_word(identifier);
        print!("{}", word);
    }
}
