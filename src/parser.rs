/*
    This is a prototyping parser for Woland.

    A. Reserved keywords.

      let   end
      true  false
      and   or    not
      if    then  elif else
      do    loop  for  while in do break
      match with
      type  actor

    B. Other tokens.

      : -> ~ =

    C. Grammar.

      prog  -> decl*

      decl  -> type | function | actor

      func  -> 'let' name ':' kind ('~' | '=') body

      kind  -> (name ':' name '->')* name

      body  -> instr* 'end'
            |  instr newline

      instr -> expr newline
            |  bind
            |  cond
            |  loop

      expr  -> prim
            |  name
            |  call

      prim  -> string | integer | boolean

      call  -> name expr*

      bind  -> 'let' 'mut'? name ':' name ('~' | '=') expr newline

      cond  -> 'if' expr 'then' body ('elif' exp 'then' body)* ('else' body)? 'end'

      loop  -> 'loop' expr 'do' body 'end'
*/

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until},
    character::complete::{
        alpha1, alphanumeric1, char, i64, multispace0, newline, none_of, space0,
    },
    combinator::{into, opt, recognize, value, verify},
    multi::{fold_many0, many0, many1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};

use crate::ast::*;

pub fn ast(input: &str) -> IResult<&str, AST> {
    let (input, decls) = many1(func)(input)?;
    Ok((
        input,
        AST {
            decls: decls.into_iter().collect(),
        },
    ))
}

pub fn func(input: &str) -> IResult<&str, (String, Decl)> {
    let (input, _) = ws(tag("let"))(input)?;
    let (input, name) = ws(name)(input)?;
    let (input, kind) = ws(kind)(input)?;
    let (input, _) = alt((ws(tag("~")), ws(tag("="))))(input)?;
    let (input, body) = many1(ws(instr))(input)?;
    let (input, _) = ws(tag("end"))(input)?;
    Ok((input, (name.to_string(), Decl::Func(Func { kind, body }))))
}

fn kind(input: &str) -> IResult<&str, Kind> {
    let (input, _) = ws(char(':'))(input)?;
    let (input, params) = many0(terminated(ws(name_typed), ws(tag("->"))))(input)?;
    let (input, ret) = ws(name)(input)?;
    Ok((
        input,
        Kind {
            params: params
                .iter()
                .map(|(s, t)| (s.to_string(), t.to_string()))
                .collect(),
            ret: ret.to_string(),
        },
    ))
}

fn instr(input: &str) -> IResult<&str, Instr> {
    let (input, result) = alt((
        terminated(into(expr), newline),
        ws(assign),
        ws(bind),
        into(ws(branch)),
        into(ws(loop_)),
        terminated(into(keyword), newline),
    ))(input)?;
    Ok((input, result))
}

impl From<Expr> for Instr {
    fn from(expr: Expr) -> Self {
        Instr::Expr(expr)
    }
}

impl From<Branch> for Instr {
    fn from(cond: Branch) -> Self {
        Instr::Branch(cond)
    }
}

impl From<Loop> for Instr {
    fn from(loop_: Loop) -> Self {
        Instr::Loop(loop_)
    }
}

impl From<Keyword> for Instr {
    fn from(word: Keyword) -> Self {
        Instr::Keyword(word)
    }
}

fn expr(input: &str) -> IResult<&str, Expr> {
    // None of the alt inputs show consume multispace!
    let (input, expr) = alt((
        delimited(
            ws(char('(')),
            alt((ws(prim), into(ws(name)), ws(call), ws(string))),
            char(')'),
        ),
        alt((prim, into(name), call, string)),
    ))(input)?;
    let (input, _) = space0(input)?;
    Ok((input, expr))
}

impl From<&str> for Expr {
    fn from(str: &str) -> Self {
        Expr::Name(str.to_string())
    }
}

