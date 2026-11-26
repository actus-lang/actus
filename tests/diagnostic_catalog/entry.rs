//! Parses one catalog entry (ADR-0123, "One entry per code") and checks its shape.

/// The file a block without `file=` is written to.
pub const DEFAULT_SOURCE: &str = "src/main.act";

const MAXIMUM_SUMMARY_CHARACTERS: usize = 60;

/// One fenced `actus` or `toml` block of an entry.
#[derive(Debug, Default)]
pub struct Block {
    pub section: String,
    pub fails: Option<String>,
    pub build: bool,
    pub compiles: bool,
    pub file: Option<String>,
    pub body: String,
    pub line: usize,
}

impl Block {
    /// The project-relative file this block is written to.
    pub fn target(&self) -> &str {
        self.file.as_deref().unwrap_or(DEFAULT_SOURCE)
    }
}

#[derive(Debug)]
pub struct Entry {
    pub code: String,
    pub blocks: Vec<Block>,
}

pub fn is_code(text: &str) -> bool {
    text.strip_prefix('E')
        .is_some_and(|digits| digits.len() == 4 && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

/// Parses `text`, the content of `<code>.md`, and checks the shape.
pub fn parse_entry(code: &str, text: &str) -> Result<Entry, String> {
    let mut lines = text.lines();
    let title = lines.next().unwrap_or_default();
    let summary = title
        .strip_prefix("# ")
        .and_then(|title| title.strip_prefix(code))
        .and_then(|rest| rest.strip_prefix(": "))
        .ok_or_else(|| format!("title must be `# {code}: <summary>`, found `{title}`"))?;
    if summary.is_empty() || summary.chars().count() > MAXIMUM_SUMMARY_CHARACTERS {
        return Err(format!("summary must be 1 to {MAXIMUM_SUMMARY_CHARACTERS} characters"));
    }
    if summary.chars().next().is_some_and(char::is_uppercase) {
        return Err(format!("summary `{summary}` must start in lower case"));
    }
    let (headings, blocks, has_introduction) = scan_body(text)?;
    if !has_introduction {
        return Err("the title must be followed by an explanation".to_owned());
    }
    check_sections(code, &headings, &blocks)?;
    Ok(Entry { code: code.to_owned(), blocks })
}

/// Reads the headings and fenced blocks below the title. The flag says whether
/// prose stands between the title and the first heading.
fn scan_body(text: &str) -> Result<(Vec<String>, Vec<Block>, bool), String> {
    let mut headings: Vec<String> = Vec::new();
    let mut blocks = Vec::new();
    // `Some(None)` is a fence of another language, `Some(Some(_))` a checked block.
    let mut open: Option<Option<Block>> = None;
    let mut has_introduction = false;
    for (index, line) in text.lines().enumerate().skip(1) {
        match open.as_mut() {
            Some(fenced) if line.trim_end() == "```" => {
                blocks.extend(fenced.take());
                open = None;
            }
            Some(Some(block)) => {
                block.body.push_str(line);
                block.body.push('\n');
            }
            Some(None) => {}
            None => {
                if let Some(info) = line.strip_prefix("```") {
                    let section = headings.last().cloned().unwrap_or_default();
                    open = Some(parse_info(info, &section, index + 1)?);
                } else if let Some(heading) = line.strip_prefix("## ") {
                    headings.push(heading.trim().to_owned());
                } else if headings.is_empty() && !line.trim().is_empty() {
                    has_introduction = true;
                }
            }
        }
    }
    if open.is_some() {
        return Err("a code fence is never closed".to_owned());
    }
    Ok((headings, blocks, has_introduction))
}

/// Reads the info string of an opening fence; `None` for other languages.
fn parse_info(info: &str, section: &str, line: usize) -> Result<Option<Block>, String> {
    let mut words = info.split_whitespace();
    let language = words.next().unwrap_or_default();
    if language != "actus" && language != "toml" {
        return Ok(None);
    }
    let mut block = Block { section: section.to_owned(), line, ..Block::default() };
    for word in words {
        if let Some(value) = word.strip_prefix("fails=") {
            let (code, flag) = value.split_once(',').map_or((value, ""), |pair| pair);
            if !is_code(code) || !(flag.is_empty() || flag == "build") {
                return Err(format!("line {line}: bad marker `{word}`"));
            }
            block.fails = Some(code.to_owned());
            block.build = flag == "build";
        } else if word == "compiles" {
            block.compiles = true;
        } else if let Some(path) = word.strip_prefix("file=") {
            if path.is_empty()
                || path.starts_with('/')
                || path.contains('\\')
                || path.contains("..")
            {
                return Err(format!("line {line}: `{word}` must be a relative path with `/`"));
            }
            block.file = Some(path.to_owned());
        } else {
            return Err(format!("line {line}: unknown marker `{word}`"));
        }
    }
    if language == "toml" && block.file.is_none() {
        return Err(format!("line {line}: a toml block needs `file=`"));
    }
    if language == "actus" && block.fails.is_none() && !block.compiles && block.file.is_none() {
        return Err(format!("line {line}: an actus block needs `fails=` or `compiles`"));
    }
    Ok(Some(block))
}

fn check_sections(code: &str, headings: &[String], blocks: &[Block]) -> Result<(), String> {
    let names: Vec<&str> = headings.iter().map(String::as_str).collect();
    let example_form = match names.as_slice() {
        ["Example", "Fix"] => true,
        ["Cause", "Fix"] => false,
        _ => return Err(format!("sections must be Example and Fix, or Cause and Fix: {names:?}")),
    };
    for block in blocks {
        if let Some(named) = &block.fails
            && (named != code || block.section != "Example")
        {
            return Err(format!(
                "line {}: `fails={named}` must be in Example and name `{code}`",
                block.line
            ));
        }
        if !example_form {
            return Err(format!("line {}: a Cause entry has no checked blocks", block.line));
        }
    }
    if example_form {
        let in_section = |section: &str, wanted: fn(&Block) -> bool| {
            blocks.iter().any(|block| block.section == section && wanted(block))
        };
        if !in_section("Example", |block| block.fails.is_some()) {
            return Err("Example needs a block marked `fails=`".to_owned());
        }
        if !in_section("Fix", |block| block.compiles) {
            return Err("Fix needs a block marked `compiles`".to_owned());
        }
    }
    Ok(())
}
