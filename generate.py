#!/usr/bin/env python3
"""Build a tiny static blog from plain-text files.

Each post lives in posts/YYYY-MM-DD-slug.txt. The top of a post may have
optional header lines, in any order, followed by the text:

    tags: games, opinion
    cover: cover-volume12.jpg
    Why I think TLOU2 is bad game: ...

Output goes to site/:
    index.html            all posts
    tags/<tag>.html       one page per tag
    assets/               copied from assets/
"""

import html
import re
import shutil
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).parent
POSTS_DIR = ROOT / "posts"
ASSETS_DIR = ROOT / "assets"
OUTPUT_DIR = ROOT / "site"
TAGS_DIR_NAME = "tags"

SITE_TITLE = "( .  人  . )"
MAX_CHARS = 1000  # only a warning, never enforced

CSS = """
body {
  max-width: 40em;
  margin: 2em auto;
  padding: 0 1em;
  font-family: Georgia, "Times New Roman", serif;
  line-height: 1.5;
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
.cover { max-width: 100%; height: auto; margin-bottom: 0.6em; }
hr { border: none; border-top: 1px solid #ccc; margin: 1.5em 0; }
""".strip()


@dataclass
class Post:
    date: str
    text: str
    cover: str = ""
    tags: list[str] = field(default_factory=list)


def parse_date(stem: str) -> str:
    """Return the YYYY-MM-DD prefix of a filename stem, or '' if absent."""
    if len(stem) >= 10 and stem[4] == "-" and stem[7] == "-":
        return stem[:10]
    return ""


def slugify(tag: str) -> str:
    """Make a tag safe for use in a filename/URL."""
    return re.sub(r"[^\w]+", "-", tag.lower()).strip("-") or "tag"


def parse_header(raw: str) -> tuple[dict[str, str], str]:
    """Split leading 'key: value' lines (tags, cover) from the post body."""
    header: dict[str, str] = {}
    lines = raw.split("\n")
    while lines:
        key, sep, value = lines[0].partition(":")
        if sep and key.strip().lower() in ("tags", "cover"):
            header[key.strip().lower()] = value.strip()
            lines.pop(0)
        else:
            break
    return header, "\n".join(lines).strip()


def load_posts() -> list[Post]:
    posts = []
    for path in sorted(POSTS_DIR.glob("*.txt"), reverse=True):
        raw = path.read_text(encoding="utf-8").strip()
        if not raw:
            continue

        header, text = parse_header(raw)
        cover = header.get("cover", "")
        tags = [t.strip() for t in header.get("tags", "").split(",") if t.strip()]

        if cover and not (ASSETS_DIR / cover).is_file():
            print(f"WARNING: {path.name} uses missing cover 'assets/{cover}'.")
        if len(text) > MAX_CHARS:
            print(f"NOTE: {path.name} is {len(text)} characters "
                  f"(guideline is {MAX_CHARS}).")

        posts.append(Post(parse_date(path.stem), text, cover, tags))
    return posts


def group_by_tag(posts: list[Post]) -> dict[str, list[Post]]:
    """Map tag slug -> posts (tag display name is kept on the first match)."""
    groups: dict[str, list[Post]] = defaultdict(list)
    for post in posts:
        for tag in post.tags:
            groups[slugify(tag)].append(post)
    return dict(groups)


def tag_names(posts: list[Post]) -> dict[str, str]:
    """Map tag slug -> display name."""
    names: dict[str, str] = {}
    for post in posts:
        for tag in post.tags:
            names.setdefault(slugify(tag), tag)
    return names


def render_post(post: Post, root: str) -> str:
    """`root` is the relative path back to the site root ('' or '../')."""
    parts = ['<div class="entry">']
    if post.date:
        parts.append(f'<p class="date">{html.escape(post.date)}</p>')
    if post.cover:
        src = html.escape(f"{root}assets/{post.cover}", quote=True)
        parts.append(f'<img class="cover" src="{src}" alt="Cover image">')
    parts.append(f"<p>{html.escape(post.text)}</p>")
    if post.tags:
        links = ", ".join(
            f'<a href="{root}{TAGS_DIR_NAME}/{slugify(t)}.html">{html.escape(t)}</a>'
            for t in post.tags
        )
        parts.append(f'<p class="tags">Tags: {links}</p>')
    parts.append("</div>")
    return "\n".join(parts)


def render_page(posts: list[Post], root: str = "", subtitle: str = "",
                tag_bar: str = "") -> str:
    title = html.escape(SITE_TITLE)
    sub = f"<h2>{html.escape(subtitle)}</h2>" if subtitle else ""
    if posts:
        body = "\n<hr>\n".join(render_post(p, root) for p in posts) + "\n<hr>"
    else:
        body = "<p><em>Nothing posted yet.</em></p>"

    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title}</title>
<style>
{CSS}
</style>
</head>
<body>
<h1><a href="{root or './'}">{title}</a></h1>
{sub}
{tag_bar}
{body}
</body>
</html>
"""


def render_tag_bar(groups: dict[str, list[Post]], names: dict[str, str]) -> str:
    if not groups:
        return ""
    links = " · ".join(
        f'<a href="{TAGS_DIR_NAME}/{slug}.html">{html.escape(names[slug])}</a> '
        f"({len(groups[slug])})"
        for slug in sorted(groups)
    )
    return f'<p class="tag-list">Tags: {links}</p>'


def build() -> None:
    # Start from a clean output folder so deleted posts/assets don't linger.
    if OUTPUT_DIR.exists():
        shutil.rmtree(OUTPUT_DIR)
    OUTPUT_DIR.mkdir(parents=True)

    posts = load_posts()
    groups = group_by_tag(posts)
    names = tag_names(posts)

    index = render_page(posts, tag_bar=render_tag_bar(groups, names))
    (OUTPUT_DIR / "index.html").write_text(index, encoding="utf-8")

    if groups:
        tags_dir = OUTPUT_DIR / TAGS_DIR_NAME
        tags_dir.mkdir()
        for slug, tagged in groups.items():
            page = render_page(tagged, root="../", subtitle=f"Tag: {names[slug]}")
            (tags_dir / f"{slug}.html").write_text(page, encoding="utf-8")

    if ASSETS_DIR.is_dir():
        shutil.copytree(ASSETS_DIR, OUTPUT_DIR / "assets")

    print(f"Wrote {len(posts)} post(s) and {len(groups)} tag page(s) "
          f"to {OUTPUT_DIR}.")


if __name__ == "__main__":
    POSTS_DIR.mkdir(exist_ok=True)
    build()
