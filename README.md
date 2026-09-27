# 0881

A plain-text blog. Posting means adding a text file through GitHub's
own website - no terminal, no installs, works from a phone.

## One-time setup

1. Add these files to this repo (drag-and-drop them in on github.com,
   or `git add` + `git push` if you're comfortable with git):
   - `generate.py`
   - `.github/workflows/deploy.yml`
   - `posts/2026-09-27-hello.txt` (a starter post, delete or edit later)
   - `.gitignore`
2. On GitHub, go to this repo's **Settings → Pages**, and under
   "Build and deployment", set **Source** to **GitHub Actions**.
3. Push/commit those files to the `main` branch. Check the
   **Actions** tab - you'll see a workflow run start automatically.
   Once it finishes (a green checkmark, usually under a minute), your
   site is live at:

   **https://lucomorya.github.io/0881/**

## How to post, from now on

1. On github.com, open the `posts/` folder in this repo.
2. Click **Add file → Create new file**.
3. Name it `2026-09-28-anything.txt` (the date up front controls the
   order; the rest of the name is just for you).
4. Type your post in the text box - keep it to roughly 1000
   characters.
5. Scroll down, click **Commit changes** (or "Commit directly to the
   main branch").

That's it. GitHub automatically rebuilds and republishes the site
within about a minute - check the **Actions** tab if you want to
watch it happen. No local setup required, and this all works from
GitHub's mobile site or app too.

To edit or delete a post later, open its file in `posts/`, click the
pencil (edit) or trash icon, and commit - same auto-rebuild happens.

## If you'd rather post from a computer instead

You can still do it the original way: clone the repo, add/edit a
`.txt` file in `posts/` locally, then `git add`, `git commit`,
`git push`. The same GitHub Action picks it up and rebuilds the site
either way - the browser method above and this one both end at the
same place.

## Changing the title or look

- Site title: edit `SITE_TITLE` near the top of `generate.py`.
- Appearance: edit the `<style>` block inside `generate.py`.

Either change triggers a rebuild the next time you commit anything.
