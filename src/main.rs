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
//! sidebar.txt (optional, next to Cargo.toml) lists what you are currently
//! reading/watching/etc. One item per line:
//!
//!     reading: cover-volume12.jpg | Harem in the Labyrinth of Another World
//!     watching:
//!
//! collection.txt (optional, next to Cargo.toml) becomes collection.html.
//! Lines starting with == start a section; every other line is one item:
//!
//!     == MANGA
//!     DANDADAN | 22 di ?? | J-POP | cover: dandadan.jpg | review: 2026-10-20-dandadan
//!
//! After the title, fields are separated by |. A field starting with
//! `cover:` is the picture (in assets/), `review:` is the filename (without
//! .txt) of the post that reviews it, or a full http(s) link. Any other
//! field is shown as plain details. A cover is only clickable when there is
//! a review.
//!
//! Lines starting with # and blank lines are ignored in both files.
//!
//! Output goes to site/:
//!     index.html            all posts
//!     tags/<tag>.html       one page per tag (grouped by year)
//!     collection.html       your collection (only if collection.txt exists)
//!     assets/               copied from assets/
//!
//! Run from the blog's folder, or pass the folder as the first argument:
//!     cargo run --release -- path/to/blog

use std::collections::{BTreeMap, HashMap, HashSet};
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
  max-width: 58em;
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
h3.year, h3.section {
  font-size: 0.95em;
  font-weight: normal;
  color: #666;
  font-family: Verdana, Arial, sans-serif;
  margin: 1.5em 0 0.5em 0;
}
.date, .tags, .tag-list, .nav {
  color: #666;
  font-family: Verdana, Arial, sans-serif;
  font-size: 0.85em;
  margin: 0 0 0.3em 0;
}
.entry p {
  margin: 0 0 0.6em 0;
  white-space: pre-wrap;
  text-align: justify;
  hyphens: auto;
}
.entry p.date, .entry p.tags {
  white-space: normal;
  text-align: left;
  hyphens: manual;
}
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
.layout {
  display: grid;
  grid-template-columns: minmax(0, 40em) 15em;
  gap: 3em;
  align-items: start;
}
.sidebar {
  position: sticky;
  top: 2em;
  border: 1px solid #ccc;
  padding: 0.8em 1em;
}
.sidebar h3 {
  font-family: Verdana, Arial, sans-serif;
  font-size: 0.85em;
  font-weight: normal;
  color: #666;
  margin: 0 0 0.5em 0;
}
.sidebar h3:not(:first-child) { margin-top: 1.2em; }
.shelf { display: flex; flex-wrap: wrap; gap: 0.5em; }
.shelf figure { margin: 0; width: calc(50% - 0.25em); }
.shelf img { display: block; width: 100%; height: auto; }
.shelf figcaption {
  font-family: Verdana, Arial, sans-serif;
  font-size: 0.7em;
  color: #666;
  line-height: 1.3;
  margin-top: 0.2em;
}
.collection { list-style: none; margin: 0; padding: 0; }
.collection li {
  display: flex;
  align-items: center;
  gap: 0.8em;
  margin-bottom: 0.6em;
}
.collection .thumb { display: block; width: 56px; height: auto; }
.collection .thumb-gap { display: block; width: 56px; flex: none; }
.collection .info { display: flex; flex-direction: column; line-height: 1.35; }
.collection .details {
  font-family: Verdana, Arial, sans-serif;
  font-size: 0.8em;
  color: #666;
}
@media (max-width: 50em) {
  .layout { grid-template-columns: minmax(0, 1fr); }
  .sidebar { position: static; }
}
"##;

struct Config {
    posts_dir: PathBuf,
    assets_dir: PathBuf,
    output_dir: PathBuf,
    sidebar_file: PathBuf,
    collection_file: PathBuf,
}

struct Post {
    id: String,
    date: String,
    text: String,
    pics: String,
    tags: Vec<String>,
    spoiler: bool,
}

