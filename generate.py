#!/usr/bin/env python3
"""
generate.py - turns a folder of plain text files into one plain HTML page.

No installs, no server, no database. Uses only Python's standard library.

HOW TO POST:
  1. Create a new file in posts/, named like 2026-09-27-whatever.txt
     (the date at the front controls sort order; the rest is just for you).
  2. Write up to about 1000 characters of plain text in it.
  3. Run:  python3 generate.py
  4. Open site/index.html in a browser, or upload it anywhere.

That's the whole workflow.
"""

import html
from pathlib import Path

POSTS_DIR = Path(__file__).parent / "posts"
OUTPUT_DIR = Path(__file__).parent / "site"
SITE_TITLE = "0081"
MAX_CHARS = 1000  # just a warning, not enforced - these are your own files


def load_posts():
    posts = []
    for path in sorted(POSTS_DIR.glob("*.txt"), reverse=True):
        text = path.read_text(encoding="utf-8").strip()
        if not text:
            continue
        if len(text) > MAX_CHARS:
            print(f"NOTE: {path.name} is {len(text)} characters (over the "
                  f"{MAX_CHARS} guideline) - consider trimming it.")
        # Filename convention: YYYY-MM-DD-slug.txt -> use the date part as the label
        stem = path.stem
        date_label = stem[:10] if len(stem) >= 10 and stem[4] == "-" and stem[7] == "-" else ""
        posts.append((date_label, text))
    return posts


def render(posts):
    entries = []
    for date_label, text in posts:
        safe_text = html.escape(text)
        date_html = f'<p class="date">{html.escape(date_label)}</p>' if date_label else ""
        entries.append(
            f'<div class="entry">\n{date_html}\n<p>{safe_text}</p>\n</div>\n<hr>'
        )

    body = "\n".join(entries) if entries else "<p><em>Nothing posted yet.</em></p>"

    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(SITE_TITLE)}</title>
<style>
  body {{
    max-width: 40em;
    margin: 2em auto;
    padding: 0 1em;
    font-family: Georgia, "Times New Roman", serif;
    line-height: 1.5;
    color: #222;
  }}
  h1 {{
    font-size: 1.6em;
    margin-bottom: 0.2em;
  }}
  .date {{
    color: #666;
    font-family: Verdana, Arial, sans-serif;
    font-size: 0.85em;
    margin: 0 0 0.3em 0;
  }}
  .entry p {{
    margin: 0 0 0.6em 0;
    white-space: pre-wrap;
  }}
  hr {{
    border: none;
    border-top: 1px solid #ccc;
    margin: 1.5em 0;
  }}
</style>
</head>
<body>
<h1>{html.escape(SITE_TITLE)}</h1>
{body}
</body>
</html>
"""


def main():
    POSTS_DIR.mkdir(exist_ok=True)
    OUTPUT_DIR.mkdir(exist_ok=True)
    posts = load_posts()
    html_out = render(posts)
    out_path = OUTPUT_DIR / "index.html"
    out_path.write_text(html_out, encoding="utf-8")
    print(f"Wrote {out_path} with {len(posts)} post(s).")


if __name__ == "__main__":
    main()
