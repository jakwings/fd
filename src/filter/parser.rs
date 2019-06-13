use super::*;

// Operator in order of decreasing precedence:
// * "(" expression ")"
// * "NOT" expression , "!" expression
// * expression "AND" expression , expression expression
// * expression "XOR" expression
// * expression "OR"  expression
// * expression "," expression
// Operator names are case-insensitive.
//
// Expressions (unchained):
// * name <glob pattern>    # unaffected by the --case-insensitive flag
// * iname <glob pattern>
// * path <glob pattern>    # match the absolute/relative path, e.g. path '**/*.rs'
// * ipath <glob pattern>
// * regex <regex pattern>  # match the absolute/relative path, e.g. regex '/[^/]*\.rs$'
// * iregex <regex pattern>
// * type <file type[,file type]...>
// * prune                  # do not descend into a directory
// * quit                   # stop searching but not instantly due to multi-threading
// * true
// * false
// * print                  # unaffected by the --print0 flag
// * print0
// * ...
// The head of an expression is case-insensitive.

const MAX_RANK: u8 = 4;
const MAX_DEPTH: u8 = 8;

pub struct Config {
    pub unicode: bool,
}

enum Token<'a> {
    Lpr(&'a OsStr),
    Rpr(&'a OsStr),
    Txt(&'a OsStr),
    Raw(&'a OsStr),
}

struct Tokens<'a> {
    tokens: Vec<Token<'a>>,
}

impl<'a> std::fmt::Debug for Tokens<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let mut indent = String::new();

        fn to_string(token: &OsStr) -> String {
            String::from_utf8(token.as_bytes().to_vec()).unwrap_or(format!("{:?}", token))
        }

        write!(f, "0|")?;
        for token in &self.tokens {
            match token {
                Token::Lpr(token) => {
                    indent.push_str("  ");
                    write!(f, " {}\n{}|{}", to_string(token), indent.len() / 2, indent)?;
                }
                Token::Rpr(token) => {
                    if indent.len() > 0 {
                        indent.pop();
                        indent.pop();
                        write!(f, "\n{}|{}", indent.len() / 2, indent)?;
                    }
                    write!(f, " {}", to_string(token))?;
                }
                Token::Txt(token) => {
                    write!(f, " {}", to_string(token))?;
                }
                Token::Raw(token) => {
                    write!(f, " {:?}", token)?;
                }
            }
        }

        Ok(())
    }
}

impl<'a> Tokens<'a> {
    fn push(&mut self, token: Token<'a>) {
        self.tokens.push(token);
    }
}

impl<'a> Tokens<'a> {
    fn new() -> Tokens<'a> {
        Tokens { tokens: Vec::new() }
    }
}

pub struct Parser<'a, Iter: Iterator<Item = &'a OsStr>> {
    config: Config,
    tokens: Tokens<'a>,
    source: std::iter::Peekable<&'a mut Iter>,
}

