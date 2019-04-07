use super::*;

// Operator in order of decreasing precedence:
// * "(" expression ")" , "NOT" expression , "!" expression
// * expression "AND" expression , expression expression
// * expression "XOR" expression
// * expression "OR"  expression
// * expression "," expression
// Operator names are case-insensitive.
//
// Expressions (unchained):
// * name <glob pattern>    # unaffected by the --case-insensitive flag
// * iname <glob pattern>
// * path <glob pattern>    # match the absolute/relative path, e.g. path '/*.rs'
// * ipath <glob pattern>
// * regex <regex pattern>  # match the absolute/relative path, e.g. regex '/.*\.rs$'
// * iregex <regex pattern>
// * type <file type[,file type]...>
// * true
// * false
// * print                  # unaffected by the --print0 flag
// * print0
// * ...
// The head of an expression is case-insensitive.

const MAX_RANK: usize = usize::max_value();

pub struct Config {
    pub unicode: bool,
}

pub struct Parser<'a, Iter: Iterator<Item = &'a OsStr>> {
    args: std::iter::Peekable<&'a mut Iter>,
    config: Config,
}

impl<'a, Iter: Iterator<Item = &'a OsStr>> Parser<'a, Iter> {
    pub fn new(args: &'a mut Iter, config: Config) -> Parser<'a, Iter> {
        Parser {
            args: args.peekable(),
            config,
        }
    }

    // TODO: better error message
    pub fn parse(&mut self) -> Result<Chain, Error> {
        self.parse_expr(0, 0)
    }

    fn next(&mut self, expected: Option<&[u8]>, errmsg: &str) -> Result<OsString, Error> {
        match (expected, self.args.next()) {
            (None, Some(token)) => Ok(token.into()),
            (Some(expected), Some(token)) if token.as_bytes() == expected => Ok(token.into()),
            _ => Err(Error::from_str(errmsg)),
        }
    }

    fn parse_expr(&mut self, depth: usize, rank: usize) -> Result<Chain, Error> {
        if let Some(token) = self.args.next() {
            self.parse_predicate(depth, token)
                .and_then(|chain| self.parse_operator(depth, rank, chain))
        } else {
            Err(Error::from_str("filter chain was incomplete"))
        }
    }

    fn parse_predicate(&mut self, depth: usize, token: &OsStr) -> Result<Chain, Error> {
        if token.len() > 1 && token.as_bytes().starts_with(b"!") {
            let token = OsStr::from_bytes(token.as_bytes().split_first().unwrap().1);

            self.parse_predicate(depth, token).map(|chain| chain.not())
        } else {
            match token.to_ascii_lowercase().as_bytes() {
                b"(" => self.parse_expr(depth + 1, 0).and_then(|chain| {
                    self.next(
                        Some(b")"),
                        &format!(r#"found unpaired "(" in depth {}"#, depth),
                    )
                    .and(Ok(chain))
                }),
                b"not" | b"!" => self.parse_expr(depth, MAX_RANK).map(|c| c.not()),
                b"type" => self.parse_file_type(),
                b"name" => self.parse_name_glob(false),
                b"iname" => self.parse_name_glob(true),
                b"path" => self.parse_path_glob(false),
                b"ipath" => self.parse_path_glob(true),
                b"regex" => self.parse_regex(false),
                b"iregex" => self.parse_regex(true),
                b"true" => Ok(Chain::new(Filter::Anything, false)),
                b"false" => Ok(Chain::new(Filter::Anything, true)),
                b"print" => Ok(Chain::new(Filter::Action(Action::Print), false)),
                b"print0" => Ok(Chain::new(Filter::Action(Action::Print0), false)),
                _ => Err(Error::from_str(&format!(
                    "found unrecognized predicate {:?}",
                    token
                ))),
            }
        }
    }

    fn parse_operator(
        &mut self,
        depth: usize,
        rank: usize,
        mut chain: Chain,
    ) -> Result<Chain, Error> {
        chain = Chain::default().and(Filter::Chain(chain), false);

        loop {
            match self.args.peek() {
                Some(&token) => match token.to_ascii_lowercase().as_bytes() {
                    b"and" => {
                        if rank > 3 {
                            break;
                        }
                        self.args.next();
                        chain = chain.and(Filter::Chain(self.parse_expr(depth, 3)?), false);
                    }
                    b"xor" => {
                        if rank > 2 {
                            break;
                        }
                        self.args.next();
                        chain = chain.xor(Filter::Chain(self.parse_expr(depth, 2)?), false);
                    }
                    b"or" => {
                        if rank > 1 {
                            break;
                        }
                        self.args.next();
                        chain = chain.or(Filter::Chain(self.parse_expr(depth, 1)?), false);
                    }
                    b"," => {
                        if rank > 0 {
                            break;
                        }
                        self.args.next();
                        chain = chain.yor(Filter::Chain(self.parse_expr(depth, 0)?), false);
                    }
                    b")" => {
                        if depth > 0 {
                            break;
                        }
                        Err(Error::from_str(&format!(
                            r#"found unpair ")" in depth {}"#,
                            depth
                        )))?;
                    }
                    _ => {
                        if rank > 3 {
                            break;
                        }
                        // implicit "AND"
                        chain = chain.and(Filter::Chain(self.parse_expr(depth, 3)?), false);
                    }
                },
                None => break,
            }
        }

        Ok(chain)
    }

    fn parse_file_type(&mut self) -> Result<Chain, Error> {
        self.next(None, "expected a file type").and_then(|token| {
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
                PatternBuilder::new(&token)
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
                PatternBuilder::new(&token)
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
                PatternBuilder::new(&token)
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
        ($($x:expr),*) => (<[_]>::into_vec(Box::new([$(OsStr::new($x)),*])));
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
        fail!(args!["((", "TRUE", "))"]);
        fail!(args!["TRUE", "AND", "(", "FALSE"]);
        fail!(args!["TRUE", "AND", "(", "FALSE", ")", ")"]);
    }
}
