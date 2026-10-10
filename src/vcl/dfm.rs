//! Parser for Delphi's text form files (`.dfm`): the object tree of a form
//! with its published properties, as written by the Delphi IDE.

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    /// identifiers: enum values, `True`/`False`, colours, event handler names
    Ident(String),
    /// `[fsBold, fsItalic]`
    Set(Vec<String>),
    /// `( ... )`: string lists, `DesignSize`, ...
    List(Vec<Value>),
    /// `{ hex }`: binary data (bitmaps)
    Bin(Vec<u8>),
    /// `< item ... end >`: collections
    Items(Vec<Vec<(String, Value)>>),
}

impl Value {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            _ => None,
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) | Value::Ident(s) => Some(s),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Ident(s) if s.eq_ignore_ascii_case("true") => Some(true),
            Value::Ident(s) if s.eq_ignore_ascii_case("false") => Some(false),
            _ => None,
        }
    }
    /// The strings of a `( 'a' 'b' )` list.
    pub fn as_strings(&self) -> Vec<String> {
        match self {
            Value::List(v) => v.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
            Value::Str(s) => vec![s.clone()],
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Obj {
    pub name: String,
    pub class: String,
    pub props: Vec<(String, Value)>,
    pub children: Vec<Obj>,
}

impl Obj {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.props.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v)
    }
    pub fn int(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(Value::as_int)
    }
    pub fn str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(Value::as_str)
    }
    pub fn bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(Value::as_bool)
    }
    pub fn find(&self, name: &str) -> Option<&Obj> {
        if self.name.eq_ignore_ascii_case(name) {
            return Some(self);
        }
        self.children.iter().find_map(|c| c.find(name))
    }
}

struct P<'a> {
    s: &'a [u8],
    i: usize,
    line: usize,
}

