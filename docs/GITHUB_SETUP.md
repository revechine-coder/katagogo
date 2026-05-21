# GitHub And Git LFS Notes

The GitHub repository is:

```text
https://github.com/revechine-coder/katagogo.git
```

## Authentication

This machine uses GitHub CLI authentication for HTTPS Git operations.

Useful commands:

```sh
gh auth status
gh auth login --hostname github.com --git-protocol https --web
gh auth setup-git
```

The browser device-code flow works when Chrome is already logged in to GitHub.

## Git LFS

GitHub rejects normal Git blobs above 100 MB. KataGo model and release artifacts must stay in Git LFS.

Install and enable LFS:

```sh
brew install git-lfs
git lfs install
```

Tracked large files currently include:

```text
kata-engine/kata1-b18c384nbt.bin.gz
kata-engine/kata1-b28c512nbt.bin.gz
releases/KataGoGo 2026-05-20 23-02-54/KataGoGo.zip
releases/KataGoGo 2026-05-20 23-02-54/KataGoGo.app/Contents/Resources/kata-engine/kata1-b18c384nbt.bin.gz
```

Check LFS status:

```sh
git lfs ls-files
git lfs status
```

After cloning:

```sh
git lfs pull
```

## Publishing

Normal publish flow:

```sh
git status --short
git add README.md docs/REPOSITORY.md docs/GITHUB_SETUP.md
git commit -m "Document repository setup"
git push
```

If a future large file is rejected by GitHub, add it to LFS before pushing:

```sh
git lfs track "path/to/large-file"
git add .gitattributes "path/to/large-file"
git commit -m "Track large artifact with Git LFS"
git push
```

If the rejected file already exists in local history and the remote did not accept the push, migrate it:

```sh
git lfs migrate import --include="path/to/large-file"
git push
```

Only rewrite shared history after confirming no one else has based work on the old commits.
