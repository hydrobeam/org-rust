use org_parser::Parser;
use std::path::Path;

#[derive(Debug)]
enum Command<'a> {
    If(&'a str),
    Endif,
    Else,
}

impl<'a> Command<'a> {
    fn check(mut extract: &'a str) -> Option<Self> {
        extract = extract.trim();
        if let Some(ind) = extract.find(" ") {
            let (command, value) = extract.split_at(ind);
            match command {
                "if" => Some(Command::If(value.trim())),
                _ => {
                    eprintln!("invalid command: {}", command);
                    None
                }
            }
        } else {
            match extract {
                "else" => Some(Command::Else),
                "endif" => Some(Command::Endif),
                _ => None,
            }
        }
    }
}
use crate::types::CliError;

#[derive(Clone, Copy, Debug)]
enum LogicItem {
    If,
    Else,
    None,
}

pub struct Template<'a, 'b> {
    p: &'a Parser<'a>,
    captures: Vec<(usize, usize, &'b str)>,
    template_path: &'a Path,
    template_contents: &'b str,
    exported_content: &'a str,
    i: usize,
    end: usize,
}

impl<'a, 'b> Template<'a, 'b> {
    pub fn form_template(
        p: &'a Parser,
        template_path: &'a Path,
        template_contents: &'b str,
        exported_content: &'a str,
    ) -> Result<Self, CliError> {
        // the regex is checked at compile time and won't exceed the size limits + is valid
        let re = regex::Regex::new(r#"\{\{\{(.*)\}\}\}"#).unwrap();
        // collect all matches to {{{.*}}} regex - things we want to replace with keywords
        let captures = re
            .captures_iter(&template_contents)
            .map(|capture| {
                let mtch = capture.get(1).unwrap();
                // we expand the range of the capture to include the {{{}}}
                (mtch.start() - 3, mtch.end() + 3, mtch.as_str().trim())
            })
            .collect::<Vec<(usize, usize, &str)>>();
        Ok(Self {
            p,
            captures,
            template_path,
            template_contents,
            exported_content,
            i: 0,
            end: 0,
        })
    }
    pub fn process(&mut self) -> Result<String, CliError> {
        self.process_captures(0, LogicItem::None).map(|mut v| {
            v.push_str(
                &self.template_contents
                    [self.captures.last().unwrap().1..self.template_contents.len()],
            );
            v
        })
    }

    fn process_captures(&mut self, mut begin: usize, l: LogicItem) -> Result<String, CliError> {
        let mut local_items: String = String::new();

        while self.i < self.captures.len() {
            let (start, end, extract) = self.captures[self.i];
            self.end = end;
            local_items.push_str(&self.template_contents[begin..start]);

            if extract == "content" {
                local_items.push_str(&self.exported_content);
            } else if let Some(c) = Command::check(extract) {
                match c {
                    Command::If(cond) => {
                        self.i += 1;
                        if let Some(_) = self.p.keywords.get(&*cond) {
                            local_items.push_str(&self.process_captures(self.end, LogicItem::If)?);
                        } else {
                            // skip till else/endif
                            let mut if_count = 0;
                            while let (_, end2, extract2) = self.captures.get(self.i).ok_or(
                                CliError::new()
                                    .with_path(self.template_path)
                                    .with_cause("Unterminated if block in template"),
                            )? {
                                if let Some(c) = Command::check(extract2) {
                                    match c {
                                        Command::If(_) => {
                                            if_count += 1;
                                        }
                                        Command::Endif => {
                                            if if_count == 0 {
                                                self.end = *end2;
                                                break;
                                            } else {
                                                if_count -= -1;
                                            }
                                        }
                                        Command::Else => {
                                            if if_count == 0 {
                                                self.i += 1;
                                                // parse else
                                                local_items.push_str(
                                                    &self
                                                        .process_captures(*end2, LogicItem::Else)?,
                                                );
                                                break;
                                            }
                                        }
                                    }
                                }
                                self.i += 1;
                            }
                        };
                    }
                    Command::Else => {
                        self.i += 1;
                        if matches!(l, LogicItem::If) {
                            let mut if_count = 0;
                            while let (_, end2, extract2) = self.captures.get(self.i).ok_or(
                                CliError::new()
                                    .with_path(self.template_path)
                                    .with_cause("Unterminated else block in template"),
                            )? {
                                if let Some(c) = Command::check(extract2) {
                                    match c {
                                        Command::If(_) => {
                                            if_count += 1;
                                        }
                                        Command::Endif => {
                                            if if_count == 0 {
                                                self.end = *end2;
                                                return Ok(local_items);
                                            } else {
                                                if_count -= -1;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                self.i += 1;
                            }
                        }

                        if !matches!(l, LogicItem::Else) {
                            Err(CliError::new()
                                .with_path(self.template_path)
                                .with_cause("Unexpected else block in template"))?
                        }

                        local_items.push_str(&self.process_captures(self.end, LogicItem::Else)?);
                    }
                    Command::Endif => {
                        if matches!(l, LogicItem::Else | LogicItem::If) {
                            self.end = end;
                            return Ok(local_items);
                        } else {
                            Err(CliError::new()
                                .with_path(self.template_path)
                                .with_cause("Unexpected endif block in template"))?
                        }
                    }
                }
            } else if let Some(ind) = extract.find("|") {
                let (l, r) = extract.split_at(ind);
                local_items.push_str(if let Some(val) = self.p.keywords.get(l) {
                    val
                } else {
                    r
                })
            } else if let Some(kw) = self.p.keywords.get(extract) {
                local_items.push_str(kw);
            }

            self.i += 1;
            begin = self.end;
        }

        if !matches!(l, LogicItem::None) {
            Err(CliError::new()
                .with_path(self.template_path)
                .with_cause("Unterminated conditional block in template"))?
        } else {
            Ok(local_items)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_template() {}
}