fn prim(input: &str) -> IResult<&str, Expr> {
    // let (input, int) = i64(input)?;
    let (input, prim) = alt((
        into::<&str, bool, Expr, nom::error::Error<&str>, _, _>(alt((
            value(true, tag("true")),
            value(false, tag("false")),
        ))),
        into::<&str, i64, Expr, nom::error::Error<&str>, _, _>(i64),
    ))(input)?;
    Ok((input, prim))
}

impl From<i64> for Expr {
    fn from(int: i64) -> Self {
        Expr::Prim(Prim::I64(int))
    }
}

impl From<bool> for Expr {
    fn from(b: bool) -> Self {
        Expr::Prim(Prim::Bool(b))
    }
}

fn name(input: &str) -> IResult<&str, &str> {
    verify(
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
        )),
        |s: &str| {
            !vec![
                "let", "end", "true", "false", "if", "then", "else", "loop", "break",
            ]
            .contains(&s)
        },
    )(input)
}

fn name_typed(input: &str) -> IResult<&str, (&str, &str)> {
    delimited(
        opt(ws(char('('))),
        separated_pair(ws(name), ws(char(':')), name),
        opt(ws(char(')'))),
    )(input)
}

fn call(input: &str) -> IResult<&str, Expr> {
    // HACK: this ('@') is a temporary solution to be able
    // to identify function names without doing any
    // analysis and keeping this (protyping) parser
    // simply and happy. No language should ever do this!
    let (input, _) = char('@')(input)?;
    let (input, func) = name(input)?;
    let (input, _) = space0(input)?;
    let (input, args) = many0(terminated(expr, space0))(input)?;
    Ok((
        input,
        Expr::Call(Call {
            func_name: func.to_string(),
            args: args,
        }),
    ))
}

fn bind(input: &str) -> IResult<&str, Instr> {
    let (input, _) = ws(tag("let"))(input)?;
    let (input, mutspec) = opt(ws(tag("mut")))(input)?;
    let (input, (id, ty)) = ws(name_typed)(input)?;
    let (input, _) = alt((ws(tag("~")), ws(tag("="))))(input)?;
    let (input, expr) = expr(input)?;
    let (input, _) = newline(input)?;
    let bind = Bind {
        id: id.to_string(),
        ty: ty.to_string(),
        expr,
    };
    Ok((
        input,
        match mutspec {
            Some(_) => Instr::MutBind(bind),
            None => Instr::Bind(bind),
        },
    ))
}

fn assign(input: &str) -> IResult<&str, Instr> {
    let (input, name) = ws(name)(input)?;
    let (input, _) = alt((ws(tag("~")), ws(tag("="))))(input)?;
    let (input, expr) = expr(input)?;
    let (input, _) = newline(input)?;
    Ok((
        input,
        Instr::Assign(Assign {
            name: name.to_string(),
            expr,
        }),
    ))
}

fn branch(input: &str) -> IResult<&str, Branch> {
    let (input, head) = pair(
        preceded(ws(tag("if")), ws(expr)),
        preceded(ws(tag("then")), many1(ws(instr))),
    )(input)?;

    let (input, mut middle) = many0(pair(
        preceded(ws(tag("elsif")), ws(expr)),
        preceded(ws(tag("then")), many1(ws(instr))),
    ))(input)?;

    let (input, last) = opt(preceded(ws(tag("else")), many1(ws(instr))))(input)?;

    let (input, _) = ws(tag("end"))(input)?;

    let mut paths = vec![head];
    paths.append(&mut middle);
    if let Some(is) = last {
        paths.push((Expr::Prim(Prim::Bool(true)), is))
    }
    Ok((input, Branch { paths }))
}

fn loop_(input: &str) -> IResult<&str, Loop> {
    let (input, _) = ws(tag("loop"))(input)?;
    let (input, body) = many1(ws(instr))(input)?;
    let (input, _) = ws(tag("end"))(input)?;
    Ok((input, Loop { body }))
}

fn keyword(input: &str) -> IResult<&str, Keyword> {
    // let (input, keyword) = alt((
    // value(Keyword::Break, ws(tag("break"))),
    // value(Keyword::Whatever, ws(tag("whatever"))),
    // ))(input)?;
    let (input, keyword) = alt((
        value(Keyword::Break, tag("break")),
        value(Keyword::Ellipsis, tag("...")),
    ))(input)?;
    Ok((input, keyword))
}

