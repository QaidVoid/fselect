use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::ops::Add;
use std::ops::Index;
use std::path::Path;

use regex::Captures;
use regex::Error;
use regex::Regex;

#[derive(Clone, Debug)]
pub struct HgignoreFilter {
    pub regex: Regex,
}

impl HgignoreFilter {
    fn new(regex: Regex) -> HgignoreFilter {
        HgignoreFilter {
            regex
        }
    }
}

enum Syntax {
    Regexp, Glob
}

impl Syntax {
    fn from(s: &str) -> Result<Syntax, String> {
        if s == "regexp" {
            return Ok(Syntax::Regexp);
        } else if s == "glob" {
            return Ok(Syntax::Glob);
        } else {
            return Err("Error parsing syntax directive".to_string());
        }
    }
}

pub fn parse_hgignore(file_path: &Path, dir_path: &Path) -> Result<Vec<HgignoreFilter>, String> {
    let mut result = vec![];
    let mut err = String::new();

    if let Ok(file) = File::open(file_path) {
        let mut syntax = Syntax::Regexp;

        let reader = BufReader::new(file);
        reader.lines()
            .filter(|line| {
                match line {
                    Ok(line) => !line.trim().is_empty() && !line.starts_with("#"),
                    _ => false
                }
            })
            .for_each(|line| {
                if err.is_empty() {
                    match line {
                        Ok(line) => {
                            if line.starts_with("syntax:") {
                                let line = line.replace("syntax:", "");
                                let syntax_directive = line.trim();
                                match Syntax::from(syntax_directive) {
                                    Ok(parsed_syntax) => syntax = parsed_syntax,
                                    Err(parse_err) => err = parse_err
                                }
                            } else if line.starts_with("subinclude:") {
                                let include = line.replace("subinclude:", "");
                                let mut parse_result = parse_hgignore(&Path::new(&include), dir_path);
                                match parse_result {
                                    Ok(ref mut filters) => {
                                        result.append(filters);
                                    },
                                    Err(parse_err) => {
                                        err = parse_err;
                                    }
                                };
                            } else {
                                let pattern = convert_hgignore_pattern(&line, dir_path, &syntax);
                                match pattern {
                                    Ok(pattern) => result.push(pattern),
                                    Err(parse_err) => err = parse_err
                                }
                            }
                        },
                        _ => { }
                    }
                }
            });
    };

    match err.is_empty() {
        true => Ok(result),
        false => Err(err)
    }
}

fn convert_hgignore_pattern(pattern: &str, file_path: &Path, syntax: &Syntax) -> Result<HgignoreFilter, String> {
    match syntax {
        Syntax::Glob => {
            match convert_hgignore_glob(pattern, file_path) {
                Ok(regex) => Ok(HgignoreFilter::new(regex)),
                _ => Err("Error creating regex while parsing .hgignore glob: ".to_string() + pattern)
            }
        },
        Syntax::Regexp => {
            match convert_hgignore_regexp(pattern, file_path) {
                Ok(regex) => Ok(HgignoreFilter::new(regex)),
                _ => Err("Error creating regex while parsing .hgignore regexp: ".to_string() + pattern)
            }
        }
    }
}

fn convert_hgignore_glob(glob: &str, file_path: &Path) -> Result<Regex, Error> {
    #[cfg(not(windows))]
        {
            let replace_regex = Regex::new("(\\*\\*|\\?|\\.|\\*|\\[|\\]|\\(|\\)|\\^|\\$)").unwrap();
            let mut pattern = replace_regex.replace_all(&glob, |c: &Captures| {
                match c.index(0) {
                    "**" => ".*",
                    "." => "\\.",
                    "*" => "[^/]*",
                    "?" => "[^/]+",
                    "[" => "\\[",
                    "]" => "\\]",
                    "(" => "\\(",
                    ")" => "\\)",
                    "^" => "\\^",
                    "$" => "\\$",
                    _ => panic!("Error parsing pattern")
                }.to_string()
            }).to_string();

            pattern = file_path.to_string_lossy().to_string()
                .replace("\\", "\\\\")
                .add("/([^/]+/)*").add(&pattern);

            Regex::new(&pattern)
        }

    #[cfg(windows)]
        {
            let replace_regex = Regex::new("(\\*\\*|\\?|\\.|\\*|\\[|\\]|\\(|\\)|\\^|\\$)").unwrap();
            let mut pattern = replace_regex.replace_all(&glob, |c: &Captures| {
                match c.index(0) {
                    "**" => ".*",
                    "." => "\\.",
                    "*" => "[^\\\\]*",
                    "?" => "[^\\\\]+",
                    "[" => "\\[",
                    "]" => "\\]",
                    "(" => "\\(",
                    ")" => "\\)",
                    "^" => "\\^",
                    "$" => "\\$",
                    _ => panic!("Error parsing pattern")
                }.to_string()
            }).to_string();

            pattern = file_path.to_string_lossy().to_string()
                .replace("\\", "\\\\")
                .add("\\\\([^\\\\]+\\\\)*").add(&pattern);

            Regex::new(&pattern)
        }
}

fn convert_hgignore_regexp(regexp: &str, file_path: &Path) -> Result<Regex, Error> {
    #[cfg(not(windows))]
        {
            let mut pattern = file_path.to_string_lossy().to_string();
            if !pattern.starts_with("^") {
                pattern = pattern.add("/([^/]+/)*");
            } else {
                pattern.trim_start_matches("^");
            }

            pattern = pattern.add(&regexp);

            Regex::new(&pattern)
        }

    #[cfg(windows)]
        {
            let mut pattern = file_path.to_string_lossy().to_string();
            if !pattern.starts_with("^") {
                pattern = pattern.add("\\\\([^\\\\]+\\\\)*");
            } else {
                pattern.trim_start_matches("^");
            }

            pattern = pattern.add(&regexp);

            Regex::new(&pattern)
        }
}