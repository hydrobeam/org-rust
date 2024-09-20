use crate::{types::CliError, utils::relative_path_from};
use org_parser::Parser;
use std::{fs::read_to_string, path::Path};

#[derive(Debug)]
enum Command<'a> {
    If(&'a str),
    Endif,
    Else,
    Include(&'a str),
}

impl<'a> Command<'a> {
    fn check(mut extract: &'a str) -> Option<Self> {
        extract = extract.trim();
        if let Some(ind) = extract.find(" ") {
            let (command, value) = extract.split_at(ind);
            let value = value.trim();
            match command {
                "if" => Some(Command::If(value)),
                "include" => Some(Command::Include(value)),
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

#[derive(Clone, Copy, Debug)]
enum LogicItem {
    If,
    Else,
    None,
}

pub struct Template<'a, 'template> {
    p: &'a Parser<'a>,
    template_path: &'a Path,
    template_contents: &'template str,
    exported_content: &'a str,
    end: usize,
}

impl<'a, 'template> Template<'a, 'template> {
    pub fn new(
        p: &'a Parser,
        template_path: &'a Path,
        template_contents: &'template str,
        exported_content: &'a str,
    ) -> Self {
        Self {
            p,
            template_path,
            template_contents,
            exported_content,
            end: 0,
        }
    }
    pub fn process(&mut self) -> Result<String, CliError> {
        // the regex is checked at compile time and won't exceed the size limits + is valid
        // ? => not greedy
        let re = regex::Regex::new(r#"\{\{\{(.*?)\}\}\}"#).unwrap();
        // collect all matches to {{{.*}}} regex - things we want to replace with keywords
        let mut captures = re.captures_iter(&self.template_contents).map(|capture| {
            let mtch = capture.get(1).unwrap();
            // we expand the range of the capture to include the {{{}}}
            (mtch.start() - 3, mtch.end() + 3, mtch.as_str().trim())
        });

        self.process_captures(&mut captures, 0, LogicItem::None)
    }

    fn process_captures(
        &mut self,
        c: &mut impl Iterator<Item = (usize, usize, &'template str)>,
        mut begin: usize,
        l: LogicItem,
    ) -> Result<String, CliError> {
        // building string to hold the processed template output
        let mut local_items: String = String::new();

        while let Some((start, end, extract)) = c.next() {
            self.end = end;
            local_items.push_str(&self.template_contents[begin..start]);

            if extract == "content" {
                local_items.push_str(&self.exported_content);
            } else if let Some(command) = Command::check(extract) {
                match command {
                    Command::If(cond) => {
                        if let Some(_) = self.p.keywords.get(&*cond) {
                            local_items.push_str(&self.process_captures(
                                c,
                                self.end,
                                LogicItem::If,
                            )?);
                        } else {
                            // skip till else/endif
                            // if an if is encountered, then it just increases the number of endifs we have to see.
                            let mut if_count = 0;
                            while let (_, end2, extract2) = c.next().ok_or(
                                CliError::new()
                                    .with_path(self.template_path)
                                    .with_cause("Unterminated if block in template"),
                            )? {
                                if let Some(command2) = Command::check(extract2) {
                                    match command2 {
                                        Command::If(_) => {
                                            if_count += 1;
                                        }
                                        Command::Endif => {
                                            if if_count == 0 {
                                                self.end = end2;
                                                break;
                                            } else {
                                                if_count -= -1;
                                            }
                                        }
                                        Command::Else => {
                                            if if_count == 0 {
                                                // parse else
                                                local_items.push_str(&self.process_captures(
                                                    c,
                                                    end2,
                                                    LogicItem::Else,
                                                )?);
                                                break;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        };
                    }
                    Command::Else => {
                        if matches!(l, LogicItem::If) {
                            let mut if_count = 0;
                            while let (_, end2, extract2) = c.next().ok_or(
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
                                                self.end = end2;
                                                return Ok(local_items);
                                            } else {
                                                if_count -= -1;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }

                        if !matches!(l, LogicItem::Else) {
                            Err(CliError::new()
                                .with_path(self.template_path)
                                .with_cause("Unexpected else block in template"))?
                        }

                        local_items.push_str(&self.process_captures(
                            c,
                            self.end,
                            LogicItem::Else,
                        )?);
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
                    Command::Include(file) => {
                        let include_path =
                            relative_path_from(&self.template_path, Path::new(file))?;
                        let included_template = read_to_string(&include_path).map_err(|e| {
                            CliError::from(e)
                                .with_path(&include_path)
                                .with_cause(&format!(
                                    "failed to read included file within template file: {}",
                                    self.template_path.display()
                                ))
                        })?;
                        let mut t = Template::new(
                            self.p,
                            &include_path,
                            &included_template,
                            self.exported_content,
                        );
                        local_items.push_str(&t.process()?);
                    }
                }
            } else if let Some(ind) = extract.find("|") {
                let (l, r) = extract.split_at(ind);
                local_items.push_str(if let Some(val) = self.p.keywords.get(l) {
                    val
                } else {
                    //  the split resutls in "abc|123" being split like l = abc, r = |123
                    //  so we toss the first byte :>
                    &r[1..]
                })
            } else if let Some(kw) = self.p.keywords.get(extract) {
                local_items.push_str(kw);
            }

            begin = self.end;
        }

        if !matches!(l, LogicItem::None) {
            Err(CliError::new()
                .with_path(self.template_path)
                .with_cause("Unterminated conditional block in template"))?
        } else {
            // we only process till the last template, not till the end of the file.
            // fill in the remainder of the template here.
            let last_chunk = &self.template_contents[self.end..self.template_contents.len()];
            local_items.push_str(last_chunk);
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
