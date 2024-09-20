use std::error::Error;
use std::rc::Rc;

use org_parser::{element::Heading, NodeID, Parser};

pub(crate) fn keyword_lookup<'a>(parser: &'a Parser, name: &'a str) -> Option<&'a str> {
    parser.keywords.get(name).copied()
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct Options {
    toc: Option<u8>,
}

impl Options {
    pub(crate) fn new(toc: Option<u8>) -> Self {
        Self { toc }
    }

    pub(crate) fn handle_opts(parser: &Parser) -> Result<Options, Box<dyn Error>> {
        // TODO: make keywords not case sensitive
        if let Some(options) = keyword_lookup(parser, "options") {
            let ret = options.split_ascii_whitespace();
            let mut toc = None;
            for optpair in ret {
                if let Some((opt, val)) = optpair.split_once(':') {
                    match opt {
                        "toc" => {
                            toc = if val == "nil" {
                                None
                            } else if let Ok(num) = val.parse::<u8>() {
                                Some(num)
                            } else {
                                Some(6)
                            };
                        }
                        _ => {}
                    }
                }
            }

            Ok(Options::new(toc))
        } else {
            Ok(Options::default())
        }
    }
}

#[derive(Debug)]
pub struct TocItem<'a> {
    pub name: &'a [NodeID],
    pub level: u8,
    pub target: Rc<str>,
    pub children: Vec<TocItem<'a>>,
}

pub(crate) fn process_toc<'a>(
    parser: &'a Parser,
    opts: &Options,
) -> Result<Vec<TocItem<'a>>, Box<dyn Error>> {
    let mut tocs: Vec<TocItem> = Vec::new();

    let Some(global_toc_level) = opts.toc else {
        return Err("shruge".into());
    };

    for sub_id in parser.pool[parser.pool.root_id()].obj.children().unwrap() {
        let node = &parser.pool[*sub_id];
        if let org_parser::Expr::Heading(heading) = &node.obj {
            if global_toc_level >= heading.heading_level.into() {
                if let Some(properties) = &heading.properties {
                    if let Some(val) = properties.get("unnumbered") {
                        if val == "notoc" {
                            continue;
                        }
                    }
                }
                tocs.push(handle_babies(
                    parser,
                    heading,
                    node.id_target.clone(),
                    global_toc_level,
                ));
            }
        }
    }

    // if let Some(toc_min) = opts.toc_min {
    //     let mut clean = false;
    //     while !clean {
    //         clean = true;
    //         for toc in tocs {
    //             if toc.level < toc_min {
    //                 clean = false;
    //             }
    //             for child in toc.children {
    //                 tocs.push(child);
    //             }
    //         }
    //     }
    // }

    Ok(tocs)
}

fn handle_babies<'a>(
    p: &'a Parser<'a>,
    heading: &'a Heading,
    target: Option<Rc<str>>,
    global_toc_level: u8,
) -> TocItem<'a> {
    let mut children_vec = Vec::new();
    if let Some(childs) = &heading.children {
        for child in childs {
            let node = &p.pool[*child];
            if let org_parser::Expr::Heading(heading) = &node.obj {
                if global_toc_level >= heading.heading_level.into() {
                    if let Some(properties) = &heading.properties {
                        if let Some(val) = properties.get("unnumbered") {
                            if val == "notoc" {
                                continue;
                            }
                        }
                    }
                    children_vec.push(handle_babies(
                        p,
                        &heading,
                        node.id_target.clone(),
                        global_toc_level,
                    ));
                }
            }
        }
    }

    TocItem {
        name: if let Some((_, node_ids)) = &heading.title {
            &node_ids
        } else {
            &[]
        },
        level: heading.heading_level.into(),
        target: if let Some(inner) = target {
            inner
        } else {
            "".into()
        },
        children: children_vec,
    }
}

#[cfg(test)]
mod tests {
    use crate::{ConfigOptions, Exporter, Html};

    use super::*;

    #[test]
    fn test_toc() -> Result<(), Box<dyn Error>> {
        //TODO: properly test
        let a = Html::export(
            r#"

#+begin_export html

<style>
ul {
  list-style-type: none;
}
</style>
#+end_export

*** swag
* abc
** love

*** 3


#+options: toc:t

"#,
            ConfigOptions::default(),
        )?;
        println!("{a}");
        Ok(())
    }
}
