//! Build a tiny static blog from plain-text files.
//!
//! Each post lives in posts/YYYY-MM-DD-slug.txt. The top of a post may have
//! optional header lines, in any order, followed by the text:
//!
//!     tags: anime, games, manga
//!     pics: cover-volume12.jpg
//!     spoiler: yes
//!     Post here
//!
//! Pictures are stored in assets/. With `spoiler: yes`, the picture is hidden
//! behind a "show me" toggle first.
//!
//! Output goes to site/:
//!     index.html            all posts
//!     tags/<tag>.html       one page per tag
//!     assets/               copied from assets/
//!
//! Run from the blog's folder, or pass the folder as the first argument:
//!     cargo run --release -- path/to/blog

use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const TAGS_DIR_NAME: &str = "tags";
const SITE_TITLE: &str = "おっぱい";
const MAX_CHARS: usize = 1000; // only a warning, never enforced

const CSS: &str = r##"
body {
  max-width: 40em;
  margin: 2em auto;
  padding: 0 1em;
  font-family: Georgia, "Times New Roman", serif;
  font-size: 1.125rem;
  line-height: 1.6;
  color: #222;
}
h1 { font-size: 1.6em; margin-bottom: 0.2em; }
h1 a { color: inherit; text-decoration: none; }
h2 { font-size: 1.1em; font-weight: normal; color: #666; }
.date, .tags, .tag-list {
  color: #666;
  font-family: Verdana, Arial, sans-serif;
  font-size: 0.85em;
  margin: 0 0 0.3em 0;
}
.entry p { margin: 0 0 0.6em 0; white-space: pre-wrap; }
.entry p.date, .entry p.tags { white-space: normal; }
.pic {
  max-width: min(100%, 240px);
  height: auto;
  margin-bottom: 0.6em;
}
details { margin-bottom: 0.6em; }
summary {
  cursor: pointer;
  color: #666;
  font-family: Verdana, Arial, sans-serif;
  font-size: 0.85em;
  margin-bottom: 0.4em;
}
hr {
  border: none;
  border-top: 1px solid #ccc;
  margin: 1.5em 0;
}
"##;

struct Config {
    posts_dir: PathBuf,
    assets_dir: PathBuf,
    output_dir: PathBuf,
}

struct Post {
    date: String,
    text: String,
    pics: String,
    tags: Vec<String>,
    spoiler: bool,
}

/// Equivalent of Python's `html.escape(s, quote=True)`.
fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

/// Return the YYYY-MM-DD prefix of a filename stem, or "" if absent.
fn parse_date(stem: &str) -> String {
    let chars: Vec<char> = stem.chars().collect();
    if chars.len() >= 10 && chars[4] == '-' && chars[7] == '-' {
        chars[..10].iter().collect()
    } else {
        String::new()
    }
}

/// Make a tag safe for use in a filename/URL.
fn slugify(tag: &str) -> String {
    let mut out = String::new();
    let mut pending_dash = false;

    for c in tag.to_lowercase().chars() {
        if c.is_alphanumeric() || c == '_' {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else {
            pending_dash = true;
        }
    }

    if out.is_empty() {
        "tag".to_string()
    } else {
        out
    }
}

/// Accept only a simple filename from assets/.
///
/// A picture reference may be `photo.jpg`, but not a path such as
/// `../photo.jpg`, `/photo.jpg`, or `subdir/photo.jpg`.
fn safe_pic_name(value: &str) -> Result<String> {
    let value = value.trim();

    if value.is_empty() {
        return Ok(String::new());
    }

    // Only allow a plain filename, never a directory or path.
    if Path::new(value).file_name() != Some(OsStr::new(value)) {
        return Err(format!(
            "Invalid pics value {value:?}: \
             pictures must be filenames directly inside assets/"
        )
        .into());
    }

    if value == "." || value == ".." || value.contains('\0') || value.contains('\\') {
        return Err(format!("Invalid pics value {value:?}.").into());
    }

    Ok(value.to_string())
}

/// Split leading header lines from the post body.
fn parse_header(raw: &str) -> (HashMap<String, String>, String) {
    let mut header = HashMap::new();
    let lines: Vec<&str> = raw.split('\n').collect();
    let mut i = 0;

    while i < lines.len() {
        if let Some((key, value)) = lines[i].split_once(':') {
            let key = key.trim().to_lowercase();
            if matches!(key.as_str(), "tags" | "pics" | "spoiler") {
                header.insert(key, value.trim().to_string());
                i += 1;
                continue;
            }
        }
        break;
    }

    let body = lines[i..].join("\n").trim().to_string();
    (header, body)
}

fn load_posts(cfg: &Config) -> Result<Vec<Post>> {
    let mut paths: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(&cfg.posts_dir)? {
        let path = entry?.path();
        let hidden = path
            .file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |n| n.starts_with('.'));

        if path.is_file() && path.extension() == Some(OsStr::new("txt")) && !hidden {
            paths.push(path);
        }
    }

    // Newest first (filenames start with the date).
    paths.sort();
    paths.reverse();

    let mut posts = Vec::new();

    for path in paths {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("{name}: {e}"))?
            .replace("\r\n", "\n");
        let raw = raw.trim();

        if raw.is_empty() {
            continue;
        }

        let (header, text) = parse_header(raw);

        let pics = safe_pic_name(header.get("pics").map_or("", String::as_str))
            .map_err(|e| format!("{name}: {e}"))?;

        let tags: Vec<String> = header
            .get("tags")
            .map_or("", String::as_str)
            .split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect();

        let spoiler = matches!(
            header
                .get("spoiler")
                .map_or(String::new(), |s| s.to_lowercase())
                .as_str(),
            "yes" | "true" | "1"
        );

        if !pics.is_empty() && !cfg.assets_dir.join(&pics).is_file() {
            println!("WARNING: {name} uses missing picture 'assets/{pics}'.");
        }

        let len = text.chars().count();
        if len > MAX_CHARS {
            println!("NOTE: {name} is {len} characters (guideline is {MAX_CHARS}).");
        }

        posts.push(Post {
            date: parse_date(&stem),
            text,
            pics,
            tags,
            spoiler,
        });
    }

    Ok(posts)
}

