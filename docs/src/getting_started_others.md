# Other Use Cases

Since its indexer is essentially just a CLI tool, morsels could in-theory be used almost anywhere (e.g. other static site generators) easily without a custom wrapper implementation (such as the Mdbook plugin).

For example, to deploy another static site generator to gh-pages using github actions, simply chain the morsels tool on top of the static site generator output:

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
      - name: Install Morsels
        run: cargo install morsels_indexer # or, using the binary release
      - name: Run Morsels
        run: morsels <docs_build_folder> <docs_build_folder/morsels_output> -c <morsels_config_path>
      - name: Deploy to github pages 🚀
        uses: JamesIves/github-pages-deploy-action@4.1.5
        with:
          branch: gh-pages
          folder: <docs_build_folder>
```

## Custom Data Formats (non `.html`)

Some use cases may not always have `.html` files readily available (e.g. pure client-side rendered ones) or in the right format.

In such cases, morsels also supports `.json` and `.csv` files, which is covered in greater detail later under [indexer configuration](./indexer_configuration.md).

Another simpler (but likely slow) alternative you could consider specifically for client-side rendered projects is to display the page in a headless browser, then index said html file.
