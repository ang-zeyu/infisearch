# Primarily for publishing
# Use npm scripts for development

# Update the cargo.toml version numbers before running anything!

# Run in order
# Check preReleaseXX outputs manually before running release

preReleaseCommon:
	cd packages/morsels_common &&\
	cargo package &&\
	cargo package --list

releaseCommon:
	cd packages/morsels_common &&\
	cargo publish

preReleaseLanguages:
	cd packages/morsels_languages/morsels_lang_ascii &&\
	cargo package &&\
	cargo package --list
	cd packages/morsels_languages/morsels_lang_chinese &&\
	cargo package &&\
	cargo package --list

releaseLanguages:
	cd packages/morsels_languages/morsels_lang_ascii &&\
	cargo publish
	cd packages/morsels_languages/morsels_lang_chinese &&\
	cargo publish

# Latin (ascii with stemmers) is separate as the prior needs to be published first
preReleaseLatin:
	cd packages/morsels_languages/morsels_lang_latin &&\
	cargo package &&\
	cargo package --list

releaseLatin:
	cd packages/morsels_languages/morsels_lang_latin &&\
	cargo publish

# Indexer relies on all of the above
preReleaseIndexer:
	cd packages/morsels_indexer &&\
	cargo package &&\
	cargo package --list

releaseIndexer:
	cd packages/morsels_indexer &&\
	cargo publish

preReleaseSearch:
	npm run setup
	npm run buildSearch
	git add packages/search-ui/dist/*
	git commit -m "Update search-ui dist"
	npx lerna version --no-push

releaseSearch:
	npx lerna publish from-git

preReleaseMdbook:
	npx cpy packages/search-ui/dist packages/mdbook-morsels/search-ui-dist
	cd packages/mdbook-morsels &&\
	cargo package &&\
	cargo package --list

releaseMdbook:
	cd packages/mdbook-morsels &&\
	cargo publish
