# Others

Since its indexer is essentially just a CLI tool, InfiSearch could be used almost anywhere (e.g. other static site generators) even without a custom wrapper implementation (e.g. the Mdbook plugin).

For example, to deploy another static site generator's output to gh-pages using github actions, simply chain the CLI tool on top of the static site generator output, after you've linked the necessary scripts:

```yml
name: docs
on:
  push:
    branches:
      - docs
jobs:
  build-docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Build documentation
        run: # <insert your favourite ssg build command>
      - name: Install InfiSearch
        run: cargo install infisearch # or, using the binary release
      - name: Run InfiSearch
        run: infisearch <docs_build_folder> <docs_build_folder/output> -c <indexer_config_path>
      - name: Deploy to github pages 🚀
        uses: JamesIves/github-pages-deploy-action@4.1.5
        with:
          branch: gh-pages
          folder: <docs_build_folder>
```