struct ShelfItem {
    pic: String,
    title: String,
}

/// (section name, items), in the order they appear in sidebar.txt.
type Shelf = Vec<(String, Vec<ShelfItem>)>;

struct CollectionItem {
    title: String,
    details: String,
    cover: String,
    review: String,
}

/// (section name, items), in the order they appear in collection.txt.
type Collection = Vec<(String, Vec<CollectionItem>)>;

/// Everything needed to draw one HTML page.
struct Page<'a> {
    root: &'a str,
    subtitle: &'a str,
    nav: &'a str,
    tag_bar: &'a str,
    sidebar: &'a str,
    body: &'a str,
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

/// If `field` starts with `key` (case-insensitive, e.g. "cover:"),
/// return the rest of it, trimmed.
fn strip_key<'a>(field: &'a str, key: &str) -> Option<&'a str> {
    let n = key.len();
    if field.len() >= n && field.is_char_boundary(n) && field[..n].eq_ignore_ascii_case(key) {
        Some(field[n..].trim())
    } else {
        None
    }
}

fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
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
            id: stem,
            text,
            pics,
            tags,
            spoiler,
        });
    }

    Ok(posts)
}

/// Read sidebar.txt. Each line: `section: picture.jpg | Title`.
/// A missing file simply means no sidebar.
fn load_shelf(cfg: &Config) -> Result<Shelf> {
    if !cfg.sidebar_file.is_file() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&cfg.sidebar_file)?.replace("\r\n", "\n");
    let mut shelf: Shelf = Vec::new();

    for (n, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (section, rest) = line.split_once(':').ok_or_else(|| {
            format!(
                "sidebar.txt line {}: expected 'section: picture | title'",
                n + 1
            )
        })?;
        let (pic, title) = rest.split_once('|').unwrap_or((rest, ""));

        let pic =
            safe_pic_name(pic).map_err(|e| format!("sidebar.txt line {}: {e}", n + 1))?;

        // Make sure the section exists, even if it has no items yet.
        let section = section.trim().to_string();
        let idx = match shelf.iter().position(|(name, _)| *name == section) {
            Some(i) => i,
            None => {
                shelf.push((section, Vec::new()));
                shelf.len() - 1
            }
        };

        // No picture: just keep the (possibly empty) section, skip the item.
        if pic.is_empty() {
            if !title.trim().is_empty() {
                println!(
                    "WARNING: sidebar.txt line {} has a title but no picture; skipped.",
                    n + 1
                );
            }
            continue;
        }

        if !cfg.assets_dir.join(&pic).is_file() {
            println!("WARNING: sidebar.txt uses missing picture 'assets/{pic}'.");
        }

        shelf[idx].1.push(ShelfItem {
            pic,
            title: title.trim().to_string(),
        });
    }

    Ok(shelf)
}

