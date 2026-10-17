PORT ?= 8642
WASM := target/wasm32-unknown-unknown/release/rainbow_river.wasm

.PHONY: build serve test clean

build:
	cargo build --release --target wasm32-unknown-unknown -p rainbow-river
	@if command -v wasm-opt >/dev/null; then \
		wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int $(WASM) -o web/rainbow-river.wasm; \
	else cp $(WASM) web/rainbow-river.wasm; fi
	@ls -l web/rainbow-river.wasm | awk '{print "web/rainbow-river.wasm: " $$5 " bytes"}'

serve: build
	python3 -m http.server $(PORT) --directory web

test:
	cargo test

clean:
	cargo clean
	rm -f web/rainbow-river.wasm
