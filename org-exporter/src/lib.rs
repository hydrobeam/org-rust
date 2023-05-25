use std::fmt::{Result, Write};

use org_parser::element::{BlockContents, BulletKind, CounterKind, Priority, TableRow, Tag};
use org_parser::node_pool::{NodeID, NodePool};
use org_parser::object::LatexFragment;
use org_parser::parse_org;
use org_parser::types::Expr;

pub fn export_org<T: Write>(input: &str, out: &mut T) -> Result {
    let pool = parse_org(input);
    export_org_rec(&pool.root().obj, &pool, out)?;
    Ok(())
}

fn export_org_rec<T: Write>(node: &Expr, pool: &NodePool, buf: &mut T) -> Result {
    match node {
        Expr::Root(inner) => {
            for id in inner {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
        }
        Expr::Heading(inner) => {
            for _ in 0..inner.heading_level.into() {
                write!(buf, "*")?;
            }
            write!(buf, " ")?;

            if let Some(keyword) = inner.keyword {
                write!(buf, "{keyword} ")?;
            }

            if let Some(priority) = &inner.priority {
                write!(buf, "[#")?;
                match priority {
                    Priority::A => write!(buf, "A")?,
                    Priority::B => write!(buf, "B")?,
                    Priority::C => write!(buf, "C")?,
                    Priority::Num(num) => write!(buf, "{}", num)?,
                };
                write!(buf, "] ")?;
            }

            if let Some(title) = &inner.title {
                for id in title {
                    export_org_rec(&pool[*id].obj, pool, buf)?;
                }
            }

            fn tag_search<T: Write>(loc: NodeID, pool: &NodePool, buf: &mut T) -> Result {
                if let Expr::Heading(loc) = &pool[loc].obj {
                    if let Some(sub_tags) = loc.tags.as_ref() {
                        for thang in sub_tags.iter().rev() {
                            match thang {
                                Tag::Raw(val) => write!(buf, ":{val}")?,
                                Tag::Loc(id) => {
                                    tag_search(*id, pool, buf)?;
                                }
                            }
                        }
                    }
                }
                Ok(())
            }

            if let Some(tags) = &inner.tags {
                let mut valid_out = String::new();
                for tag in tags.iter().rev() {
                    match tag {
                        Tag::Raw(val) => write!(valid_out, ":{val}")?,
                        Tag::Loc(id) => {

                            // tag_search(*id, pool, &mut valid_out)?;
                        }
                    }
                }
                // handles the case where a parent heading has no tags
                if !valid_out.is_empty() {
                    write!(buf, " {valid_out}:")?;
                }
            }

            writeln!(buf)?;

            if let Some(children) = &inner.children {
                for id in children {
                    export_org_rec(&pool[*id].obj, pool, buf)?;
                }
            }
        }
        Expr::Block(inner) => {
            let val: &str = inner.kind.into();
            write!(buf, "#+begin_{val}")?;
            if let Some(params) = inner.parameters {
                write!(buf, " {}", params)?;
            }
            write!(buf, "\n")?;
            match &inner.contents {
                BlockContents::Greater(children) => {
                    for id in children {
                        export_org_rec(&pool[*id].obj, pool, buf)?;
                    }
                    writeln!(buf)?;
                }
                BlockContents::Lesser(cont) => {
                    writeln!(buf, "{cont}")?;
                }
            }
            write!(buf, "#+end_{val}\n")?;
        }
        Expr::RegularLink(inner) => {
            write!(buf, "[")?;
            write!(buf, "[{}]", inner.path)?;
            if let Some(children) = &inner.description {
                write!(buf, "[")?;
                for id in children {
                    export_org_rec(&pool[*id].obj, pool, buf)?;
                }
                write!(buf, "]")?;
            }
            write!(buf, "]")?;
        }

        Expr::Paragraph(inner) => {
            for id in &inner.0 {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
            writeln!(buf)?;
        }

        Expr::Italic(inner) => {
            write!(buf, "/")?;
            for id in &inner.0 {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
            write!(buf, "/")?;
        }
        Expr::Bold(inner) => {
            write!(buf, "*")?;
            for id in &inner.0 {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
            write!(buf, "*")?;
        }
        Expr::StrikeThrough(inner) => {
            write!(buf, "+")?;
            for id in &inner.0 {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
            write!(buf, "+")?;
        }
        Expr::Underline(inner) => {
            write!(buf, "_")?;
            for id in &inner.0 {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
            write!(buf, "_")?;
        }
        Expr::BlankLine => {
            writeln!(buf)?;
        }
        Expr::SoftBreak => {
            write!(buf, " ")?;
        }
        Expr::Plain(inner) => {
            write!(buf, "{inner}")?;
        }
        Expr::MarkupEnd(inner) => {
            unreachable!()
        }
        Expr::Verbatim(inner) => {
            write!(buf, "=")?;
            write!(buf, "{}", inner.0)?;
            write!(buf, "=")?;
        }
        Expr::Code(inner) => {
            write!(buf, "~")?;
            write!(buf, "{}", inner.0)?;
            write!(buf, "~")?;
        }
        Expr::Comment(inner) => {
            writeln!(buf, "# {}", inner.0)?;
        }
        Expr::InlineSrc(inner) => {
            write!(buf, "src_{}", inner.lang)?;
            if let Some(args) = inner.headers {
                write!(buf, "[{args}]")?;
            }
            write!(buf, "{{{}}}", inner.body)?;
        }
        Expr::Keyword(inner) => {
            writeln!(buf, "#+{}: {}", inner.key, inner.val)?;
        }
        Expr::LatexEnv(inner) => {
            write!(
                buf,
                "\\begin{{{0}}}\n{1}\n\\end{{{0}}}\n",
                inner.name, inner.contents
            )?;
        }
        Expr::LatexFragment(inner) => match inner {
            LatexFragment::Command { name, contents } => {
                write!(buf, "\\{name}")?;
                if let Some(command_cont) = contents {
                    write!(buf, "{{{command_cont}}}")?;
                }
            }
            LatexFragment::Display(inner) => {
                write!(buf, "\\[{inner}\\]")?;
            }
            LatexFragment::Inline(inner) => {
                write!(buf, "\\({inner}\\)")?;
            }
        },
        Expr::Item(inner) => {
            match inner.bullet {
                BulletKind::Unordered => {
                    write!(buf, "-")?;
                }
                BulletKind::Ordered(counterkind) => match counterkind {
                    CounterKind::Letter(lettre) => {
                        write!(buf, "{}", lettre as char)?;
                    }
                    CounterKind::Number(num) => {
                        write!(buf, "{num}")?;
                    }
                },
            }
            write!(buf, " ")?;

            if let Some(check) = &inner.check_box {
                let val: &str = check.into();
                write!(buf, "[{val}] ")?;
            }

            if let Some(tag) = inner.tag {
                write!(buf, "{tag} :: ")?;
            }

            for id in &inner.children {
                export_org_rec(&pool[*id].obj, pool, buf)?;
                write!(buf, "  ")?;
            }
        }
        Expr::PlainList(inner) => {
            for id in &inner.children {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
        }
        Expr::PlainLink(inner) => {
            write!(buf, "{}:{}", inner.protocol, inner.path)?;
        }
        Expr::Entity(inner) => {
            write!(buf, "{}", inner.mapped_item)?;
        }
        Expr::Table(inner) => {
            for id in &inner.children {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
        }

        Expr::TableRow(inner) => {
            match inner {
                TableRow::Standard(stans) => {
                    write!(buf, "|")?;
                    for id in stans {
                        export_org_rec(&pool[*id].obj, pool, buf)?;
                    }
                }
                TableRow::Rule => {
                    // TODO: figure out alignment
                    write!(buf, "|-")?;
                }
            }
            writeln!(buf)?;
        }
        Expr::TableCell(inner) => {
            for id in &inner.0 {
                export_org_rec(&pool[*id].obj, pool, buf)?;
            }
            write!(buf, "|")?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_org_export() -> Result {
        let mut out_str = String::new();
        export_org(
            r"** one two
three
*four*

",
            &mut out_str,
        )?;

        assert_eq!(
            out_str,
            r"** one two
three *four*

"
        );
        Ok(())
    }

    //     #[test]
    //     fn test_list_export() -> Result {
    //         let mut out = String::new();
    //         export_org(
    //             r"
    // + one two three
    // four five six

    //    + two
    // + three
    // + four
    // +five
    // ",
    //             &mut out,
    //         )?;

    //         println!("{out}");
    //         Ok(())
    //     }

    #[test]
    fn test_link_export() -> Result {
        let mut out = String::new();
        export_org("[[https://swag.org][meowww]]", &mut out)?;

        println!("{out}");
        Ok(())
    }

    #[test]
    fn test_beeg() -> Result {
        let mut out = String::new();

        export_org(
            r"* DONE [#0] *one* two /three/ /four*       :one:two:three:four:
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
** [#1] descendant headline :five:
*** [#2] inherit the tags
** [#3] different level
subcontent
this

is a different paragraph
id) =
more subcontent

* [#4] separate andy
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
** a
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
* a
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
more content here this is a pargraph
",
            &mut out,
        )?;

        println!("{out}");
        Ok(())
    }

    #[test]
    fn less() -> Result {
        let mut out = String::new();
        export_org(
            r"* [#1] abc :c:
** [#1] descendant headline :a:b:
*** [#2] inherit the tags
** [#3] different level
",
            &mut out,
        )?;

        assert_eq!(
            out,
            r"* [#1] abc :c:
** [#1] descendant headline :a:b:
*** [#2] inherit the tags
** [#3] different level
"
        );
        Ok(())
    }
}
