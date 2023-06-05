use org_parser::types::Parser;

pub(crate) fn keyword_lookup<'a>(parser: &'a Parser, name: &'a str) -> Option<&'a str> {
    parser.keywords.get(name).copied()
}