/// Read collection.txt. A missing file simply means no collection page.
fn load_collection(cfg: &Config) -> Result<Collection> {
    if !cfg.collection_file.is_file() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&cfg.collection_file)?.replace("\r\n", "\n");
    let mut sections: Collection = Vec::new();

    for (n, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // "== MANGA" starts a new section.
        if let Some(name) = line.strip_prefix("==") {
            sections.push((name.trim_start_matches('=').trim().to_string(), Vec::new()));
            continue;
        }

        let mut fields = line.split('|').map(str::trim);
        let title = fields.next().unwrap_or("").to_string();
        let mut details: Vec<String> = Vec::new();
        let mut cover = String::new();
        let mut review = String::new();

        for field in fields {
            if let Some(v) = strip_key(field, "cover:") {
                cover = safe_pic_name(v)
                    .map_err(|e| format!("collection.txt line {}: {e}", n + 1))?;
            } else if let Some(v) = strip_key(field, "review:") {
                review = v.trim_end_matches(".txt").to_string();
            } else if !field.is_empty() {
                details.push(field.to_string());
            }
        }

        if title.is_empty() {
            return Err(format!("collection.txt line {}: missing title", n + 1).into());
        }

        if !cover.is_empty() && !cfg.assets_dir.join(&cover).is_file() {
            println!("WARNING: collection.txt uses missing picture 'assets/{cover}'.");
        }

        // Items before any "==" line go in an unnamed section.
        if sections.is_empty() {
            sections.push((String::new(), Vec::new()));
        }

        if let Some(last) = sections.last_mut() {
            last.1.push(CollectionItem {
                title,
                details: details.join(" · "),
                cover,
                review,
            });
        }
    }

    Ok(sections)
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
/// ("" or "../"). Each post gets an id so other pages can link to it.
fn render_post(post: &Post, root: &str) -> String {
    let mut parts: Vec<String> = vec![format!(
        r#"<div class="entry" id="{}">"#,
        escape(&post.id)
    )];

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

/// Render posts separated by <hr>. With `by_year`, a year heading is
/// inserted whenever the year changes (posts must be newest first).
fn render_posts(posts: &[&Post], root: &str, by_year: bool) -> String {
    let mut out = String::new();
    let mut current_year = "";

    for post in posts {
        if by_year {
            let year = post.date.get(..4).unwrap_or("");
            if year != current_year {
                current_year = year;
                if !year.is_empty() {
                    out.push_str(&format!(r#"<h3 class="year">{}</h3>"#, escape(year)));
                    out.push('\n');
                }
            }
        }
        out.push_str(&render_post(post, root));
        out.push_str("\n<hr>\n");
    }

    out
}

fn render_posts_body(posts: &[&Post], root: &str, by_year: bool) -> String {
    if posts.is_empty() {
        "<p><em>Nothing posted yet.</em></p>".to_string()
    } else {
        render_posts(posts, root, by_year)
    }
}

/// Render the sidebar. `root` is "" or "../" like in `render_post`.
fn render_shelf(shelf: &Shelf, root: &str) -> String {
    if shelf.is_empty() {
        return String::new();
    }

    let mut out = String::from(r#"<aside class="sidebar">"#);

    for (name, items) in shelf {
        out.push_str(&format!(
            "\n<h3>{}</h3>\n<div class=\"shelf\">",
            escape(name)
        ));

        for item in items {
            let src = escape(&format!("{root}assets/{}", item.pic));
            let title = escape(&item.title);

            out.push_str(&format!(
                r#"<figure><img src="{src}" alt="{title}" title="{title}" loading="lazy">"#
            ));
            if !item.title.is_empty() {
                out.push_str(&format!("<figcaption>{title}</figcaption>"));
            }
            out.push_str("</figure>");
        }

        out.push_str("</div>");
    }

    out.push_str("\n</aside>");
    out
}

/// Render the collection page body. A cover (and the title) becomes a link
/// only when the item has a review.
fn render_collection(collection: &Collection, root: &str) -> String {
    let mut out = String::new();

    for (name, items) in collection {
        if !name.is_empty() {
            out.push_str(&format!(r#"<h3 class="section">{}</h3>"#, escape(name)));
            out.push('\n');
        }

        // If some items in this section have covers, give the others a
        // blank space so the titles line up.
        let any_cover = items.iter().any(|i| !i.cover.is_empty());

        out.push_str("<ul class=\"collection\">\n");

        for item in items {
            let title = escape(&item.title);

            let href: Option<String> = if item.review.is_empty() {
                None
            } else if is_url(&item.review) {
                Some(escape(&item.review))
            } else {
                Some(escape(&format!("{root}index.html#{}", item.review)))
            };

            let thumb = if item.cover.is_empty() {
                if any_cover {
                    r#"<span class="thumb-gap"></span>"#.to_string()
                } else {
                    String::new()
                }
            } else {
                let src = escape(&format!("{root}assets/{}", item.cover));
                let img = format!(
                    r#"<img class="thumb" src="{src}" alt="{title}" loading="lazy">"#
                );
                match &href {
                    Some(h) => format!(r#"<a href="{h}">{img}</a>"#),
                    None => img,
                }
            };

            let name_html = match &href {
                Some(h) => format!(r#"<a href="{h}">{title}</a>"#),
                None => title.clone(),
            };

            let details = if item.details.is_empty() {
                String::new()
            } else {
                format!(r#"<span class="details">{}</span>"#, escape(&item.details))
            };

            out.push_str(&format!(
                "<li>{thumb}<div class=\"info\"><span class=\"name\">{name_html}</span>{details}</div></li>\n"
            ));
        }

        out.push_str("</ul>\n");
    }

    out
}

fn page_shell(p: &Page) -> String {
    let title = escape(SITE_TITLE);

    let sub = if p.subtitle.is_empty() {
        String::new()
    } else {
        format!("<h2>{}</h2>", escape(p.subtitle))
    };

    let home = if p.root.is_empty() { "./" } else { p.root };
    let css = CSS.trim();
    let nav = p.nav;
    let tag_bar = p.tag_bar;
    let body = p.body;
    let sidebar = p.sidebar;

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
<div class="layout">
<main>
<h1><a href="{home}">{title}</a></h1>
{sub}
{nav}
{tag_bar}
{body}
</main>
{sidebar}
</div>
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

    // Tag pages live one folder deeper, so their image paths need "../".
    let shelf = load_shelf(cfg)?;
    let side_index = render_shelf(&shelf, "");
    let side_tags = render_shelf(&shelf, "../");

    let collection = load_collection(cfg)?;
    let has_collection = !collection.is_empty();

    // Warn about reviews that point at a post that doesn't exist.
    let post_ids: HashSet<&str> = posts.iter().map(|p| p.id.as_str()).collect();
    for (_, items) in &collection {
        for item in items {
            if !item.review.is_empty()
                && !is_url(&item.review)
                && !post_ids.contains(item.review.as_str())
            {
                println!(
                    "WARNING: collection.txt: review '{}' (for '{}') matches no post.",
                    item.review, item.title
                );
            }
        }
    }

    let nav_index = if has_collection {
        r#"<p class="nav"><a href="collection.html">collection</a></p>"#.to_string()
    } else {
        String::new()
    };
    let nav_tags = if has_collection {
        r#"<p class="nav"><a href="../collection.html">collection</a></p>"#.to_string()
    } else {
        String::new()
    };

    // Home page.
    let all: Vec<&Post> = posts.iter().collect();
    let tag_bar = render_tag_bar(&groups, &names);
    let index_body = render_posts_body(&all, "", false);
    let index = page_shell(&Page {
        root: "",
        subtitle: "",
        nav: &nav_index,
        tag_bar: &tag_bar,
        sidebar: &side_index,
        body: &index_body,
    });
    fs::write(cfg.output_dir.join("index.html"), index)?;

    // Tag pages.
    if !groups.is_empty() {
        let tags_dir = cfg.output_dir.join(TAGS_DIR_NAME);
        fs::create_dir(&tags_dir)?;

        for (slug, tagged) in &groups {
            let subtitle = format!("Tag: {}", names[slug]);
            let body = render_posts_body(tagged, "../", true);
            let page = page_shell(&Page {
                root: "../",
                subtitle: &subtitle,
                nav: &nav_tags,
                tag_bar: "",
                sidebar: &side_tags,
                body: &body,
            });
            fs::write(tags_dir.join(format!("{slug}.html")), page)?;
        }
    }

    // Collection page.
    if has_collection {
        let body = render_collection(&collection, "");
        let page = page_shell(&Page {
            root: "",
            subtitle: "Collection",
            nav: "",
            tag_bar: "",
            sidebar: "",
            body: &body,
        });
        fs::write(cfg.output_dir.join("collection.html"), page)?;
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
        sidebar_file: root.join("sidebar.txt"),
        collection_file: root.join("collection.txt"),
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
