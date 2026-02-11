SHELL := /usr/bin/env bash

# Default goal
.DEFAULT_GOAL := help

.PHONY: help
# Parse the comment starting with a double ## next to a target as the target description
# in the help message
help: ## Show this help message
	@grep -E '^[a-zA-Z0-9_.-]+:.*?## ' $(MAKEFILE_LIST) | \
		sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-25s\033[0m %s\n", $$1, $$2}'


# wasm-pack build ffi --target web
# cp ffi/pkg/*.js ffi/pkg/*.d.ts ffi/pkg/*.wasm spa
# bun build spa/index.html --outdir spa/out --target browser
# cp ffi/pkg/*.wasm spa/out/

CRATES := todo-list ffi 
CRATE_MANIFESTS := $(addsuffix /Cargo.toml,$(CRATES))
WORKSPACE_CARGO_FILES := Cargo.toml Cargo.lock
RUST_RS_FILES := $(shell find $(CRATES) \
	\( -type d -name rust_modules -o -type d -name node_modules \) -prune \
	-o -type f -name '*.rs' -print 2>/dev/null | LC_ALL=C sort)
RUST_SOURCES := $(WORKSPACE_CARGO_FILES) $(CRATE_MANIFESTS) $(RUST_RS_FILES)

PKG_DIR := ffi/pkg/
COMPILED_WASM := ffi_bg.js ffi_bg.wasm ffi.js 
PKG_OUT := $(addprefix $(PKG_DIR),$(COMPILED_WASM))

$(PKG_OUT) &: $(RUST_SOURCES)
	wasm-pack build ffi --target web

SPA_SOURCES := spa/index.html spa/main.ts spa/style.css
SPA_OUT := spa/out/index.html

$(SPA_OUT) := $(PKG_OUT) $(SPA_SOURCES)
	rm -rf spa/out
	cp $(PKG_OUT) spa
	bun build spa/index.html --outdir spa/out --target browser 

spa: $(SPA_OUT) ## build the single-page app

.PHONY: serve-spa
serve-spa: $(SPA_OUT) ## serve the single-page app locally
	miniserve --spa --index index.html spa/out