impl<'a, Iter: Iterator<Item = &'a OsStr>> Parser<'a, Iter> {
    pub fn new(args: &'a mut Iter, config: Config) -> Parser<'a, Iter> {
        Parser {
            config,
            tokens: Tokens::new(),
            source: args.peekable(),
        }
    }

    pub fn parse(&mut self) -> Result<Chain, Error> {
        self.parse_expr(0, 0)
            .map_err(|err| Error::from_str(&format!("{:?}\n{}", self.tokens, err)))
    }

    fn next(&mut self, expected: Option<&[u8]>, errmsg: &str) -> Result<&'a OsStr, Error> {
        match (expected, self.source.next()) {
            (None, Some(token)) => Ok(token),
            (Some(expected), Some(token)) if token.as_bytes() == expected => Ok(token),
            _ => Err(Error::from_str(errmsg)),
        }
    }

    fn parse_expr(&mut self, depth: u8, rank: u8) -> Result<Chain, Error> {
        if depth > MAX_DEPTH {
            while let Some(token) = self.source.next() {
                self.tokens.push(Token::Txt(token));
            }

            Err(Error::from_str("filter chain reached the max depth"))
        } else if let Some(token) = self.source.next() {
            self.parse_predicate(depth, token)
                .and_then(|chain| self.parse_operator(depth, rank, chain))
        } else {
            Err(Error::from_str("filter chain was incomplete"))
        }
    }

    // IDEA: Dir(Pattern) = Type(Directory) & Path(Pattern)
    //       Ext(EXT) = Name(?*.EXT)
    fn parse_predicate(&mut self, depth: u8, token: &'a OsStr) -> Result<Chain, Error> {
        macro_rules! tok (($($anything:tt)+) => ({
            self.tokens.push(Token::Txt(token));
            $($anything)+
        }));

        // Double escaping for shell strings is troublesome. For convenience & consistency...
        // Do not support things like ["(!name", "?)"] because ")" is part of the pattern.
        if token.len() > 1 && token.as_bytes().starts_with(b"!") {
            let token = OsStr::from_bytes(token.as_bytes().split_first().unwrap().1);

            self.tokens.push(Token::Txt(OsStr::new("!")));
            self.parse_predicate(depth, token).map(|chain| chain.not())
        } else {
            match token.to_ascii_lowercase().as_bytes() {
                b"(" => {
                    self.tokens.push(Token::Lpr(token));
                    self.parse_expr(depth + 1, 0).and_then(|chain| {
                        self.next(
                            Some(b")"),
                            &format!(r#"found unpaired "(" in depth {}"#, depth),
                        )
                        .and_then(|token| {
                            self.tokens.push(Token::Rpr(token));
                            Ok(chain)
                        })
                    })
                }
                b"not" | b"!" => tok!(self.parse_expr(depth, MAX_RANK).map(|c| c.not())),
                b"type" => tok!(self.parse_file_type()),
                b"name" => tok!(self.parse_name_glob(false)),
                b"iname" => tok!(self.parse_name_glob(true)),
                b"path" => tok!(self.parse_path_glob(false)),
                b"ipath" => tok!(self.parse_path_glob(true)),
                b"regex" => tok!(self.parse_regex(false)),
                b"iregex" => tok!(self.parse_regex(true)),
                b"true" => tok!(Ok(Chain::new(Filter::Anything, false))),
                b"false" => tok!(Ok(Chain::new(Filter::Anything, true))),
                b"print" => tok!(Ok(Chain::new(Filter::Action(Action::Print), false))),
                b"print0" => tok!(Ok(Chain::new(Filter::Action(Action::Print0), false))),
                b"prune" => tok!(Ok(Chain::new(Filter::Action(Action::Prune), false))),
                b"quit" => tok!(Ok(Chain::new(Filter::Action(Action::Quit), false))),
                _ => tok!(Err(Error::from_str(&format!(
                    "found unrecognized predicate {:?}",
                    token
                )))),
            }
        }
    }

    fn parse_operator(&mut self, depth: u8, rank: u8, mut chain: Chain) -> Result<Chain, Error> {
        chain = Chain::default().and(Filter::Chain(chain), false);

        while let Some(&token) = self.source.peek() {
            match token.to_ascii_lowercase().as_bytes() {
                b"and" => {
                    if rank > 3 {
                        break;
                    }
                    self.tokens.push(Token::Txt(token));
                    self.source.next();
                    chain = chain.and(Filter::Chain(self.parse_expr(depth, 4)?), false);
                }
                b"xor" => {
                    if rank > 2 {
                        break;
                    }
                    self.tokens.push(Token::Txt(token));
                    self.source.next();
                    chain = chain.xor(Filter::Chain(self.parse_expr(depth, 3)?), false);
                }
                b"or" => {
                    if rank > 1 {
                        break;
                    }
                    self.tokens.push(Token::Txt(token));
                    self.source.next();
                    chain = chain.or(Filter::Chain(self.parse_expr(depth, 2)?), false);
                }
                b"," => {
                    if rank > 0 {
                        break;
                    }
                    self.tokens.push(Token::Txt(token));
                    self.source.next();
                    chain = chain.yor(Filter::Chain(self.parse_expr(depth, 1)?), false);
                }
                b")" => {
                    if depth > 0 {
                        break;
                    }
                    self.tokens.push(Token::Rpr(token));
                    return Err(Error::from_str(r#"found unpair ")" in depth 0"#));
                }
                _ => {
                    if rank > 3 {
                        break;
                    }
                    // implicit "AND"
                    self.tokens.push(Token::Txt(OsStr::new("AND")));
                    chain = chain.and(Filter::Chain(self.parse_expr(depth, 4)?), false);
                }
            }
        }

        Ok(chain)
    }

    fn parse_file_type(&mut self) -> Result<Chain, Error> {
        self.next(None, "expected a file type").and_then(|token| {
            self.tokens.push(Token::Txt(token));

            // ((((F | A) | B) | C) | ...) = ~((((T & ~A) & ~B) & ~C) & ...)
            token
                .split_at_comma()
                .into_iter()
                .try_fold(Chain::default().not(), |chain, tok| {
                    Ok(chain.and(Filter::Type(FileType::from_str(tok)?), true))
                })
        })
    }

    fn parse_name_glob(&mut self, case_insensitive: bool) -> Result<Chain, Error> {
        self.next(None, "expected a glob pattern")
            .and_then(|token| {
                self.tokens.push(Token::Raw(token));

                PatternBuilder::new(token)
                    .use_regex(false)
                    .unicode(self.config.unicode)
                    .case_insensitive(case_insensitive)
                    .match_full_path(false)
                    .build()
                    .map(|pattern| Chain::new(Filter::Name(pattern), false))
                    .map_err(|err| {
                        Error::from_str(&format!(
                            "failed to build glob pattern {:?}:\n{}",
                            token, err
                        ))
                    })
            })
    }

    fn parse_path_glob(&mut self, case_insensitive: bool) -> Result<Chain, Error> {
        self.next(None, "expected a glob pattern")
            .and_then(|token| {
                self.tokens.push(Token::Raw(token));

                PatternBuilder::new(token)
                    .use_regex(false)
                    .unicode(self.config.unicode)
                    .case_insensitive(case_insensitive)
                    .match_full_path(true)
                    .build()
                    .map(|pattern| Chain::new(Filter::Path(pattern), false))
                    .map_err(|err| {
                        Error::from_str(&format!(
                            "failed to build glob pattern {:?}:\n{}",
                            token, err
                        ))
                    })
            })
    }

    fn parse_regex(&mut self, case_insensitive: bool) -> Result<Chain, Error> {
        self.next(None, "expected a regex pattern")
            .and_then(|token| {
                self.tokens.push(Token::Raw(token));

                PatternBuilder::new(token)
                    .use_regex(true)
                    .unicode(self.config.unicode)
                    .case_insensitive(case_insensitive)
                    .match_full_path(true)
                    .build()
                    .map(|pattern| Chain::new(Filter::Path(pattern), false))
                    .map_err(|err| {
                        Error::from_str(&format!(
                            "failed to build regex pattern {:?}:\n{}",
                            token, err
                        ))
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! args {
        ($($x:expr),*) => (<[&OsStr]>::into_vec(Box::new([$(OsStr::new($x)),*])));
        ($($x:expr,)*) => (args![$(OsStr::new($x)),*]);
    }

    macro_rules! calc {
        ($expected:expr, $args:expr) => {
            let errmsg = format!("Valid: {:?}", $args);
            let filter = Parser::new(&mut $args.into_iter(), Config { unicode: false })
                .parse()
                .unwrap_or_else(|err| panic!("\n{}\n{}\n", errmsg, err));
            let errmsg = format!("{}\nFilter: {:?}", errmsg, filter);
            assert_eq!($expected, filter.test_logic(), "\n{}\n", errmsg);
        };
    }

    macro_rules! fail {
        ($args:expr) => {
            let errmsg = format!("\nInvalid: {:?}\n", $args);
            Parser::new(&mut $args.into_iter(), Config { unicode: false })
                .parse()
                .expect_err(&errmsg);
        };
    }

    #[test]
    #[rustfmt::skip::macros(calc)]
    fn filter_parser() {
        calc!(true, args!["TRUE"]);
        calc!(false, args!["FALSE"]);
        calc!(false, args!["NOT", "TRUE"]);
        calc!(true, args!["NOT", "FALSE"]);
        calc!(true, args!["!", "FALSE"]);
        calc!(true, args!["!FALSE"]);
        calc!(false, args!["!!FALSE"]);
        calc!(false, args!["(", "FALSE", ")"]);
        calc!(true, args!["!(", "FALSE", ")"]);
        calc!(false, args!["NOT", "!FALSE"]);
        calc!(false, args!["TRUE", "FALSE"]);
        calc!(false, args!["TRUE", "AND", "FALSE"]);
        calc!(false, args!["NOT", "TRUE", "FALSE"]);
        calc!(false, args!["NOT", "TRUE", "AND", "FALSE"]);
        calc!(true, args!["TRUE", "XOR", "TRUE", "AND", "FALSE"]);
        calc!(true, args!["TRUE", "OR", "TRUE", "AND", "FALSE"]);
        calc!(true, args!["TRUE", "OR", "FALSE", "XOR", "TRUE"]);
        calc!(false, args!["TRUE", "OR", "TRUE", ",", "FALSE"]);
        calc!(false, args!["TRUE", "AND", "TRUE", "AND", "FALSE"]);
        calc!(true, args!["FALSE", "OR", "FALSE", "OR", "TRUE"]);
        calc!(true, args!["TRUE", "XOR", "TRUE", "XOR", "TRUE"]);
        calc!(false, args!["FALSE", ",", "TRUE", ",", "FALSE"]);
        calc!(false, args!["NOT", "(", "PRINT", "OR", "PRINT0", ")"]);
        calc!(true, args!["NOT", "NOT", "!(", "!(", "PRINT", ",", "PRINT0", ")", ")"]);

        fail!(args![]);
        fail!(args![""]);
        fail!(args!["?"]);
        fail!(args!["!"]);
        fail!(args!["("]);
        fail!(args![")"]);
        fail!(args!["(", ")"]);
        fail!(args!["NOT", "?"]);
        fail!(args!["!!"]);
        fail!(args!["!NOT"]);
        fail!(args!["!AND"]);
        fail!(args!["TRUE", "?", "FALSE"]);
        fail!(args!["(TRUE)"]);
        fail!(args!["(TRUE", ")"]);
        fail!(args!["(", "TRUE)"]);
        fail!(args!["(!", "TRUE", ")"]);
        fail!(args!["((", "TRUE", "))"]);
        fail!(args!["TRUE", "AND", "(", "FALSE"]);
        fail!(args!["TRUE", "AND", "(", "FALSE", ")", ")"]);
        fail!(args!["(", "TRUE", ")!", "TRUE"]);
    }
}