impl<'a> P<'a> {
    fn err<T>(&self, m: &str) -> Result<T, String> {
        Err(format!("dfm line {}: {m}", self.line))
    }
    fn ws(&mut self) {
        while self.i < self.s.len() {
            match self.s[self.i] {
                b'\n' => {
                    self.line += 1;
                    self.i += 1
                }
                b' ' | b'\t' | b'\r' => self.i += 1,
                _ => break,
            }
        }
    }
    fn peek(&mut self) -> u8 {
        self.ws();
        self.s.get(self.i).copied().unwrap_or(0)
    }
    fn ident(&mut self) -> String {
        self.ws();
        let st = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphanumeric() || b"_.".contains(&self.s[self.i])) {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[st..self.i]).into_owned()
    }
    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == c {
            self.i += 1;
            Ok(())
        } else {
            self.err(&format!("expected '{}'", c as char))
        }
    }

    fn object(&mut self) -> Result<Obj, String> {
        let kw = self.ident();
        if !["object", "inherited", "inline"].iter().any(|k| kw.eq_ignore_ascii_case(k)) {
            return self.err(&format!("expected object, got '{kw}'"));
        }
        let mut o = Obj::default();
        let first = self.ident();
        if self.peek() == b':' {
            self.i += 1;
            o.name = first;
            o.class = self.ident();
        } else {
            o.class = first;
        }
        if self.peek() == b'[' {
            // inherited index
            while self.i < self.s.len() && self.s[self.i] != b']' {
                self.i += 1;
            }
            self.i += 1;
        }
        loop {
            let save = (self.i, self.line);
            let id = self.ident();
            if id.is_empty() {
                return self.err("unexpected character");
            }
            if id.eq_ignore_ascii_case("end") {
                break;
            }
            if ["object", "inherited", "inline"].iter().any(|k| id.eq_ignore_ascii_case(k)) {
                (self.i, self.line) = save;
                o.children.push(self.object()?);
                continue;
            }
            self.expect(b'=')?;
            let v = self.value()?;
            o.props.push((id, v));
        }
        Ok(o)
    }

    fn string(&mut self) -> Result<String, String> {
        // a sequence of 'quoted' parts and #nnn characters, joined by '+'
        let mut out = String::new();
        loop {
            let c = self.peek();
            if c == b'\'' {
                self.i += 1;
                let mut bytes = Vec::new();
                loop {
                    let Some(&b) = self.s.get(self.i) else { return self.err("unterminated string") };
                    self.i += 1;
                    if b == b'\'' {
                        if self.s.get(self.i) == Some(&b'\'') {
                            bytes.push(b'\'');
                            self.i += 1;
                        } else {
                            break;
                        }
                    } else {
                        bytes.push(b);
                    }
                }
                // DFM files are ANSI (Windows-1252); map bytes >= 0x80 as Latin-1
                match String::from_utf8(bytes.clone()) {
                    Ok(s) => out.push_str(&s),
                    Err(_) => out.extend(bytes.iter().map(|&b| b as char)),
                }
            } else if c == b'#' {
                self.i += 1;
                let st = self.i;
                while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                    self.i += 1;
                }
                let n: u32 = std::str::from_utf8(&self.s[st..self.i]).unwrap_or("0").parse().unwrap_or(32);
                out.push(char::from_u32(n).unwrap_or('?'));
            } else if c == b'+' {
                self.i += 1;
            } else {
                break;
            }
            // parts directly next to each other belong together; across
            // white space only a '+' continues the string
            if matches!(self.s.get(self.i), Some(b'\'') | Some(b'#')) {
                continue;
            }
            let save = (self.i, self.line);
            if self.peek() == b'+' {
                self.i += 1;
                self.peek();
                continue;
            }
            (self.i, self.line) = save;
            break;
        }
        Ok(out)
    }

    fn value(&mut self) -> Result<Value, String> {
        let c = self.peek();
        match c {
            b'\'' | b'#' => Ok(Value::Str(self.string()?)),
            b'[' => {
                self.i += 1;
                let mut v = Vec::new();
                loop {
                    match self.peek() {
                        b']' => {
                            self.i += 1;
                            break;
                        }
                        b',' => self.i += 1,
                        0 => return self.err("unterminated set"),
                        _ => {
                            let id = self.ident();
                            if id.is_empty() {
                                // numeric sets like [0, 1]
                                let st = self.i;
                                while self.i < self.s.len() && !b",]".contains(&self.s[self.i]) {
                                    self.i += 1;
                                }
                                v.push(String::from_utf8_lossy(&self.s[st..self.i]).trim().to_string());
                            } else {
                                v.push(id);
                            }
                        }
                    }
                }
                Ok(Value::Set(v))
            }
            b'(' => {
                self.i += 1;
                let mut v = Vec::new();
                loop {
                    if self.peek() == b')' {
                        self.i += 1;
                        break;
                    }
                    if self.peek() == 0 {
                        return self.err("unterminated list");
                    }
                    v.push(self.value()?);
                }
                Ok(Value::List(v))
            }
            b'{' => {
                self.i += 1;
                let mut out = Vec::new();
                let mut hi: Option<u8> = None;
                loop {
                    let Some(&b) = self.s.get(self.i) else { return self.err("unterminated binary") };
                    self.i += 1;
                    if b == b'}' {
                        break;
                    }
                    if b == b'\n' {
                        self.line += 1;
                    }
                    if let Some(d) = (b as char).to_digit(16) {
                        match hi.take() {
                            None => hi = Some(d as u8),
                            Some(h) => out.push(h << 4 | d as u8),
                        }
                    }
                }
                Ok(Value::Bin(out))
            }
            b'<' => {
                self.i += 1;
                let mut items = Vec::new();
                loop {
                    if self.peek() == b'>' {
                        self.i += 1;
                        break;
                    }
                    let id = self.ident();
                    if !id.eq_ignore_ascii_case("item") {
                        return self.err("expected item");
                    }
                    let mut props = Vec::new();
                    loop {
                        let k = self.ident();
                        if k.is_empty() {
                            return self.err("bad collection item");
                        }
                        if k.eq_ignore_ascii_case("end") {
                            break;
                        }
                        self.expect(b'=')?;
                        props.push((k, self.value()?));
                    }
                    items.push(props);
                }
                Ok(Value::Items(items))
            }
            b'-' | b'0'..=b'9' | b'$' => {
                let st = self.i;
                self.i += 1;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_hexdigit() || b".eE+-".contains(&self.s[self.i])) {
                    // a '-' or '+' only after an exponent
                    if b"+-".contains(&self.s[self.i]) && !b"eE".contains(&self.s[self.i - 1]) {
                        break;
                    }
                    self.i += 1;
                }
                let t = std::str::from_utf8(&self.s[st..self.i]).unwrap_or("0");
                if let Some(h) = t.strip_prefix('$') {
                    return Ok(Value::Int(i64::from_str_radix(h, 16).unwrap_or(0)));
                }
                if let Some(h) = t.strip_prefix("-$") {
                    return Ok(Value::Int(-i64::from_str_radix(h, 16).unwrap_or(0)));
                }
                if let Ok(i) = t.parse::<i64>() {
                    Ok(Value::Int(i))
                } else if let Ok(f) = t.parse::<f64>() {
                    Ok(Value::Float(f))
                } else {
                    self.err(&format!("bad number '{t}'"))
                }
            }
            _ => {
                let id = self.ident();
                if id.is_empty() {
                    return self.err(&format!("unexpected '{}'", c as char));
                }
                Ok(Value::Ident(id))
            }
        }
    }
}

pub fn parse(text: &str) -> Result<Obj, String> {
    let mut p = P { s: text.as_bytes(), i: 0, line: 1 };
    p.object()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_strings_sets_lists() {
        let t = "object F: TForm\n  Caption = 'a'#13#10'b''c' +\n    'd'\n  Font.Style = [fsBold]\n  Items.Strings = (\n    'x'\n    'y')\n  Glyph.Data = {\n    0A0B}\n  Tag = -2\n  W = 1.5\n  object B: TButton\n    OnClick = BClick\n  end\nend\n";
        let o = parse(t).unwrap();
        assert_eq!(o.str("Caption"), Some("a\r\nb'cd"));
        assert_eq!(o.get("Font.Style"), Some(&Value::Set(vec!["fsBold".into()])));
        assert_eq!(o.get("Items.Strings").unwrap().as_strings(), vec!["x", "y"]);
        assert_eq!(o.get("Glyph.Data"), Some(&Value::Bin(vec![10, 11])));
        assert_eq!(o.int("Tag"), Some(-2));
        assert_eq!(o.children[0].str("OnClick"), Some("BClick"));
    }

    #[test]
    fn parses_all_mb3d_forms() {
        for (name, text) in crate::app::forms::ALL {
            parse(text).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }
}
