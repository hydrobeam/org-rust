use crate::types::{MatchError, Result};
use phf::phf_map;

static ENTITY_MAP: phf::Map<&'static str, &'static str> = phf_map! {
"Agrave"         => r#"À"#,
"agrave"         => r#"à"#,
"Aacute"         => r#"Á"#,
"aacute"         => r#"á"#,
"Acirc"          => r#"Â"#,
"acirc"          => r#"â"#,
"Amacr"          => r#"Ā"#,
"amacr"          => r#"ā"#,
"Atilde"         => r#"Ã"#,
"atilde"         => r#"ã"#,
"Auml"           => r#"Ä"#,
"auml"           => r#"ä"#,
"Aring"          => r#"Å"#,
"AA"             => r#"Å"#,
"aring"          => r#"å"#,
"AElig"          => r#"Æ"#,
"aelig"          => r#"æ"#,
"Ccedil"         => r#"Ç"#,
"ccedil"         => r#"ç"#,
"Egrave"         => r#"È"#,
"egrave"         => r#"è"#,
"Eacute"         => r#"É"#,
"eacute"         => r#"é"#,
"Ecirc"          => r#"Ê"#,
"ecirc"          => r#"ê"#,
"Euml"           => r#"Ë"#,
"euml"           => r#"ë"#,
"Igrave"         => r#"Ì"#,
"igrave"         => r#"ì"#,
"Iacute"         => r#"Í"#,
"iacute"         => r#"í"#,
"Idot"           => r#"İ"#,
"inodot"         => r#"ı"#,
"Icirc"          => r#"Î"#,
"icirc"          => r#"î"#,
"Iuml"           => r#"Ï"#,
"iuml"           => r#"ï"#,
"Ntilde"         => r#"Ñ"#,
"ntilde"         => r#"ñ"#,
"Ograve"         => r#"Ò"#,
"ograve"         => r#"ò"#,
"Oacute"         => r#"Ó"#,
"oacute"         => r#"ó"#,
"Ocirc"          => r#"Ô"#,
"ocirc"          => r#"ô"#,
"Otilde"         => r#"Õ"#,
"otilde"         => r#"õ"#,
"Ouml"           => r#"Ö"#,
"ouml"           => r#"ö"#,
"Oslash"         => r#"Ø"#,
"oslash"         => r#"ø"#,
"OElig"          => r#"Œ"#,
"oelig"          => r#"œ"#,
"Scaron"         => r#"Š"#,
"scaron"         => r#"š"#,
"szlig"          => r#"ß"#,
"Ugrave"         => r#"Ù"#,
"ugrave"         => r#"ù"#,
"Uacute"         => r#"Ú"#,
"uacute"         => r#"ú"#,
"Ucirc"          => r#"Û"#,
"ucirc"          => r#"û"#,
"Uuml"           => r#"Ü"#,
"uuml"           => r#"ü"#,
"Yacute"         => r#"Ý"#,
"yacute"         => r#"ý"#,
"Yuml"           => r#"Ÿ"#,
"yuml"           => r#"ÿ"#,
"fnof"           => r#"ƒ"#,
"real"           => r#"ℜ"#,
"image"          => r#"ℑ"#,
"weierp"         => r#"℘"#,
"ell"            => r#"ℓ"#,
"imath"          => r#"ı"#,
"jmath"          => r#"ȷ"#,
"Alpha"          => r#"Α"#,
"alpha"          => r#"α"#,
"Beta"           => r#"Β"#,
"beta"           => r#"β"#,
"Gamma"          => r#"Γ"#,
"gamma"          => r#"γ"#,
"Delta"          => r#"Δ"#,
"delta"          => r#"δ"#,
"Epsilon"        => r#"Ε"#,
"epsilon"        => r#"ε"#,
"varepsilon"     => r#"ε"#,
"Zeta"           => r#"Ζ"#,
"zeta"           => r#"ζ"#,
"Eta"            => r#"Η"#,
"eta"            => r#"η"#,
"Theta"          => r#"Θ"#,
"theta"          => r#"θ"#,
"thetasym"       => r#"ϑ"#,
"vartheta"       => r#"ϑ"#,
"Iota"           => r#"Ι"#,
"iota"           => r#"ι"#,
"Kappa"          => r#"Κ"#,
"kappa"          => r#"κ"#,
"Lambda"         => r#"Λ"#,
"lambda"         => r#"λ"#,
"Mu"             => r#"Μ"#,
"mu"             => r#"μ"#,
"nu"             => r#"ν"#,
"Nu"             => r#"Ν"#,
"Xi"             => r#"Ξ"#,
"xi"             => r#"ξ"#,
"Omicron"        => r#"Ο"#,
"omicron"        => r#"ο"#,
"Pi"             => r#"Π"#,
"pi"             => r#"π"#,
"Rho"            => r#"Ρ"#,
"rho"            => r#"ρ"#,
"Sigma"          => r#"Σ"#,
"sigma"          => r#"σ"#,
"sigmaf"         => r#"ς"#,
"varsigma"       => r#"ς"#,
"Tau"            => r#"Τ"#,
"Upsilon"        => r#"Υ"#,
"upsih"          => r#"ϒ"#,
"upsilon"        => r#"υ"#,
"Phi"            => r#"Φ"#,
"phi"            => r#"φ"#,
"varphi"         => r#"ϕ"#,
"Chi"            => r#"Χ"#,
"chi"            => r#"χ"#,
"acutex"         => r#"´x"#,
"Psi"            => r#"Ψ"#,
"psi"            => r#"ψ"#,
"tau"            => r#"τ"#,
"Omega"          => r#"Ω"#,
"omega"          => r#"ω"#,
"piv"            => r#"ϖ"#,
"varpi"          => r#"ϖ"#,
"partial"        => r#"∂"#,
"alefsym"        => r#"ℵ"#,
"aleph"          => r#"ℵ"#,
"gimel"          => r#"ℷ"#,
"beth"           => r#"ℶ"#,
"dalet"          => r#"ℸ"#,
"ETH"            => r#"Ð"#,
"eth"            => r#"ð"#,
"THORN"          => r#"Þ"#,
"thorn"          => r#"þ"#,
"dots"           => r#"…"#,
"cdots"          => r#"⋯"#,
"hellip"         => r#"…"#,
"middot"         => r#"·"#,
"iexcl"          => r#"¡"#,
"iquest"         => r#"¿"#,
"shy"            => r#"\u{AD}"#,
"ndash"          => r#"–"#,
"mdash"          => r#"—"#,
"quot"           => r#"""#,
"acute"          => r#"´"#,
"ldquo"          => r#"“"#,
"rdquo"          => r#"”"#,
"bdquo"          => r#"„"#,
"lsquo"          => r#"‘"#,
"rsquo"          => r#"’"#,
"sbquo"          => r#"‚"#,
"laquo"          => r#"«"#,
"raquo"          => r#"»"#,
"lsaquo"         => r#"‹"#,
"rsaquo"         => r#"›"#,
"circ"           => r#"ˆ"#,
"vert"           => r#"|"#,
"vbar"           => r#"|"#,
"brvbar"         => r#"¦"#,
"S"              => r#"§"#,
"sect"           => r#"§"#,
"amp"            => r#"&"#,
"lt"             => r#"<"#,
"gt"             => r#">"#,
"tilde"          => r#"~"#,
"slash"          => r#"/"#,
"plus"           => r#"+"#,
"under"          => r#"_"#,
"equal"          => r#"="#,
"asciicirc"      => r#"^"#,
"dagger"         => r#"†"#,
"dag"            => r#"†"#,
"Dagger"         => r#"‡"#,
"ddag"           => r#"‡"#,
"ensp"           => r#" "#,
"emsp"           => r#" "#,
"thinsp"         => r#" "#,
"curren"         => r#"¤"#,
"cent"           => r#"¢"#,
"pound"          => r#"£"#,
"yen"            => r#"¥"#,
"euro"           => r#"€"#,
"EUR"            => r#"€"#,
"dollar"         => r#"$"#,
"USD"            => r#"$"#,
"copy"           => r#"©"#,
"reg"            => r#"®"#,
"trade"          => r#"™"#,
"minus"          => r#"−"#,
"pm"             => r#"±"#,
"plusmn"         => r#"±"#,
"times"          => r#"×"#,
"frasl"          => r#"⁄"#,
"colon"          => r#":"#,
"div"            => r#"÷"#,
"frac12"         => r#"½"#,
"frac14"         => r#"¼"#,
"frac34"         => r#"¾"#,
"permil"         => r#"‰"#,
"sup1"           => r#"¹"#,
"sup2"           => r#"²"#,
"sup3"           => r#"³"#,
"radic"          => r#"√"#,
"sum"            => r#"∑"#,
"prod"           => r#"∏"#,
"micro"          => r#"µ"#,
"macr"           => r#"¯"#,
"deg"            => r#"°"#,
"prime"          => r#"′"#,
"Prime"          => r#"″"#,
"infin"          => r#"∞"#,
"infty"          => r#"∞"#,
"prop"           => r#"∝"#,
"propto"         => r#"∝"#,
"not"            => r#"¬"#,
"neg"            => r#"¬"#,
"land"           => r#"∧"#,
"wedge"          => r#"∧"#,
"lor"            => r#"∨"#,
"vee"            => r#"∨"#,
"cap"            => r#"∩"#,
"cup"            => r#"∪"#,
"smile"          => r#"⌣"#,
"frown"          => r#"⌢"#,
"int"            => r#"∫"#,
"therefore"      => r#"∴"#,
"there4"         => r#"∴"#,
"because"        => r#"∵"#,
"sim"            => r#"∼"#,
"cong"           => r#"≅"#,
"simeq"          => r#"≅"#,
"asymp"          => r#"≈"#,
"approx"         => r#"≈"#,
"ne"             => r#"≠"#,
"neq"            => r#"≠"#,
"equiv"          => r#"≡"#,
"triangleq"      => r#"≜"#,
"le"             => r#"≤"#,
"leq"            => r#"≤"#,
"ge"             => r#"≥"#,
"geq"            => r#"≥"#,
"lessgtr"        => r#"≶"#,
"lesseqgtr"      => r#"⋚"#,
"ll"             => r#"≪"#,
"Ll"             => r#"⋘"#,
"lll"            => r#"⋘"#,
"gg"             => r#"≫"#,
"Gg"             => r#"⋙"#,
"ggg"            => r#"⋙"#,
"prec"           => r#"≺"#,
"preceq"         => r#"≼"#,
"preccurlyeq"    => r#"≼"#,
"succ"           => r#"≻"#,
"succeq"         => r#"≽"#,
"succcurlyeq"    => r#"≽"#,
"sub"            => r#"⊂"#,
"subset"         => r#"⊂"#,
"sup"            => r#"⊃"#,
"supset"         => r#"⊃"#,
"nsub"           => r#"⊄"#,
"sube"           => r#"⊆"#,
"nsup"           => r#"⊅"#,
"supe"           => r#"⊇"#,
"setminus"       => r#"∖"#,
"forall"         => r#"∀"#,
"exist"          => r#"∃"#,
"exists"         => r#"∃"#,
"nexist"         => r#"∃"#,
"nexists"        => r#"∃"#,
"empty"          => r#"∅"#,
"emptyset"       => r#"∅"#,
"isin"           => r#"∈"#,
"in"             => r#"∈"#,
"notin"          => r#"∉"#,
"ni"             => r#"∋"#,
"nabla"          => r#"∇"#,
"ang"            => r#"∠"#,
"angle"          => r#"∠"#,
"perp"           => r#"⊥"#,
"parallel"       => r#"∥"#,
"sdot"           => r#"⋅"#,
"cdot"           => r#"⋅"#,
"lceil"          => r#"⌈"#,
"rceil"          => r#"⌉"#,
"lfloor"         => r#"⌊"#,
"rfloor"         => r#"⌋"#,
"lang"           => r#"⟨"#,
"rang"           => r#"⟩"#,
"langle"         => r#"⟨"#,
"rangle"         => r#"⟩"#,
"hbar"           => r#"ℏ"#,
"mho"            => r#"℧"#,
"larr"           => r#"←"#,
"leftarrow"      => r#"←"#,
"gets"           => r#"←"#,
"lArr"           => r#"⇐"#,
"Leftarrow"      => r#"⇐"#,
"uarr"           => r#"↑"#,
"uparrow"        => r#"↑"#,
"uArr"           => r#"⇑"#,
"Uparrow"        => r#"⇑"#,
"rarr"           => r#"→"#,
"to"             => r#"→"#,
"rightarrow"     => r#"→"#,
"rArr"           => r#"⇒"#,
"Rightarrow"     => r#"⇒"#,
"darr"           => r#"↓"#,
"downarrow"      => r#"↓"#,
"dArr"           => r#"⇓"#,
"Downarrow"      => r#"⇓"#,
"harr"           => r#"↔"#,
"leftrightarrow" => r#"↔"#,
"hArr"           => r#"⇔"#,
"Leftrightarrow" => r#"⇔"#,
"crarr"          => r#"↵"#,
"hookleftarrow"  => r#"↵"#,
"arccos"         => r#"arccos"#,
"arcsin"         => r#"arcsin"#,
"arctan"         => r#"arctan"#,
"arg"            => r#"arg"#,
"cos"            => r#"cos"#,
"cosh"           => r#"cosh"#,
"cot"            => r#"cot"#,
"coth"           => r#"coth"#,
"csc"            => r#"csc"#,
"det"            => r#"det"#,
"dim"            => r#"dim"#,
"exp"            => r#"exp"#,
"gcd"            => r#"gcd"#,
"hom"            => r#"hom"#,
"inf"            => r#"inf"#,
"ker"            => r#"ker"#,
"lg"             => r#"lg"#,
"lim"            => r#"lim"#,
"liminf"         => r#"liminf"#,
"limsup"         => r#"limsup"#,
"ln"             => r#"ln"#,
"log"            => r#"log"#,
"max"            => r#"max"#,
"min"            => r#"min"#,
"Pr"             => r#"Pr"#,
"sec"            => r#"sec"#,
"sin"            => r#"sin"#,
"sinh"           => r#"sinh"#,
"tan"            => r#"tan"#,
"tanh"           => r#"tanh"#,
"bull"           => r#"•"#,
"bullet"         => r#"•"#,
"star"           => r#"*"#,
"lowast"         => r#"∗"#,
"ast"            => r#"∗"#,
"odot"           => r#"o"#,
"oplus"          => r#"⊕"#,
"otimes"         => r#"⊗"#,
"check"          => r#"✓"#,
"checkmark"      => r#"✓"#,
"para"           => r#"¶"#,
"ordf"           => r#"ª"#,
"ordm"           => r#"º"#,
"cedil"          => r#"¸"#,
"oline"          => r#"‾"#,
"uml"            => r#"¨"#,
"zwnj"           => r#"‌"#,
"zwj"            => r#"‍"#,
"lrm"            => r#"‎"#,
"rlm"            => r#"‏"#,
"smiley"         => r#"☺"#,
"blacksmile"     => r#"☻"#,
"sad"            => r#"☹"#,
"frowny"         => r#"☹"#,
"clubs"          => r#"♣"#,
"clubsuit"       => r#"♣"#,
"spades"         => r#"♠"#,
"spadesuit"      => r#"♠"#,
"hearts"         => r#"♥"#,
"heartsuit"      => r#"♥"#,
"diams"          => r#"♦"#,
"diamondsuit"    => r#"♦"#,
"diamond"        => r#"⋄"#,
"Diamond"        => r#"⋄"#,
"loz"            => r#"◊"#,
};

#[derive(Debug, Clone, Copy)]
pub struct Entity<'a> {
    pub name: &'a str,
    pub mapped_item: &'a str,
}

pub(crate) fn parse_entity(name: &str) -> Result<Entity> {
    if let Some(mapped_item) = ENTITY_MAP.get(name) {
        Ok(Entity { name, mapped_item })
    } else {
        Err(MatchError::InvalidLogic)
    }
}