fn string(input: &str) -> IResult<&str, Expr> {
    let (input, string) = delimited(
        char('"'),
        fold_many0(none_of("\""), String::new, |mut acc, ch| {
            acc.push(ch);
            acc
        }),
        char('"'),
    )(input)?;
    Ok((input, Expr::Prim(Prim::String(string))))
}

// fn ws<'a, F: 'a, O, E: ParseError<&'a str>>(
//     inner: F,
// ) -> impl FnMut(&'a str) -> IResult<&'a str, O, E>
// where
//     F: Fn(&'a str) -> IResult<&'a str, O, E>,
// {
//     terminated(inner, whitespace)
// }

fn ws<'a, F: 'a, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
where
    F: Fn(&'a str) -> IResult<&'a str, O>,
{
    terminated(inner, multispace0)
}

pub fn whitespace(i: &str) -> IResult<&str, ()> {
    value((), many0(alt((eol_comment, value((), multispace0)))))(i)
}

pub fn eol_comment(i: &str) -> IResult<&str, ()> {
    value(
        (), // Output is thrown away.
        pair(tag("//"), is_not("\n\r")),
    )(i)
}

pub fn inline_comment(i: &str) -> IResult<&str, ()> {
    value(
        (), // Output is thrown away.
        tuple((tag("/*"), take_until("*/"), tag("*/"))),
    )(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Prim::*;

    #[test]
    fn prim_i64() {
        assert_eq!(prim("42"), Ok(("", Expr::Prim(I64(42)))));
    }

    #[test]
    fn prim_bool() {
        assert_eq!(prim("true"), Ok(("", Expr::Prim(Bool(true)))));
        assert_eq!(prim("false"), Ok(("", Expr::Prim(Bool(false)))));
    }

    #[test]
    fn basic_call() {
        assert_eq!(
            instr("@dump 666 my_favourite_number\n"),
            Ok((
                "",
                Instr::Expr(Expr::Call(Call {
                    func_name: "dump".to_string(),
                    args: vec![
                        Expr::Prim(I64(666)),
                        Expr::Name("my_favourite_number".to_string())
                    ]
                }))
            ))
        );
    }

    #[test]
    fn basic_bind() {
        assert_eq!(
            bind("let number: I8 = 69\n"),
            Ok((
                "",
                Instr::Bind(Bind {
                    id: "number".to_string(),
                    ty: "I8".to_string(),
                    expr: Expr::Prim(I64(69))
                })
            ))
        )
    }

    #[test]
    fn basic_assign() {
        assert_eq!(
            assign("number = 69\n"),
            Ok((
                "",
                Instr::Assign(Assign {
                    name: "number".to_string(),
                    expr: Expr::Prim(I64(69))
                })
            ))
        )
    }

    #[test]
    fn id_with_type() {
        assert_eq!(name_typed("x: I64"), Ok(("", ("x", "I64"))));
    }

    #[test]
    fn kind_no_args() {
        assert_eq!(
            kind(": I64"),
            Ok((
                "",
                Kind {
                    params: vec![],
                    ret: "I64".to_string()
                }
            ))
        )
    }

    #[test]
    fn kind_two_args() {
        assert_eq!(
            kind(": (x: I64) -> (y: I64) -> I64"),
            Ok((
                "",
                Kind {
                    params: vec![
                        ("x".to_string(), "I64".to_string()),
                        ("y".to_string(), "I64".to_string())
                    ],
                    ret: "I64".to_string()
                }
            ))
        )
    }

    #[test]
    fn basic_func() {
        assert_eq!(
            func("let main: I32 ~\n    -1\n end"),
            Ok((
                "",
                (
                    "main".to_string(),
                    Decl::Func(Func {
                        kind: Kind {
                            params: vec![],
                            ret: "I32".to_string()
                        },
                        body: vec![Instr::Expr(Expr::Prim(I64(-1)))],
                    })
                )
            ))
        )
    }

    #[test]
    fn ast_two_funcs() {
        assert_eq!(
            ast("let whatever: I8 =\n   0\n end\n\nlet main: I64 ~\n   -1\n end"),
            Ok((
                "",
                AST {
                    decls: vec![
                        (
                            "whatever".to_string(),
                            Decl::Func(Func {
                                kind: Kind {
                                    params: vec![],
                                    ret: "I8".to_string()
                                },
                                body: vec![Instr::Expr(Expr::Prim(I64(0)))],
                            })
                        ),
                        (
                            "main".to_string(),
                            Decl::Func(Func {
                                kind: Kind {
                                    params: vec![],
                                    ret: "I64".to_string()
                                },
                                body: vec![Instr::Expr(Expr::Prim(I64(-1)))],
                            })
                        ),
                    ]
                    .into_iter()
                    .collect()
                }
            ))
        );
    }

    #[test]
    fn func_with_call() {
        assert_eq!(
            ast("let nothing: Void ~\n   @dump 2021\n end"),
            Ok((
                "",
                (AST {
                    decls: vec![(
                        "nothing".to_string(),
                        Decl::Func(Func {
                            kind: Kind {
                                params: vec![],
                                ret: "Void".to_string()
                            },
                            body: vec![Instr::Expr(Expr::Call(Call {
                                func_name: "dump".to_string(),
                                args: vec![Expr::Prim(I64(2021))]
                            }))],
                        })
                    )]
                    .into_iter()
                    .collect()
                })
            ))
        );
    }

    #[test]
    fn func_with_bind() {
        assert_eq!(
            ast("let main: Void ~\n  let number: I8 = 42\n end"),
            Ok((
                "",
                AST {
                    decls: vec![(
                        "main".to_string(),
                        Decl::Func(Func {
                            kind: Kind {
                                params: vec![],
                                ret: "Void".to_string()
                            },
                            body: vec![Instr::Bind(Bind {
                                id: "number".to_string(),
                                ty: "I8".to_string(),
                                expr: Expr::Prim(I64(42))
                            })],
                        })
                    )]
                    .into_iter()
                    .collect()
                }
            ))
        );
    }

    #[test]
    fn func_with_cond() {
        assert_eq!(
            ast("let main: Void ~\n if condition1 then\n @dump 1\n elif condition2 then\n @dump 2\n else\n @dump 0\n end\n end"),
            Ok((
                "",
                AST {
                    decls: vec![(
                        "main".to_string(),
                        Decl::Func(Func {
                            kind: Kind {
                                params: vec![],
                                ret: "Void".to_string()
                            },
                            body: vec![Instr::Branch(Branch {
                                paths: vec![
                                    (
                                        Expr::Name("condition1".to_string()),
                                        vec![Instr::Expr(Expr::Call(Call {
                                            func_name: "dump".to_string(),
                                            args: vec![Expr::Prim(I64(1))]
                                        }))]
                                    ),
                                    (
                                        Expr::Name("condition2".to_string()),
                                        vec![Instr::Expr(Expr::Call(Call {
                                            func_name: "dump".to_string(),
                                            args: vec![Expr::Prim(I64(2))]
                                        }))]
                                    ),
                                    (
                                        Expr::Prim(Bool(true)),
                                        vec![Instr::Expr(Expr::Call(Call {
                                            func_name: "dump".to_string(),
                                            args: vec![Expr::Prim(I64(0))]
                                        }))]
                                    ),

                                ]
                            })]
                        })
                    )]
                    .into_iter()
                    .collect()
                }
            ))
        );
    }

    #[test]
    fn basic_loop() {
        assert_eq!(
            loop_("loop\n @dump 42\n end"),
            Ok((
                "",
                Loop {
                    body: vec![Instr::Expr(Expr::Call(Call {
                        func_name: "dump".to_string(),
                        args: vec![Expr::Prim(I64(42))]
                    }))]
                }
            ))
        )
    }
}