/// Map tag slug -> posts (sorted by slug).
fn group_by_tag(posts: &[Post]) -> BTreeMap<String, Vec<&Post>> {
    let mut groups: BTreeMap<String, Vec<&Post>> = BTreeMap::new();

    for post in posts {
        for tag in &post.tags {
            groups.entry(slugify(tag)).or_default().push(post);
        }
    }

    groups
}

/// Map tag slug -> display name.
///
/// Fails if different tag names would produce the same URL slug.
fn tag_names(posts: &[Post]) -> Result<BTreeMap<String, String>> {
    let mut names: BTreeMap<String, String> = BTreeMap::new();

    for post in posts {
        for tag in &post.tags {
            let slug = slugify(tag);

            match names.get(&slug) {
                Some(existing) if existing != tag => {
                    return Err(format!(
                        "Tag slug collision: {existing:?} and {tag:?} \
                         both produce the URL slug {slug:?}."
                    )
                    .into());
                }
                Some(_) => {}
                None => {
                    names.insert(slug, tag.clone());
                }
            }
        }
    }

    Ok(names)
}

/// Render one post. `root` is the relative path back to the site root
/// ("" or "../").
fn render_post(post: &Post, root: &str) -> String {
    let mut parts: Vec<String> = vec![r#"<div class="entry">"#.to_string()];

    if !post.date.is_empty() {
        parts.push(format!(r#"<p class="date">{}</p>"#, escape(&post.date)));
    }

    if !post.pics.is_empty() {
        let src = escape(&format!("{root}assets/{}", post.pics));

        let mut image = format!(
            r#"<a href="{src}"><img class="pic" src="{src}" alt="Picture" loading="lazy"></a>"#
        );

        if post.spoiler {
            image = format!("<details><summary>show me</summary>{image}</details>");
        }

        parts.push(image);
    }

    parts.push(format!("<p>{}</p>", escape(&post.text)));

    if !post.tags.is_empty() {
        let links = post
            .tags
            .iter()
            .map(|tag| {
                format!(
                    r#"<a href="{root}{TAGS_DIR_NAME}/{}.html">{}</a>"#,
                    slugify(tag),
                    escape(tag)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        parts.push(format!(r#"<p class="tags">Tags: {links}</p>"#));
    }

    parts.push("</div>".to_string());

    parts.join("\n")
}

fn render_page(posts: &[&Post], root: &str, subtitle: &str, tag_bar: &str) -> String {
    let title = escape(SITE_TITLE);

    let sub = if subtitle.is_empty() {
        String::new()
    } else {
        format!("<h2>{}</h2>", escape(subtitle))
    };

    let body = if posts.is_empty() {
        "<p><em>Nothing posted yet.</em></p>".to_string()
    } else {
        let rendered: Vec<String> = posts.iter().map(|p| render_post(p, root)).collect();
        format!("{}\n<hr>", rendered.join("\n<hr>\n"))
    };

    let home = if root.is_empty() { "./" } else { root };
    let css = CSS.trim();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
{css}
</style>
</head>
<body>
<h1><a href="{home}">{title}</a></h1>
{sub}
{tag_bar}
{body}
</body>
</html>
"#
    )
}

fn render_tag_bar(
    groups: &BTreeMap<String, Vec<&Post>>,
    names: &BTreeMap<String, String>,
) -> String {
    if groups.is_empty() {
        return String::new();
    }

    let links = groups
        .iter()
        .map(|(slug, posts)| {
            format!(
                r#"<a href="{TAGS_DIR_NAME}/{slug}.html">{}</a> ({})"#,
                escape(&names[slug]),
                posts.len()
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");

    format!(r#"<p class="tag-list">Tags: {links}</p>"#)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }

    Ok(())
}

/// Build the complete static site.
fn build(cfg: &Config) -> Result<()> {
    // Start from a clean output folder so deleted posts/assets
    // don't linger in the published site.
    if cfg.output_dir.exists() {
        fs::remove_dir_all(&cfg.output_dir)?;
    }
    fs::create_dir_all(&cfg.output_dir)?;

    let posts = load_posts(cfg)?;
    let groups = group_by_tag(&posts);
    let names = tag_names(&posts)?;

    let all: Vec<&Post> = posts.iter().collect();
    let index = render_page(&all, "", "", &render_tag_bar(&groups, &names));
    fs::write(cfg.output_dir.join("index.html"), index)?;

    if !groups.is_empty() {
        let tags_dir = cfg.output_dir.join(TAGS_DIR_NAME);
        fs::create_dir(&tags_dir)?;

        for (slug, tagged) in &groups {
            let page = render_page(tagged, "../", &format!("Tag: {}", names[slug]), "");
            fs::write(tags_dir.join(format!("{slug}.html")), page)?;
        }
    }

    if cfg.assets_dir.is_dir() {
        copy_dir(&cfg.assets_dir, &cfg.output_dir.join("assets"))?;
    }

    println!(
        "Wrote {} post(s) and {} tag page(s) to {}.",
        posts.len(),
        groups.len(),
        cfg.output_dir.display()
    );

    Ok(())
}

fn run() -> Result<()> {
    let root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let cfg = Config {
        posts_dir: root.join("posts"),
        assets_dir: root.join("assets"),
        output_dir: root.join("site"),
    };

    fs::create_dir_all(&cfg.posts_dir)?;
    build(&cfg)
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
