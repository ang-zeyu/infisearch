# Primarily for publishing
# Use npm scripts for development

# Update the cargo.toml version numbers before running anything!

# And this
VERSION=v0.5.0

# Run in order
# Check preReleaseXX outputs manually before running release

preReleaseCommon:
	git stash
	cargo clean
	cd packages/morsels_common &&\
	cargo package &&\
	cargo package --list

releaseCommon:
	cd packages/morsels_common &&\
	cargo publish

preReleaseAsciiLanguage:
	cd packages/morsels_languages/morsels_lang_ascii &&\
	cargo package &&\
	cargo package --list

releaseAsciiLanguage:
	cd packages/morsels_languages/morsels_lang_ascii &&\
	cargo publish

# These 2 are separate as the prior needs to be published first
preReleaseOtherLanguages:
	cd packages/morsels_languages/morsels_lang_latin &&\
	cargo package &&\
	cargo package --list
	cd packages/morsels_languages/morsels_lang_chinese &&\
	cargo package &&\
	cargo package --list

releaseOtherLanguages:
	cd packages/morsels_languages/morsels_lang_latin &&\
	cargo publish
	cd packages/morsels_languages/morsels_lang_chinese &&\
	cargo publish

# Extremely small iteratively releases
releaseDependencies:
	make preReleaseCommon
	make releaseCommon
	timeout 20
	make releaseAsciiLanguage
	timeout 20
	make releaseOtherLanguages
	timeout 10

# git checkout -- . is to discard wasm-pack package.json changes
buildSearch:
	npm run setup
	npx lerna version $(VERSION) --amend --no-push --yes
	npm run buildSearch
	git add packages/search-ui/dist/*
	git commit --amend -m "Bump version"
	git checkout -- .
	git tag --force $(VERSION)

# Indexer relies on all of the above
preReleaseIndexer:
	cd packages/morsels_indexer &&\
	cargo package &&\
	cargo package --list

releaseIndexer:
	cd packages/morsels_indexer &&\
	cargo publish

preReleaseMdbook:
	cd packages/mdbook-morsels &&\
	cargo package &&\
	cargo package --list

releaseMdbook:
	cd packages/mdbook-morsels &&\
	cargo publish

finalise:
	git push
	git push morsels $(VERSION)
	git stash pop
	npm run updateDemo

# Extremely small iteratively releases
releaseAll:
	make releaseDependencies
	make buildSearch
	make releaseIndexer
	timeout 20
	make releaseMdbook
	make finalise

buildWinBinaries:
	cargo build --release --target x86_64-pc-windows-msvc -p morsels_indexer
	cargo build --release --target x86_64-pc-windows-msvc -p mdbook-morsels

buildLinuxBinaries:
	cargo build --release --target x86_64-unknown-linux-gnu -p morsels_indexer
	cargo build --release --target x86_64-unknown-linux-gnu -p mdbook-morsels

zipBinaries:
	zip -j target/search.morsels.zip packages/search-ui/dist/*
	zip -j target/indexer.x86_64-pc-windows-msvc.zip target/x86_64-pc-windows-msvc/release/morsels.exe target/x86_64-pc-windows-msvc/release/mdbook-morsels.exe
	zip -j target/indexer.x86_64-unknown-linux-gnu.zip target/x86_64-unknown-linux-gnu/release/morsels target/x86_64-unknown-linux-gnu/release/mdbook-morsels
