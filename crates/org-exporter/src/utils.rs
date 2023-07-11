use org_parser::Parser;

pub(crate) fn keyword_lookup<'a>(parser: &'a Parser, name: &'a str) -> Option<&'a str> {
    parser.keywords.get(name).copied()
}
